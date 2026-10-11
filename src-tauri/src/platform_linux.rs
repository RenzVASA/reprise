//! Reprise sous Linux.
//!
//! - Les apps ouvertes : on parcourt les programmes qui tournent (/proc) et on les relie
//!   à leur raccourci d'app (fichier .desktop), grâce au « scope » que GNOME et KDE donnent
//!   à chaque app lancée, ou au nom du programme.
//! - Les onglets : lus dans le fichier de session des navigateurs (voir `sessions`).
//! - Les fichiers : ceux que l'app a reçus au lancement (« evince ~/Cours/chapitre3.pdf »).
//! - L'absence : verrouillage par systemd-logind, inactivité par GNOME (ou xprintidle).
//!
//! Tout est en lecture seule, sans droits particuliers.

use crate::desktop::{self, FoundApp, Launcher};
use crate::model::{RestoreReport, Snapshot};
use crate::restore::Action;
use crate::script::{RunningApp, ScriptError};
use crate::sys;
use std::collections::HashMap;
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

// ───────────────────────────── Raccourcis d'apps ─────────────────────────────

#[derive(Debug, Clone)]
pub struct Entry {
    /// « firefox.desktop »
    pub id: String,
    pub name: String,
    /// La commande, sans les codes %u, %F…
    pub exec: Vec<String>,
    /// Position où insérer les fichiers à ouvrir (le code %u/%f), sinon à la fin.
    pub file_slot: Option<usize>,
    /// Nom du programme lancé (« firefox », « google-chrome-stable »…).
    pub exec_base: String,
}

fn data_dirs() -> Vec<PathBuf> {
    let home = std::env::var("HOME").unwrap_or_default();
    let mut dirs = vec![std::env::var("XDG_DATA_HOME").map(PathBuf::from).unwrap_or_else(|_| Path::new(&home).join(".local/share"))];
    let sys_dirs = std::env::var("XDG_DATA_DIRS").unwrap_or_else(|_| "/usr/local/share:/usr/share".into());
    dirs.extend(sys_dirs.split(':').filter(|d| !d.is_empty()).map(PathBuf::from));
    dirs.push(Path::new(&home).join(".local/share/flatpak/exports/share"));
    dirs.push("/var/lib/flatpak/exports/share".into());
    dirs.push("/var/lib/snapd/desktop".into());
    dirs.into_iter().map(|d| d.join("applications")).collect()
}

/// Découpe une ligne Exec= (guillemets doubles, barres obliques inversées).
pub fn split_exec(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quoted = false;
    let mut has = false;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' => {
                quoted = !quoted;
                has = true;
            }
            '\\' if quoted => {
                if let Some(n) = chars.next() {
                    cur.push(n);
                }
            }
            ' ' | '\t' if !quoted => {
                if has {
                    out.push(std::mem::take(&mut cur));
                    has = false;
                }
            }
            _ => {
                cur.push(c);
                has = true;
            }
        }
    }
    if has {
        out.push(cur);
    }
    out
}

fn base_name(p: &str) -> String {
    p.rsplit('/').next().unwrap_or(p).to_ascii_lowercase()
}

/// Lit un fichier .desktop. `None` s'il ne correspond pas à une app visible.
pub fn parse_desktop(id: &str, text: &str, lang: &str) -> Option<Entry> {
    let mut in_main = false;
    let mut kv: HashMap<String, String> = HashMap::new();
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_main = line == "[Desktop Entry]";
            continue;
        }
        if !in_main || line.starts_with('#') {
            continue;
        }
        if let Some((k, v)) = line.split_once('=') {
            kv.entry(k.trim().to_string()).or_insert_with(|| v.trim().to_string());
        }
    }
    let yes = |k: &str| kv.get(k).is_some_and(|v| v.eq_ignore_ascii_case("true"));
    if kv.get("Type").map(String::as_str) != Some("Application") || yes("NoDisplay") || yes("Hidden") || yes("Terminal") {
        return None;
    }
    let short = lang.split(['_', '.']).next().unwrap_or("");
    let name = kv
        .get(&format!("Name[{lang}]"))
        .or_else(|| kv.get(&format!("Name[{short}]")))
        .or_else(|| kv.get("Name"))?
        .clone();
    let raw = split_exec(kv.get("Exec")?);
    let mut exec = Vec::new();
    let mut file_slot = None;
    for t in raw {
        match t.as_str() {
            "%f" | "%F" | "%u" | "%U" => file_slot = Some(exec.len()),
            "%i" | "%c" | "%k" | "%d" | "%D" | "%n" | "%N" | "%v" | "%m" => {}
            _ => exec.push(t.replace("%%", "%")),
        }
    }
    // Le vrai programme : on saute « env VAR=… » et on comprend « flatpak run ».
    let mut i = 0;
    if exec.first().map(|s| base_name(s)) == Some("env".into()) {
        i = 1;
        while exec.get(i).is_some_and(|s| s.contains('=') && !s.starts_with('/')) {
            i += 1;
        }
    }
    let exec_base = match exec.get(i).map(|s| base_name(s)) {
        Some(b) if b == "flatpak" => exec.iter().skip(i + 1).filter(|s| !s.starts_with('-') && *s != "run").next().map(|s| s.to_ascii_lowercase()).unwrap_or(b),
        Some(b) => b,
        None => return None,
    };
    Some(Entry { id: id.to_string(), name, exec, file_slot, exec_base })
}

pub fn desktop_index() -> Vec<Entry> {
    let lang = std::env::var("LANG").unwrap_or_default();
    let lang = lang.split('.').next().unwrap_or("").to_string();
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for dir in data_dirs() {
        let Ok(files) = fs::read_dir(&dir) else { continue };
        for f in files.flatten() {
            let id = f.file_name().to_string_lossy().into_owned();
            if !id.ends_with(".desktop") || !seen.insert(id.clone()) {
                continue;
            }
            if let Ok(text) = fs::read_to_string(f.path()) {
                if let Some(e) = parse_desktop(&id, &text, &lang) {
                    out.push(e);
                }
            }
        }
    }
    out
}

// ─────────────────────────────── Programmes ──────────────────────────────────

/// Le « scope » systemd d'une app lancée depuis le bureau dit quel raccourci l'a lancée :
/// « app-gnome-org.gnome.TextEditor-4242.scope », « app-flatpak-org.mozilla.firefox-12.scope »…
pub fn cgroup_app_ids(cgroup: &str) -> Vec<String> {
    let Some(seg) = cgroup.lines().filter_map(|l| l.rsplit('/').next()).find(|s| s.starts_with("app-")) else {
        return Vec::new();
    };
    let mut s = seg.trim_start_matches("app-").to_string();
    if let Some(at) = s.find('@') {
        s.truncate(at);
    }
    for suffix in [".scope", ".service", ".slice"] {
        if let Some(stripped) = s.strip_suffix(suffix) {
            s = stripped.to_string();
        }
    }
    // Retire le numéro ou l'identifiant aléatoire final.
    if let Some(dash) = s.rfind('-') {
        let tail = &s[dash + 1..];
        if !tail.is_empty() && tail.chars().all(|c| c.is_ascii_hexdigit()) {
            s.truncate(dash);
        }
    }
    let s = unescape(&s);
    let mut ids = vec![s.clone()];
    for prefix in ["gnome-", "flatpak-", "kde-", "snap-", "dbus-:1.2-", "dbus-"] {
        if let Some(rest) = s.strip_prefix(prefix) {
            ids.push(rest.to_string());
        }
    }
    ids
}

fn unescape(s: &str) -> String {
    let mut out = String::new();
    let b = s.as_bytes();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'\\' && b.get(i + 1) == Some(&b'x') {
            if let Some(v) = s.get(i + 2..i + 4).and_then(|h| u8::from_str_radix(h, 16).ok()) {
                out.push(v as char);
                i += 4;
                continue;
            }
        }
        out.push(b[i] as char);
        i += 1;
    }
    out
}

struct Proc {
    pid: u32,
    argv: Vec<String>,
    exe_base: String,
    cgroup: String,
}

fn processes() -> Vec<Proc> {
    let me = std::process::id();
    let uid = fs::metadata("/proc/self").map(|m| m.uid()).unwrap_or(u32::MAX);
    let mut out = Vec::new();
    let Ok(dir) = fs::read_dir("/proc") else { return out };
    for e in dir.flatten() {
        let Some(pid) = e.file_name().to_str().and_then(|s| s.parse::<u32>().ok()) else { continue };
        if pid == me || e.metadata().map(|m| m.uid()).ok() != Some(uid) {
            continue;
        }
        let p = e.path();
        let argv: Vec<String> = fs::read(p.join("cmdline"))
            .map(|b| b.split(|c| *c == 0).filter(|s| !s.is_empty()).map(|s| String::from_utf8_lossy(s).into_owned()).collect())
            .unwrap_or_default();
        if argv.is_empty() || argv.iter().any(|a| a.starts_with("--type=") || a == "--gapplication-service") {
            continue; // processus internes (onglets de Chrome…) ou service d'arrière-plan
        }
        let exe_base = fs::read_link(p.join("exe")).map(|l| base_name(&l.to_string_lossy())).unwrap_or_default();
        let cgroup = fs::read_to_string(p.join("cgroup")).unwrap_or_default();
        out.push(Proc { pid, argv, exe_base, cgroup });
    }
    out
}

/// Programmes qui ne portent pas le même nom que leur raccourci.
const ALIASES: &[(&str, &str)] = &[
    ("chrome", "google-chrome"),
    ("msedge", "microsoft-edge"),
    ("firefox-bin", "firefox"),
    ("brave", "brave-browser"),
    ("soffice.bin", "libreoffice"),
    ("oosplash", "libreoffice"),
];

fn file_args(argv: &[String], home: &str) -> Vec<String> {
    argv.iter()
        .skip(1)
        .map(|a| match a.strip_prefix("file://") {
            Some(p) => crate::script::percent_decode(p),
            None => a.clone(),
        })
        .filter(|a| sys::is_user_path(a, home))
        .collect()
}

/// La fenêtre au premier plan (X11 seulement ; sous Wayland, on ne peut pas le savoir).
fn active_pid() -> Option<u32> {
    std::env::var_os("DISPLAY")?;
    let root = sys::output("xprop", &["-root", "_NET_ACTIVE_WINDOW"])?;
    let id = root.rsplit(' ').next()?.trim().to_string();
    if !id.starts_with("0x") || id == "0x0" {
        return None;
    }
    let w = sys::output("xprop", &["-id", &id, "_NET_WM_PID"])?;
    w.rsplit('=').next()?.trim().parse().ok()
}

pub fn found_apps() -> Vec<FoundApp> {
    let entries = desktop_index();
    let by_id: HashMap<String, usize> = entries
        .iter()
        .enumerate()
        .map(|(i, e)| (e.id.trim_end_matches(".desktop").to_ascii_lowercase(), i))
        .collect();
    let mut by_exec: HashMap<String, usize> = HashMap::new();
    for (i, e) in entries.iter().enumerate() {
        by_exec.entry(e.exec_base.clone()).or_insert(i);
    }
    let home = std::env::var("HOME").unwrap_or_default();
    let active = active_pid();

    let mut apps: Vec<FoundApp> = Vec::new();
    for p in processes() {
        let from_scope = cgroup_app_ids(&p.cgroup).into_iter().find_map(|id| by_id.get(&id.to_ascii_lowercase()).copied());
        let argv0 = base_name(&p.argv[0]);
        let from_exec = [p.exe_base.as_str(), argv0.as_str()].into_iter().find_map(|b| {
            by_exec.get(b).or_else(|| ALIASES.iter().find(|(k, _)| *k == b).and_then(|(_, v)| by_exec.get(*v).or_else(|| by_id.get(*v)))).copied()
        });
        // Le nom du programme d'abord : le scope d'un terminal englobe tout ce qu'on y lance.
        let Some(idx) = from_exec.or(from_scope) else { continue };
        let e = &entries[idx];
        if e.id.to_ascii_lowercase().contains("reprise") {
            continue;
        }
        let files = file_args(&p.argv, &home);
        match apps.iter_mut().find(|a| a.id == e.id) {
            Some(a) => {
                a.frontmost |= active == Some(p.pid);
                for f in files {
                    if !a.files.contains(&f) {
                        a.files.push(f);
                    }
                }
            }
            None => apps.push(FoundApp { name: e.name.clone(), id: e.id.clone(), frontmost: active == Some(p.pid), files }),
        }
    }
    apps
}

// ─────────────────────────────── Capture ─────────────────────────────────────

pub fn running_apps() -> Result<Vec<RunningApp>, ScriptError> {
    Ok(found_apps()
        .into_iter()
        .map(|a| RunningApp { name: a.name, bundle_id: Some(a.id), frontmost: a.frontmost })
        .collect())
}

pub fn full_capture(_apps: &[RunningApp], _own: &str, ignored: &[String]) -> Snapshot {
    desktop::build_snapshot(&found_apps(), &[], ignored, Vec::new())
}

pub fn capture_all(own: &str, ignored: &[String]) -> Result<Snapshot, ScriptError> {
    Ok(full_capture(&[], own, ignored))
}

// ─────────────────────────────── Reprise ─────────────────────────────────────

pub struct Linux {
    entries: Vec<Entry>,
}

impl Launcher for Linux {
    fn launch(&self, id: &str, args: &[String]) -> Result<(), String> {
        if let Some(e) = self.entries.iter().find(|e| e.id == id) {
            let mut argv = e.exec.clone();
            let at = e.file_slot.unwrap_or(argv.len()).min(argv.len());
            for (k, a) in args.iter().enumerate() {
                argv.insert(at + k, a.clone());
            }
            let rest: Vec<&str> = argv.iter().skip(1).map(String::as_str).collect();
            if sys::spawn(&argv[0], &rest).is_ok() {
                return Ok(());
            }
        }
        if Path::new(id).is_absolute() {
            let rest: Vec<&str> = args.iter().map(String::as_str).collect();
            return sys::spawn(id, &rest);
        }
        let mut gl = vec![id.trim_end_matches(".desktop")];
        gl.extend(args.iter().map(String::as_str));
        sys::spawn("gtk-launch", &gl)
    }

    fn open(&self, target: &str) -> Result<(), String> {
        sys::spawn("xdg-open", &[target])
    }
}

pub fn execute(actions: &[Action], _own: &str) -> RestoreReport {
    desktop::execute(&Linux { entries: desktop_index() }, actions)
}

pub fn open_url(url: &str) -> Result<(), String> {
    sys::spawn("xdg-open", &[url])
}

/// Montre le fichier dans le gestionnaire de fichiers (sélectionné si possible).
pub fn reveal(path: &Path) -> Result<(), String> {
    let uri = format!("['file://{}']", path.to_string_lossy());
    let shown = sys::output(
        "gdbus",
        &[
            "call", "--session", "--dest", "org.freedesktop.FileManager1",
            "--object-path", "/org/freedesktop/FileManager1",
            "--method", "org.freedesktop.FileManager1.ShowItems", &uri, "",
        ],
    );
    if shown.is_some() {
        return Ok(());
    }
    let dir = if path.is_dir() { path } else { path.parent().unwrap_or(path) };
    sys::spawn("xdg-open", &[&dir.to_string_lossy()])
}

// ─────────────────────────── Filet de sécurité ───────────────────────────────

fn gdbus_call(dest: &str, object: &str, method: &str, args: &[&str]) -> Option<String> {
    let mut a = vec!["call", "--session", "--dest", dest, "--object-path", object, "--method", method];
    a.extend_from_slice(args);
    sys::output("gdbus", &a)
}

/// « (true,) » → true
pub fn parse_gdbus_bool(s: &str) -> Option<bool> {
    let t = s.trim().trim_start_matches('(').trim_end_matches(')').trim_end_matches(',').trim();
    match t {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

/// « (uint64 12345,) » → 12345
pub fn parse_gdbus_u64(s: &str) -> Option<u64> {
    s.split(|c: char| !c.is_ascii_digit()).filter(|t| !t.is_empty()).last()?.parse().ok()
}

pub fn screen_locked() -> Option<bool> {
    if let Ok(id) = std::env::var("XDG_SESSION_ID") {
        if let Some(v) = sys::output("loginctl", &["show-session", &id, "-p", "LockedHint", "--value"]) {
            match v.trim() {
                "yes" => return Some(true),
                "no" => return Some(false),
                _ => {}
            }
        }
    }
    gdbus_call("org.gnome.ScreenSaver", "/org/gnome/ScreenSaver", "org.gnome.ScreenSaver.GetActive", &[])
        .and_then(|o| parse_gdbus_bool(&o))
}

pub fn idle_seconds() -> Option<u64> {
    if let Some(ms) = gdbus_call(
        "org.gnome.Mutter.IdleMonitor",
        "/org/gnome/Mutter/IdleMonitor/Core",
        "org.gnome.Mutter.IdleMonitor.GetIdletime",
        &[],
    )
    .and_then(|o| parse_gdbus_u64(&o))
    {
        return Some(ms / 1000);
    }
    sys::output("xprintidle", &[]).and_then(|o| o.trim().parse::<u64>().ok()).map(|ms| ms / 1000)
}

/// Une vidéo ou un stream empêche la mise en veille de l'écran (GNOME).
pub fn display_kept_awake() -> Option<bool> {
    gdbus_call("org.gnome.SessionManager", "/org/gnome/SessionManager", "org.gnome.SessionManager.IsInhibited", &["8"])
        .and_then(|o| parse_gdbus_bool(&o))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn desktop_files() {
        let firefox = "[Desktop Entry]\nVersion=1.0\nName=Firefox Web Browser\nName[fr]=Navigateur Web Firefox\nExec=env BAMF_DESKTOP_FILE_HINT=/var/lib/snapd/desktop/applications/firefox_firefox.desktop /snap/bin/firefox %u\nType=Application\n\n[Desktop Action new-window]\nName=New Window\nExec=/snap/bin/firefox --new-window\n";
        let e = parse_desktop("firefox_firefox.desktop", firefox, "fr_FR").unwrap();
        assert_eq!(e.name, "Navigateur Web Firefox");
        assert_eq!(e.exec_base, "firefox");
        assert_eq!(e.file_slot, Some(3));
        let flat = "[Desktop Entry]\nName=Chromium\nExec=/usr/bin/flatpak run --branch=stable --arch=x86_64 --command=chromium org.chromium.Chromium @@u %U @@\nType=Application\n";
        assert_eq!(parse_desktop("org.chromium.Chromium.desktop", flat, "en_US").unwrap().exec_base, "org.chromium.chromium");
        assert!(parse_desktop("x.desktop", "[Desktop Entry]\nName=X\nExec=x\nType=Application\nNoDisplay=true\n", "").is_none());
        assert!(parse_desktop("t.desktop", "[Desktop Entry]\nName=Top\nExec=htop\nType=Application\nTerminal=true\n", "").is_none());
        assert_eq!(split_exec(r#"sh -c "echo \"a b\"" %f"#), vec!["sh", "-c", "echo \"a b\"", "%f"]);
    }

    #[test]
    fn scopes() {
        assert_eq!(
            cgroup_app_ids("0::/user.slice/user-1000.slice/user@1000.service/app.slice/app-gnome-org.gnome.TextEditor-4242.scope\n"),
            vec!["gnome-org.gnome.TextEditor", "org.gnome.TextEditor"]
        );
        assert_eq!(
            cgroup_app_ids("0::/user.slice/user-1000.slice/user@1000.service/app.slice/app-gnome-google\\x2dchrome-9876.scope"),
            vec!["gnome-google-chrome", "google-chrome"]
        );
        assert_eq!(
            cgroup_app_ids("0::/user.slice/user-1000.slice/user@1000.service/app.slice/app-flatpak-org.mozilla.firefox-1234.scope"),
            vec!["flatpak-org.mozilla.firefox", "org.mozilla.firefox"]
        );
        assert_eq!(cgroup_app_ids("0::/user.slice/user-1000.slice/user@1000.service/app.slice/app-org.kde.dolphin@a1b2.service"), vec!["org.kde.dolphin"]);
        assert!(cgroup_app_ids("0::/user.slice/user-1000.slice/session-2.scope").is_empty());
    }

    #[test]
    fn gdbus_answers() {
        assert_eq!(parse_gdbus_bool("(true,)\n"), Some(true));
        assert_eq!(parse_gdbus_bool("(false,)"), Some(false));
        assert_eq!(parse_gdbus_u64("(uint64 123456,)\n"), Some(123456));
    }

    #[test]
    fn launches_with_file_in_the_right_place() {
        let e = parse_desktop("ev.desktop", "[Desktop Entry]\nName=Docs\nExec=evince --fullscreen %U --x\nType=Application\n", "").unwrap();
        assert_eq!(e.exec, vec!["evince", "--fullscreen", "--x"]);
        assert_eq!(e.file_slot, Some(2));
    }

    #[test]
    fn real_system_does_not_panic() {
        // Sur la machine de test : on doit au moins pouvoir tout parcourir sans planter.
        let apps = found_apps();
        let _ = desktop::build_snapshot(&apps, &[], &[], Vec::new());
        let _ = (screen_locked(), idle_seconds(), display_kept_awake());
    }
}
