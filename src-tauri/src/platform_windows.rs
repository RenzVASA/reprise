//! Reprise sous Windows.
//!
//! - Les apps ouvertes : PowerShell liste les programmes qui ont une fenêtre, avec leur
//!   chemin et leur ligne de commande (pour retrouver les fichiers qu'ils ont ouverts).
//! - Les dossiers : les fenêtres de l'Explorateur, demandées à Windows (Shell.Application).
//! - Les onglets : lus dans le fichier de session des navigateurs (voir `sessions`).
//! - La place des fenêtres : demandée directement à Windows (user32), puis remise en place.
//! - L'absence : verrouillage et inactivité demandés directement à Windows (user32).
//!
//! Tout est en lecture seule, sans droits administrateur.

use crate::desktop::{self, FoundApp, FoundFolder, Launcher};
use crate::model::{Frame, RestoreReport, Snapshot};
use crate::restore::Action;
use crate::script::{RunningApp, ScriptError};
use crate::sys;
use std::io::Read;
use std::path::Path;
use std::process::Stdio;
use std::time::{Duration, Instant};

const US: char = '\u{1f}';
const RS: char = '\u{1e}';

/// Le script PowerShell de capture. Champs séparés par 0x1F, lignes par 0x1E.
/// A = app : pid, nom, chemin du programme, titre de la fenêtre, ligne de commande.
/// F = dossier ouvert dans l'Explorateur : chemin, gauche, haut, largeur, hauteur.
pub const CAPTURE_PS: &str = r#"
$ErrorActionPreference = 'SilentlyContinue'
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
$US = [char]31; $RS = [char]30
$sb = New-Object System.Text.StringBuilder
$cmd = @{}
Get-CimInstance Win32_Process | ForEach-Object { $cmd[[int]$_.ProcessId] = $_.CommandLine }
Get-Process | Where-Object { $_.MainWindowHandle -ne 0 -and $_.Path } | ForEach-Object {
  $name = $_.Description
  if (-not $name) { $name = $_.ProcessName }
  [void]$sb.Append('A' + $US + $_.Id + $US + $name + $US + $_.Path + $US + $_.MainWindowTitle + $US + $cmd[[int]$_.Id] + $RS)
}
try {
  (New-Object -ComObject Shell.Application).Windows() | ForEach-Object {
    try {
      $p = $_.Document.Folder.Self.Path
      if ($p) { [void]$sb.Append('F' + $US + $p + $US + $_.Left + $US + $_.Top + $US + $_.Width + $US + $_.Height + $RS) }
    } catch {}
  }
} catch {}
[Console]::Out.Write($sb.ToString())
"#;

/// Programmes qui ont une fenêtre mais ne sont pas « tes apps » : morceaux de Windows,
/// et l'Explorateur (ses fenêtres sont enregistrées comme dossiers).
const SYSTEM_EXES: &[&str] = &[
    "explorer.exe",
    "applicationframehost.exe",
    "textinputhost.exe",
    "searchhost.exe",
    "searchapp.exe",
    "startmenuexperiencehost.exe",
    "shellexperiencehost.exe",
    "lockapp.exe",
    "systemsettingsbroker.exe",
    "nvidia overlay.exe",
    "gamebar.exe",
    "widgets.exe",
    "msedgewebview2.exe",
];

#[derive(Debug, Clone, PartialEq)]
pub struct RawProc {
    pub pid: u32,
    pub name: String,
    pub path: String,
    pub title: String,
    pub command_line: String,
}

pub fn parse_capture(out: &str) -> (Vec<RawProc>, Vec<FoundFolder>) {
    let mut procs = Vec::new();
    let mut folders = Vec::new();
    for rec in out.split(RS) {
        // PowerShell peut glisser une marque d'encodage (BOM) au tout début.
        let f: Vec<&str> = rec.trim_matches(['\r', '\n', '\u{feff}']).split(US).collect();
        match f.first().copied() {
            Some("A") if f.len() >= 5 => procs.push(RawProc {
                pid: f[1].trim().parse().unwrap_or(0),
                name: f[2].trim().to_string(),
                path: f[3].trim().to_string(),
                title: f[4].trim().to_string(),
                command_line: f.get(5).map(|s| s.trim().to_string()).unwrap_or_default(),
            }),
            Some("F") if f.len() >= 2 => {
                let p = f[1].trim();
                // Les dossiers spéciaux (« Ce PC »…) ont des chemins en « ::{…} ».
                if !p.starts_with("::") && Path::new(p).is_dir() && !folders.iter().any(|x: &FoundFolder| x.path.eq_ignore_ascii_case(p)) {
                    let n = |i: usize| f.get(i).and_then(|v| v.trim().parse::<i32>().ok());
                    let frame = match (n(2), n(3), n(4), n(5)) {
                        (Some(x), Some(y), Some(w), Some(h)) => Some(Frame { title: String::new(), x, y, w, h }).filter(|f| f.is_usable()),
                        _ => None,
                    };
                    folders.push(FoundFolder { path: p.to_string(), frame });
                }
            }
            _ => {}
        }
    }
    (procs, folders)
}

/// Fichiers passés à l'app au lancement (« WINWORD.EXE /n C:\…\Exposé.docx »).
pub fn files_in_command_line(command_line: &str, home: &str) -> Vec<String> {
    sys::split_windows_args(command_line)
        .into_iter()
        .skip(1)
        .filter(|a| windows_user_path(a, home))
        .collect()
}

/// Un chemin de l'utilisateur sous Windows : dans son dossier, ou sur un autre disque,
/// mais pas dans Windows, Program Files ni AppData.
pub fn windows_user_path(p: &str, home: &str) -> bool {
    let l = p.to_ascii_lowercase().replace('/', "\\");
    if l.len() < 4 || l.as_bytes().get(1) != Some(&b':') || l.as_bytes().get(2) != Some(&b'\\') {
        return false;
    }
    let banned = ["\\windows\\", "\\program files", "\\programdata\\", "\\appdata\\", "\\$recycle.bin\\"];
    if banned.iter().any(|b| l.contains(b)) {
        return false;
    }
    let home = home.to_ascii_lowercase().replace('/', "\\");
    (l.starts_with(&home) || !l.starts_with("c:\\")) && l.trim_end_matches('\\') != home.trim_end_matches('\\') && Path::new(p).exists()
}

/// Lance PowerShell avec un script, en lui laissant au plus `secs` secondes.
fn powershell(script: &str, secs: u64) -> Result<String, ScriptError> {
    let utf16: Vec<u8> = script.encode_utf16().flat_map(|u| u.to_le_bytes()).collect();
    let encoded = sys::base64(&utf16);
    let mut child = sys::command("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-EncodedCommand", &encoded])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| ScriptError::Other(format!("PowerShell introuvable : {e}")))?;
    let mut pipe = child.stdout.take();
    let reader = std::thread::spawn(move || {
        let mut buf = Vec::new();
        if let Some(p) = pipe.as_mut() {
            let _ = p.read_to_end(&mut buf);
        }
        buf
    });
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if start.elapsed() > Duration::from_secs(secs) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(ScriptError::Timeout);
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(30)),
            Err(e) => return Err(ScriptError::Other(e.to_string())),
        }
    }
    let out = reader.join().unwrap_or_default();
    Ok(String::from_utf8_lossy(&out).into_owned())
}

fn own_exe() -> String {
    std::env::current_exe().map(|p| p.to_string_lossy().to_ascii_lowercase()).unwrap_or_default()
}

/// Transforme la liste brute en apps : une par programme, sans les morceaux de Windows.
pub fn to_found_apps(procs: &[RawProc], own: &str, front_pid: Option<u32>, home: &str) -> Vec<FoundApp> {
    let mut apps: Vec<FoundApp> = Vec::new();
    for p in procs {
        let base = sys::file_name(&p.path).to_ascii_lowercase();
        if SYSTEM_EXES.contains(&base.as_str()) || p.path.eq_ignore_ascii_case(own) {
            continue;
        }
        let files = files_in_command_line(&p.command_line, home);
        let front = front_pid == Some(p.pid);
        match apps.iter_mut().find(|a| a.id.eq_ignore_ascii_case(&p.path)) {
            Some(a) => {
                a.frontmost |= front;
                for f in files {
                    if !a.files.iter().any(|x| x.eq_ignore_ascii_case(&f)) {
                        a.files.push(f);
                    }
                }
            }
            None => apps.push(FoundApp {
                name: if p.name.is_empty() { base.trim_end_matches(".exe").to_string() } else { p.name.clone() },
                id: p.path.clone(),
                frontmost: front,
                files,
            }),
        }
    }
    apps
}

type Captured = (Vec<FoundApp>, Vec<(String, Frame)>, Vec<FoundFolder>);

fn capture_raw() -> Result<Captured, ScriptError> {
    let out = powershell(CAPTURE_PS, 25)?;
    let (procs, folders) = parse_capture(&out);
    let home = std::env::var("USERPROFILE").unwrap_or_default();
    let apps = to_found_apps(&procs, &own_exe(), win::foreground_pid(), &home);
    Ok((apps, window_frames(), folders))
}

/// La place de chaque fenêtre ouverte, par programme. Les navigateurs rouvrent leurs propres
/// fenêtres et l'Explorateur a ses dossiers : on ne les compte pas ici.
fn window_frames() -> Vec<(String, Frame)> {
    let me = std::process::id();
    win::top_windows()
        .into_iter()
        .filter(|w| w.pid != me && !w.minimized && !w.title.is_empty())
        .filter_map(|w| {
            let exe = w.exe.clone()?;
            let base = sys::file_name(&exe).to_ascii_lowercase();
            if base == "explorer.exe" || crate::sessions::browser_for(&exe).is_some() || SYSTEM_EXES.contains(&base.as_str()) {
                return None;
            }
            let (l, t, r, b) = w.rect;
            let frame = Frame { title: w.title.clone(), x: l, y: t, w: r - l, h: b - t };
            frame.is_usable().then_some((exe, frame))
        })
        .collect()
}

/// Choisit, pour chaque cadre sauvegardé, la fenêtre à déplacer : même titre d'abord ;
/// sinon, s'il n'y a qu'une fenêtre et qu'un cadre, c'est elle.
pub fn match_windows(frames: &[Frame], windows: &[(isize, String)]) -> Vec<(isize, Frame)> {
    let mut used: Vec<isize> = Vec::new();
    let mut out = Vec::new();
    for f in frames {
        if let Some((h, _)) = windows.iter().find(|(h, t)| !used.contains(h) && !f.title.is_empty() && t == &f.title) {
            used.push(*h);
            out.push((*h, f.clone()));
        }
    }
    if out.is_empty() && frames.len() == 1 && windows.len() == 1 {
        out.push((windows[0].0, frames[0].clone()));
    }
    out
}

// ─────────────────────────────── Capture ─────────────────────────────────────

pub fn running_apps() -> Result<Vec<RunningApp>, ScriptError> {
    // La liste complète arrive avec la capture : ici on donne juste l'app au premier plan,
    // pour que la fenêtre de sauvegarde s'ouvre tout de suite.
    Ok(Vec::new())
}

pub fn full_capture(_apps: &[RunningApp], _own: &str, ignored: &[String]) -> Snapshot {
    match capture_raw() {
        Ok((apps, frames, folders)) => desktop::build_snapshot(&apps, &frames, &folders, ignored, Vec::new()),
        Err(e) => desktop::build_snapshot(
            &[],
            &[],
            &[],
            ignored,
            vec![match e {
                ScriptError::Timeout => "Windows a mis trop de temps à répondre. Réessaie dans un instant.".into(),
                ScriptError::Other(m) => format!("La capture a échoué : {m}"),
                _ => "La capture a échoué.".into(),
            }],
        ),
    }
}

pub fn capture_all(_own: &str, ignored: &[String]) -> Result<Snapshot, ScriptError> {
    let (apps, frames, folders) = capture_raw()?;
    Ok(desktop::build_snapshot(&apps, &frames, &folders, ignored, Vec::new()))
}

// ─────────────────────────────── Reprise ─────────────────────────────────────

pub struct Windows;

impl Launcher for Windows {
    fn launch(&self, id: &str, args: &[String]) -> Result<(), String> {
        if !Path::new(id).exists() {
            return Err(format!("Programme introuvable : {id}"));
        }
        let a: Vec<&str> = args.iter().map(String::as_str).collect();
        sys::spawn(id, &a)
    }

    fn open(&self, target: &str) -> Result<(), String> {
        // L'Explorateur ouvre dossiers, fichiers et adresses avec l'app par défaut.
        sys::spawn("explorer.exe", &[target])
    }

    /// « Faire place nette » : réduit toutes les fenêtres sauf celles de Reprise.
    fn hide_others(&self) {
        let me = std::process::id();
        for w in win::top_windows() {
            if w.pid != me && !w.minimized && !w.title.is_empty() {
                win::minimize(w.hwnd);
            }
        }
    }

    fn place(&self, id: &str, frames: &[Frame]) -> usize {
        let windows: Vec<(isize, String)> = win::top_windows()
            .into_iter()
            .filter(|w| w.exe.as_deref().is_some_and(|e| e.eq_ignore_ascii_case(id) || sys::file_name(e).eq_ignore_ascii_case(id)))
            .map(|w| (w.hwnd, w.title))
            .collect();
        match_windows(frames, &windows)
            .into_iter()
            .filter(|(h, f)| win::move_window(*h, f.x, f.y, f.w, f.h))
            .count()
    }

    fn file_manager_id(&self) -> &str {
        "explorer.exe"
    }
}

pub fn execute(actions: &[Action], _own: &str) -> RestoreReport {
    desktop::execute(&Windows, actions, &desktop::Timing::default())
}

pub fn open_url(url: &str) -> Result<(), String> {
    sys::spawn("rundll32.exe", &["url.dll,FileProtocolHandler", url])
}

/// Ouvre l'Explorateur avec le fichier sélectionné.
pub fn reveal(path: &Path) -> Result<(), String> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let mut c = sys::command("explorer.exe");
        c.raw_arg(format!("/select,\"{}\"", path.to_string_lossy()));
        return c.spawn().map(|_| ()).map_err(|e| e.to_string());
    }
    #[cfg(not(windows))]
    {
        let _ = path;
        Err("Windows seulement".into())
    }
}

// ─────────────────────────── Filet de sécurité ───────────────────────────────

/// Titres de fenêtre qui veulent dire « je regarde une vidéo ou un stream ».
const VIDEO_HINTS: &[&str] = &[
    "youtube", "twitch", "netflix", "prime video", "disney+", "crunchyroll", "vlc", "plex", "kick",
    "films et tv", "films & tv", "lecteur multimédia", "media player", "mpv", "molotov", "canal+", "france.tv",
];

pub fn is_video_title(title: &str) -> bool {
    let t = title.to_lowercase();
    VIDEO_HINTS.iter().any(|h| t.contains(h))
}

pub fn screen_locked() -> Option<bool> {
    win::locked()
}

pub fn idle_seconds() -> Option<u64> {
    win::idle_ms().map(|ms| ms / 1000)
}

/// « Tu regardes quelque chose » : on croise trois indices, sans droits administrateur.
/// - une app demande à Windows de garder l'écran allumé (ce que font les navigateurs et
///   les lecteurs pendant une vidéo) ;
/// - une app est en plein écran (vidéo, jeu, présentation) ;
/// - le titre de la fenêtre au premier plan parle d'une vidéo ou d'un stream.
pub fn display_kept_awake() -> Option<bool> {
    let awake = win::display_required().unwrap_or(false)
        || win::fullscreen_busy().unwrap_or(false)
        || win::foreground_title().is_some_and(|t| is_video_title(&t));
    Some(awake)
}

/// Appels directs à Windows (user32/kernel32), sans bibliothèque en plus.
#[cfg(windows)]
mod win {
    #[repr(C)]
    struct LastInputInfo {
        cb_size: u32,
        dw_time: u32,
    }

    #[link(name = "user32")]
    extern "system" {
        fn GetLastInputInfo(plii: *mut LastInputInfo) -> i32;
        fn GetForegroundWindow() -> isize;
        fn GetWindowThreadProcessId(hwnd: isize, pid: *mut u32) -> u32;
        fn GetWindowTextW(hwnd: isize, text: *mut u16, max: i32) -> i32;
        fn OpenInputDesktop(flags: u32, inherit: i32, access: u32) -> isize;
        fn SwitchDesktop(desktop: isize) -> i32;
        fn CloseDesktop(desktop: isize) -> i32;
    }
    #[link(name = "user32")]
    extern "system" {
        fn EnumWindows(cb: extern "system" fn(isize, isize) -> i32, lparam: isize) -> i32;
        fn IsWindowVisible(hwnd: isize) -> i32;
        fn IsIconic(hwnd: isize) -> i32;
        fn GetWindowRect(hwnd: isize, rect: *mut Rect) -> i32;
        fn GetWindow(hwnd: isize, cmd: u32) -> isize;
        fn GetWindowLongW(hwnd: isize, index: i32) -> i32;
        fn GetClassNameW(hwnd: isize, name: *mut u16, max: i32) -> i32;
        fn ShowWindow(hwnd: isize, cmd: i32) -> i32;
        fn SetWindowPos(hwnd: isize, after: isize, x: i32, y: i32, cx: i32, cy: i32, flags: u32) -> i32;
    }
    #[link(name = "kernel32")]
    extern "system" {
        fn GetTickCount() -> u32;
        fn OpenProcess(access: u32, inherit: i32, pid: u32) -> isize;
        fn QueryFullProcessImageNameW(process: isize, flags: u32, name: *mut u16, size: *mut u32) -> i32;
        fn CloseHandle(handle: isize) -> i32;
    }
    #[link(name = "dwmapi")]
    extern "system" {
        fn DwmGetWindowAttribute(hwnd: isize, attr: u32, value: *mut u32, size: u32) -> i32;
    }
    #[link(name = "shell32")]
    extern "system" {
        fn SHQueryUserNotificationState(state: *mut i32) -> i32;
    }
    #[link(name = "powrprof")]
    extern "system" {
        fn CallNtPowerInformation(level: i32, input: *mut u8, in_len: u32, output: *mut u32, out_len: u32) -> i32;
    }

    #[repr(C)]
    #[derive(Default)]
    pub struct Rect {
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
    }

    pub struct Win {
        pub hwnd: isize,
        pub pid: u32,
        pub title: String,
        pub exe: Option<String>,
        pub rect: (i32, i32, i32, i32),
        pub minimized: bool,
    }

    extern "system" fn collect(hwnd: isize, lparam: isize) -> i32 {
        // SAFETY : `lparam` est le pointeur vers le Vec passé par `top_windows`, vivant pendant l'appel.
        let list = unsafe { &mut *(lparam as *mut Vec<isize>) };
        list.push(hwnd);
        1
    }

    fn exe_of(pid: u32) -> Option<String> {
        const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;
        // SAFETY : handle fermé avant de sortir, tampon de taille connue.
        unsafe {
            let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
            if h == 0 {
                return None;
            }
            let mut buf = [0u16; 1024];
            let mut len = buf.len() as u32;
            let ok = QueryFullProcessImageNameW(h, 0, buf.as_mut_ptr(), &mut len) != 0;
            CloseHandle(h);
            ok.then(|| String::from_utf16_lossy(&buf[..len as usize]))
        }
    }

    fn text_of(hwnd: isize) -> String {
        let mut buf = [0u16; 512];
        // SAFETY : tampon de 512 caractères, taille passée à Windows.
        let n = unsafe { GetWindowTextW(hwnd, buf.as_mut_ptr(), buf.len() as i32) };
        if n > 0 {
            String::from_utf16_lossy(&buf[..n as usize])
        } else {
            String::new()
        }
    }

    fn class_of(hwnd: isize) -> String {
        let mut buf = [0u16; 256];
        // SAFETY : idem.
        let n = unsafe { GetClassNameW(hwnd, buf.as_mut_ptr(), buf.len() as i32) };
        if n > 0 {
            String::from_utf16_lossy(&buf[..n as usize])
        } else {
            String::new()
        }
    }

    /// Les vraies fenêtres d'apps : visibles, sans propriétaire, pas des palettes d'outils,
    /// pas cachées sur un autre bureau, pas le bureau ni la barre des tâches.
    pub fn top_windows() -> Vec<Win> {
        const GW_OWNER: u32 = 4;
        const GWL_EXSTYLE: i32 = -20;
        const WS_EX_TOOLWINDOW: i32 = 0x80;
        const DWMWA_CLOAKED: u32 = 14;
        let mut handles: Vec<isize> = Vec::new();
        // SAFETY : le rappel ne fait qu'ajouter au Vec, qui vit jusqu'à la fin de l'appel.
        unsafe { EnumWindows(collect, &mut handles as *mut Vec<isize> as isize) };
        let mut exes: std::collections::HashMap<u32, Option<String>> = std::collections::HashMap::new();
        let mut out = Vec::new();
        for hwnd in handles {
            // SAFETY : appels de lecture sur des handles de fenêtres qui viennent d'être listés.
            unsafe {
                if IsWindowVisible(hwnd) == 0 || GetWindow(hwnd, GW_OWNER) != 0 || GetWindowLongW(hwnd, GWL_EXSTYLE) & WS_EX_TOOLWINDOW != 0 {
                    continue;
                }
                let mut cloaked = 0u32;
                if DwmGetWindowAttribute(hwnd, DWMWA_CLOAKED, &mut cloaked, 4) == 0 && cloaked != 0 {
                    continue;
                }
            }
            let class = class_of(hwnd);
            if ["Progman", "WorkerW", "Shell_TrayWnd", "Shell_SecondaryTrayWnd"].contains(&class.as_str()) {
                continue;
            }
            let mut pid = 0u32;
            let mut r = Rect::default();
            // SAFETY : lecture seule.
            let (minimized, has_rect) = unsafe {
                GetWindowThreadProcessId(hwnd, &mut pid);
                (IsIconic(hwnd) != 0, GetWindowRect(hwnd, &mut r) != 0)
            };
            if !has_rect {
                continue;
            }
            let exe = exes.entry(pid).or_insert_with(|| exe_of(pid)).clone();
            out.push(Win { hwnd, pid, title: text_of(hwnd), exe, rect: (r.left, r.top, r.right, r.bottom), minimized });
        }
        out
    }

    pub fn minimize(hwnd: isize) {
        const SW_MINIMIZE: i32 = 6;
        // SAFETY : simple ordre envoyé à la fenêtre.
        unsafe { ShowWindow(hwnd, SW_MINIMIZE) };
    }

    /// Déplace et redimensionne une fenêtre (en la sortant d'abord de l'état réduit ou agrandi).
    pub fn move_window(hwnd: isize, x: i32, y: i32, w: i32, h: i32) -> bool {
        const SW_RESTORE: i32 = 9;
        const SWP_NOZORDER: u32 = 0x0004;
        const SWP_NOACTIVATE: u32 = 0x0010;
        // SAFETY : ordres envoyés à une fenêtre existante.
        unsafe {
            if IsIconic(hwnd) != 0 {
                ShowWindow(hwnd, SW_RESTORE);
            }
            SetWindowPos(hwnd, 0, x, y, w, h, SWP_NOZORDER | SWP_NOACTIVATE) != 0
        }
    }

    /// Une app a demandé à Windows de garder l'écran allumé (vidéo en cours…).
    pub fn display_required() -> Option<bool> {
        const SYSTEM_EXECUTION_STATE: i32 = 16;
        const ES_DISPLAY_REQUIRED: u32 = 0x2;
        let mut state = 0u32;
        // SAFETY : sortie de 4 octets, pas d'entrée (comme l'exige la documentation).
        let st = unsafe { CallNtPowerInformation(SYSTEM_EXECUTION_STATE, std::ptr::null_mut(), 0, &mut state, 4) };
        (st == 0).then_some(state & ES_DISPLAY_REQUIRED != 0)
    }

    /// Une app est en plein écran (vidéo, jeu, présentation) : Windows se met en « occupé ».
    pub fn fullscreen_busy() -> Option<bool> {
        let mut state = 0i32;
        // SAFETY : sortie d'un entier.
        let hr = unsafe { SHQueryUserNotificationState(&mut state) };
        // 2 = occupé (plein écran), 3 = jeu plein écran, 4 = présentation.
        (hr == 0).then_some(matches!(state, 2..=4))
    }

    pub fn idle_ms() -> Option<u64> {
        let mut info = LastInputInfo { cb_size: std::mem::size_of::<LastInputInfo>() as u32, dw_time: 0 };
        // SAFETY : structure de la bonne taille, appel sans effet de bord.
        let ok = unsafe { GetLastInputInfo(&mut info) } != 0;
        ok.then(|| u64::from(unsafe { GetTickCount() }.wrapping_sub(info.dw_time)))
    }

    /// Écran verrouillé : le bureau de saisie n'est plus le nôtre.
    pub fn locked() -> Option<bool> {
        const DESKTOP_SWITCHDESKTOP: u32 = 0x0100;
        // SAFETY : handle fermé juste après.
        unsafe {
            let h = OpenInputDesktop(0, 0, DESKTOP_SWITCHDESKTOP);
            if h == 0 {
                return Some(true);
            }
            let ours = SwitchDesktop(h) != 0;
            CloseDesktop(h);
            Some(!ours)
        }
    }

    pub fn foreground_pid() -> Option<u32> {
        // SAFETY : lecture seule.
        unsafe {
            let w = GetForegroundWindow();
            if w == 0 {
                return None;
            }
            let mut pid = 0u32;
            GetWindowThreadProcessId(w, &mut pid);
            (pid != 0).then_some(pid)
        }
    }

    pub fn foreground_title() -> Option<String> {
        // SAFETY : tampon de 512 caractères, taille passée à Windows.
        unsafe {
            let w = GetForegroundWindow();
            if w == 0 {
                return None;
            }
            let mut buf = [0u16; 512];
            let n = GetWindowTextW(w, buf.as_mut_ptr(), buf.len() as i32);
            (n > 0).then(|| String::from_utf16_lossy(&buf[..n as usize]))
        }
    }
}

/// Hors Windows (tests) : rien.
#[cfg(not(windows))]
mod win {
    pub struct Win {
        pub hwnd: isize,
        pub pid: u32,
        pub title: String,
        pub exe: Option<String>,
        pub rect: (i32, i32, i32, i32),
        pub minimized: bool,
    }
    pub fn top_windows() -> Vec<Win> {
        Vec::new()
    }
    pub fn minimize(_hwnd: isize) {}
    pub fn move_window(_hwnd: isize, _x: i32, _y: i32, _w: i32, _h: i32) -> bool {
        false
    }
    pub fn display_required() -> Option<bool> {
        None
    }
    pub fn fullscreen_busy() -> Option<bool> {
        None
    }
    pub fn idle_ms() -> Option<u64> {
        None
    }
    pub fn locked() -> Option<bool> {
        None
    }
    pub fn foreground_pid() -> Option<u32> {
        None
    }
    pub fn foreground_title() -> Option<String> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_powershell_output() {
        let tmp = std::env::temp_dir().to_string_lossy().into_owned();
        let out = format!(
            "\u{feff}A{US}101{US}Google Chrome{US}C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe{US}YouTube - Google Chrome{US}\"C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe\" {RS}\
             A{US}202{US}{US}C:\\Windows\\explorer.exe{US}Documents{US}C:\\Windows\\Explorer.EXE{RS}\
             A{US}303{US}Bloc-notes{US}C:\\Windows\\System32\\notepad.exe{US}note.txt - Bloc-notes{US}notepad.exe{RS}\
             F{US}::{{20D04FE0-3AEA-1069-A2D8-08002B30309D}}{RS}\
             F{US}{tmp}{US}100{US}50{US}900{US}600{RS}\r\n"
        );
        let (procs, folders) = parse_capture(&out);
        assert_eq!(procs.len(), 3);
        assert_eq!(procs[0].pid, 101);
        assert_eq!(folders, vec![FoundFolder { path: tmp, frame: Some(Frame { title: String::new(), x: 100, y: 50, w: 900, h: 600 }) }]);
        let apps = to_found_apps(&procs, "c:\\reprise\\reprise.exe", Some(303), "C:\\Users\\moi");
        let names: Vec<_> = apps.iter().map(|a| (a.name.as_str(), a.frontmost)).collect();
        assert_eq!(names, vec![("Google Chrome", false), ("Bloc-notes", true)]);
    }

    #[test]
    fn picks_windows_to_move() {
        let f = |t: &str| Frame { title: t.into(), x: 1, y: 2, w: 300, h: 200 };
        let wins = vec![(11, "Rapport.docx - Word".to_string()), (22, "Notes.docx - Word".to_string())];
        let m = match_windows(&[f("Notes.docx - Word"), f("Absent - Word")], &wins);
        assert_eq!(m.iter().map(|(h, _)| *h).collect::<Vec<_>>(), vec![22]);
        // Titre changé, mais une seule fenêtre et un seul cadre : c'est elle.
        let m = match_windows(&[f("Ancien titre")], &[(33, "Nouveau titre".to_string())]);
        assert_eq!(m.len(), 1);
        assert!(match_windows(&[f("x")], &[(1, "a".into()), (2, "b".into())]).is_empty());
    }

    #[test]
    fn user_paths_only() {
        assert!(!windows_user_path(r"C:\Windows\System32\notepad.exe", r"C:\Users\moi"));
        assert!(!windows_user_path(r"C:\Program Files\App\x.dll", r"C:\Users\moi"));
        assert!(!windows_user_path(r"C:\Users\moi\AppData\Local\x.json", r"C:\Users\moi"));
        assert!(!windows_user_path("/n", r"C:\Users\moi"));
        assert!(!windows_user_path(r"C:\Users\moi", r"C:\Users\moi"));
        assert!(is_video_title("Le meilleur de 2026 - YouTube — Opera"));
        assert!(!is_video_title("Exposé.docx - Word"));
    }

    #[test]
    fn script_is_encodable() {
        let utf16: Vec<u8> = CAPTURE_PS.encode_utf16().flat_map(|u| u.to_le_bytes()).collect();
        let enc = sys::base64(&utf16);
        assert!(enc.len() < 30_000, "la ligne de commande Windows est limitée à 32 767 caractères");
    }
}
