//! Onglets des navigateurs sous Windows et Linux.
//!
//! Il n'y a pas d'AppleScript hors du Mac. À la place, on lit le fichier où chaque navigateur
//! note lui-même ses fenêtres et ses onglets ouverts, pour pouvoir les rouvrir après un plantage :
//! - Firefox : `sessionstore-backups/recovery.jsonlz4` (du JSON compressé en LZ4) ;
//! - Chrome, Edge, Brave, Vivaldi, Opera… : `Sessions/Session_…` (le format « SNSS » de Chromium).
//!
//! Le navigateur met ces fichiers à jour toutes les quelques secondes : un onglet ouvert à
//! l'instant peut manquer. On ne fait que lire, jamais écrire.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[derive(Debug, Clone, PartialEq)]
pub struct Tab {
    /// Numéro de la fenêtre (1, 2, 3…), pour reconstruire les fenêtres.
    pub window: u32,
    pub url: String,
    pub title: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Firefox,
    Chromium,
}

/// Un navigateur connu : comment le reconnaître, et où il range ses sessions.
pub struct Browser {
    pub name: &'static str,
    pub kind: Kind,
    /// Morceaux de texte qui le trahissent dans le chemin du programme ou le nom de l'app.
    hints: &'static [&'static str],
    /// Dossiers de données sous Windows : (variable d'environnement, sous-dossier).
    windows: &'static [(&'static str, &'static str)],
    /// Dossiers de données sous Linux, depuis le dossier personnel.
    linux: &'static [&'static str],
}

/// L'ordre compte : Opera GX avant Opera, Chromium avant Chrome.
pub const BROWSERS: &[Browser] = &[
    Browser {
        name: "Firefox",
        kind: Kind::Firefox,
        hints: &["firefox"],
        windows: &[("APPDATA", "Mozilla/Firefox/Profiles")],
        linux: &[".mozilla/firefox", "snap/firefox/common/.mozilla/firefox", ".var/app/org.mozilla.firefox/.mozilla/firefox"],
    },
    Browser {
        name: "Opera GX",
        kind: Kind::Chromium,
        hints: &["opera_gx", "operagx", "opera gx"],
        windows: &[("APPDATA", "Opera Software/Opera GX Stable")],
        linux: &[],
    },
    Browser {
        name: "Opera",
        kind: Kind::Chromium,
        hints: &["opera"],
        windows: &[("APPDATA", "Opera Software/Opera Stable")],
        linux: &[".config/opera", "snap/opera/current/.config/opera", ".var/app/com.opera.Opera/config/opera"],
    },
    Browser {
        name: "Edge",
        kind: Kind::Chromium,
        hints: &["msedge", "microsoft-edge", "com.microsoft.edge"],
        windows: &[("LOCALAPPDATA", "Microsoft/Edge/User Data")],
        linux: &[".config/microsoft-edge", ".var/app/com.microsoft.Edge/config/microsoft-edge"],
    },
    Browser {
        name: "Brave",
        kind: Kind::Chromium,
        hints: &["brave"],
        windows: &[("LOCALAPPDATA", "BraveSoftware/Brave-Browser/User Data")],
        linux: &[
            ".config/BraveSoftware/Brave-Browser",
            "snap/brave/current/.config/BraveSoftware/Brave-Browser",
            ".var/app/com.brave.Browser/config/BraveSoftware/Brave-Browser",
        ],
    },
    Browser {
        name: "Vivaldi",
        kind: Kind::Chromium,
        hints: &["vivaldi"],
        windows: &[("LOCALAPPDATA", "Vivaldi/User Data")],
        linux: &[".config/vivaldi", ".var/app/com.vivaldi.Vivaldi/config/vivaldi"],
    },
    Browser {
        name: "Chromium",
        kind: Kind::Chromium,
        hints: &["chromium"],
        windows: &[("LOCALAPPDATA", "Chromium/User Data")],
        linux: &[".config/chromium", "snap/chromium/common/chromium", ".var/app/org.chromium.Chromium/config/chromium"],
    },
    Browser {
        name: "Chrome",
        kind: Kind::Chromium,
        hints: &["chrome"],
        windows: &[("LOCALAPPDATA", "Google/Chrome/User Data")],
        linux: &[".config/google-chrome", ".var/app/com.google.Chrome/config/google-chrome"],
    },
];

/// Reconnaît un navigateur d'après le chemin de son programme ou le nom de son app.
pub fn browser_for(identity: &str) -> Option<&'static Browser> {
    let id = identity.to_ascii_lowercase().replace('\\', "/");
    // Le nom du fichier compte plus que le dossier (« Google/Chrome/…/chrome_proxy.exe »…).
    let base = id.rsplit('/').next().unwrap_or(&id).to_string();
    let skip = ["proxy", "crashpad", "updater", "update", "helper", "notification"];
    if skip.iter().any(|s| base.contains(s)) {
        return None;
    }
    // Opera GX s'installe comme « Opera GX\opera.exe » : on regarde aussi le dossier.
    let gx = &BROWSERS[1];
    if gx.hints.iter().any(|h| id.contains(h)) {
        return Some(gx);
    }
    BROWSERS.iter().find(|b| b.hints.iter().any(|h| base.contains(h)))
}

/// Les dossiers de données possibles de ce navigateur, sur ce système.
fn data_dirs(b: &Browser) -> Vec<PathBuf> {
    if cfg!(windows) {
        b.windows
            .iter()
            .filter_map(|(var, rel)| std::env::var_os(var).map(|base| Path::new(&base).join(rel)))
            .collect()
    } else {
        let Some(home) = std::env::var_os("HOME") else { return Vec::new() };
        b.linux.iter().map(|rel| Path::new(&home).join(rel)).collect()
    }
}

fn modified(p: &Path) -> Option<SystemTime> {
    fs::metadata(p).and_then(|m| m.modified()).ok()
}

/// Les fichiers de session du navigateur, du plus récent au plus ancien.
fn session_files(b: &Browser) -> Vec<PathBuf> {
    let mut candidates: Vec<PathBuf> = Vec::new();
    for dir in data_dirs(b) {
        let Ok(entries) = fs::read_dir(&dir) else { continue };
        let mut profiles: Vec<PathBuf> = entries.flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect();
        profiles.push(dir.clone()); // Opera range tout directement dans son dossier.
        for prof in profiles {
            match b.kind {
                Kind::Firefox => candidates.push(prof.join("sessionstore-backups").join("recovery.jsonlz4")),
                Kind::Chromium => {
                    if let Ok(files) = fs::read_dir(prof.join("Sessions")) {
                        for f in files.flatten() {
                            if f.file_name().to_string_lossy().starts_with("Session_") && f.path().is_file() {
                                candidates.push(f.path());
                            }
                        }
                    }
                }
            }
        }
    }
    let mut dated: Vec<(SystemTime, PathBuf)> = candidates.into_iter().filter_map(|p| modified(&p).map(|t| (t, p))).collect();
    dated.sort_by(|a, b| b.0.cmp(&a.0));
    dated.into_iter().map(|(_, p)| p).collect()
}

/// Lit un fichier que le navigateur garde ouvert. Sous Windows, on dit explicitement qu'on
/// accepte de partager le fichier (lecture, écriture, suppression), sinon Windows peut refuser.
fn read_shared(path: &Path) -> std::io::Result<Vec<u8>> {
    use std::io::Read;
    let mut opts = fs::OpenOptions::new();
    opts.read(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        opts.share_mode(0x1 | 0x2 | 0x4); // FILE_SHARE_READ | WRITE | DELETE
    }
    let mut f = opts.open(path)?;
    let mut buf = Vec::new();
    f.read_to_end(&mut buf)?;
    Ok(buf)
}

/// Les onglets ouverts dans ce navigateur, d'après son fichier de session.
/// On essaie le fichier le plus récent, puis les précédents s'il est illisible ou vide.
pub fn read_tabs(b: &Browser) -> Result<Vec<Tab>, String> {
    let files = session_files(b);
    if files.is_empty() {
        return Err(format!("La session de {} est introuvable.", b.name));
    }
    let mut last_error = None;
    let mut read_one = false;
    for path in files.iter().take(4) {
        match read_shared(path) {
            Ok(data) => {
                read_one = true;
                let tabs: Vec<Tab> = match b.kind {
                    Kind::Firefox => parse_firefox(&data),
                    Kind::Chromium => parse_snss(&data),
                }
                .into_iter()
                .filter(|t| keep_url(&t.url))
                .collect();
                if !tabs.is_empty() {
                    return Ok(tabs);
                }
            }
            Err(e) => last_error = Some(e),
        }
    }
    match (read_one, last_error) {
        (true, _) | (false, None) => Ok(Vec::new()),
        (false, Some(e)) => Err(format!("La session de {} n'a pas pu être lue ({e}).", b.name)),
    }
}

/// Seules les vraies pages comptent (pas « Nouvel onglet », pas les pages de réglages).
pub fn keep_url(url: &str) -> bool {
    let u = url.trim().to_ascii_lowercase();
    u.starts_with("http://") || u.starts_with("https://") || u.starts_with("file://")
}

// ─────────────────────────────── Firefox ───────────────────────────────

/// Décompresse un bloc LZ4 (format « block », sans en-tête).
pub fn lz4_block(src: &[u8], expected: usize) -> Option<Vec<u8>> {
    let mut out: Vec<u8> = Vec::with_capacity(expected);
    let mut i = 0;
    while i < src.len() {
        let token = src[i];
        i += 1;
        let mut lit = (token >> 4) as usize;
        if lit == 15 {
            loop {
                let b = *src.get(i)?;
                i += 1;
                lit += b as usize;
                if b != 255 {
                    break;
                }
            }
        }
        out.extend_from_slice(src.get(i..i + lit)?);
        i += lit;
        if i >= src.len() {
            break; // la dernière séquence n'a que des littéraux
        }
        let offset = u16::from_le_bytes([*src.get(i)?, *src.get(i + 1)?]) as usize;
        i += 2;
        if offset == 0 || offset > out.len() {
            return None;
        }
        let mut len = (token & 15) as usize;
        if len == 15 {
            loop {
                let b = *src.get(i)?;
                i += 1;
                len += b as usize;
                if b != 255 {
                    break;
                }
            }
        }
        len += 4;
        let start = out.len() - offset;
        for k in 0..len {
            let b = out[start + k];
            out.push(b);
        }
        if out.len() > expected.max(1) * 2 + 1_000_000 {
            return None;
        }
    }
    Some(out)
}

/// Fichier « mozlz4 » : en-tête magique, taille, puis un bloc LZ4.
pub fn mozlz4(data: &[u8]) -> Option<Vec<u8>> {
    if data.len() < 12 || &data[..8] != b"mozLz40\0" {
        return None;
    }
    let size = u32::from_le_bytes([data[8], data[9], data[10], data[11]]) as usize;
    lz4_block(&data[12..], size)
}

pub fn parse_firefox(data: &[u8]) -> Vec<Tab> {
    let Some(raw) = mozlz4(data) else { return Vec::new() };
    let Ok(text) = String::from_utf8(raw) else { return Vec::new() };
    parse_firefox_json(&text)
}

pub fn parse_firefox_json(text: &str) -> Vec<Tab> {
    let Some(root) = json::parse(text) else { return Vec::new() };
    let mut tabs = Vec::new();
    for (wi, w) in root.get("windows").and_then(|w| w.as_array()).unwrap_or(&[]).iter().enumerate() {
        for t in w.get("tabs").and_then(|t| t.as_array()).unwrap_or(&[]) {
            if t.get("hidden").and_then(|h| h.as_bool()) == Some(true) {
                continue;
            }
            let entries = t.get("entries").and_then(|e| e.as_array()).unwrap_or(&[]);
            if entries.is_empty() {
                continue;
            }
            let idx = t.get("index").and_then(|i| i.as_f64()).map(|i| i as usize).unwrap_or(entries.len());
            let e = &entries[idx.clamp(1, entries.len()) - 1];
            let url = e.get("url").and_then(|u| u.as_str()).unwrap_or("").to_string();
            if url.is_empty() {
                continue;
            }
            let title = e.get("title").and_then(|u| u.as_str()).filter(|s| !s.is_empty()).unwrap_or(&url).to_string();
            tabs.push(Tab { window: wi as u32 + 1, url, title });
        }
    }
    tabs
}

// ─────────────────────────────── Chromium ──────────────────────────────

const SET_TAB_WINDOW: u8 = 0;
const SET_TAB_INDEX_IN_WINDOW: u8 = 2;
const PRUNED_FROM_BACK: u8 = 5;
const UPDATE_TAB_NAVIGATION: u8 = 6;
const SET_SELECTED_NAVIGATION_INDEX: u8 = 7;
const SET_WINDOW_TYPE: u8 = 9;
const PRUNED_FROM_FRONT: u8 = 11;
const TAB_CLOSED: u8 = 16;
const WINDOW_CLOSED: u8 = 17;
const NAVIGATION_PATH_PRUNED: u8 = 24;

#[derive(Default)]
struct SnssTab {
    window: Option<i32>,
    visual_index: i32,
    selected: Option<i32>,
    navs: HashMap<i32, (String, String)>,
}

fn i32_at(p: &[u8], at: usize) -> Option<i32> {
    p.get(at..at + 4).map(|b| i32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

/// Lit une navigation « picklée » : taille, n° d'onglet, n° de navigation, URL, titre.
fn read_navigation(p: &[u8]) -> Option<(i32, i32, String, String)> {
    let mut at = 4; // en-tête du pickle : taille du contenu
    let tab = i32_at(p, at)?;
    at += 4;
    let index = i32_at(p, at)?;
    at += 4;
    let len = usize::try_from(i32_at(p, at)?).ok()?;
    at += 4;
    let url = String::from_utf8_lossy(p.get(at..at + len)?).into_owned();
    at += len.div_ceil(4) * 4;
    let title = match i32_at(p, at).and_then(|n| usize::try_from(n).ok()) {
        Some(n) => {
            let bytes = p.get(at + 4..at + 4 + n * 2).unwrap_or(&[]);
            let units: Vec<u16> = bytes.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
            String::from_utf16_lossy(&units)
        }
        None => String::new(),
    };
    Some((tab, index, url, title))
}

pub fn parse_snss(data: &[u8]) -> Vec<Tab> {
    if data.len() < 8 || &data[..4] != b"SNSS" {
        return Vec::new();
    }
    let version = i32_at(data, 4).unwrap_or(0);
    if version != 1 && version != 3 {
        return Vec::new(); // versions chiffrées : illisibles
    }
    let mut tabs: HashMap<i32, SnssTab> = HashMap::new();
    let mut closed_windows: Vec<i32> = Vec::new();
    let mut window_types: HashMap<i32, i32> = HashMap::new();
    let mut i = 8;
    while i + 2 <= data.len() {
        let size = u16::from_le_bytes([data[i], data[i + 1]]) as usize;
        i += 2;
        if size == 0 || i + size > data.len() {
            break;
        }
        let id = data[i];
        let p = &data[i + 1..i + size];
        i += size;
        match id {
            SET_TAB_WINDOW => {
                if let (Some(w), Some(t)) = (i32_at(p, 0), i32_at(p, 4)) {
                    tabs.entry(t).or_default().window = Some(w);
                }
            }
            SET_TAB_INDEX_IN_WINDOW => {
                if let (Some(t), Some(ix)) = (i32_at(p, 0), i32_at(p, 4)) {
                    tabs.entry(t).or_default().visual_index = ix;
                }
            }
            UPDATE_TAB_NAVIGATION => {
                if let Some((t, ix, url, title)) = read_navigation(p) {
                    tabs.entry(t).or_default().navs.insert(ix, (url, title));
                }
            }
            SET_SELECTED_NAVIGATION_INDEX => {
                if let (Some(t), Some(ix)) = (i32_at(p, 0), i32_at(p, 4)) {
                    tabs.entry(t).or_default().selected = Some(ix);
                }
            }
            SET_WINDOW_TYPE => {
                if let (Some(w), Some(ty)) = (i32_at(p, 0), i32_at(p, 4)) {
                    window_types.insert(w, ty);
                }
            }
            PRUNED_FROM_BACK => {
                if let (Some(t), Some(ix)) = (i32_at(p, 0), i32_at(p, 4)) {
                    if let Some(tab) = tabs.get_mut(&t) {
                        tab.navs.retain(|k, _| *k < ix);
                    }
                }
            }
            PRUNED_FROM_FRONT => {
                if let (Some(t), Some(n)) = (i32_at(p, 0), i32_at(p, 4)) {
                    if let Some(tab) = tabs.get_mut(&t) {
                        tab.navs = tab.navs.drain().filter(|(k, _)| *k >= n).map(|(k, v)| (k - n, v)).collect();
                        tab.selected = tab.selected.map(|s| (s - n).max(0));
                    }
                }
            }
            NAVIGATION_PATH_PRUNED => {
                if let (Some(t), Some(ix), Some(n)) = (i32_at(p, 0), i32_at(p, 4), i32_at(p, 8)) {
                    if let Some(tab) = tabs.get_mut(&t) {
                        tab.navs = tab
                            .navs
                            .drain()
                            .filter(|(k, _)| *k < ix || *k >= ix + n)
                            .map(|(k, v)| if k >= ix + n { (k - n, v) } else { (k, v) })
                            .collect();
                    }
                }
            }
            TAB_CLOSED => {
                if let Some(t) = i32_at(p, 0) {
                    tabs.remove(&t);
                }
            }
            WINDOW_CLOSED => {
                if let Some(w) = i32_at(p, 0) {
                    closed_windows.push(w);
                }
            }
            _ => {}
        }
    }

    // Fenêtres normales (pas les pop-up ni les outils de développement), dans l'ordre de création.
    let mut windows: Vec<i32> = tabs
        .values()
        .filter_map(|t| t.window)
        .filter(|w| !closed_windows.contains(w) && window_types.get(w).copied().unwrap_or(0) == 0)
        .collect();
    windows.sort_unstable();
    windows.dedup();

    let mut out = Vec::new();
    for (wn, w) in windows.iter().enumerate() {
        let mut in_window: Vec<(i32, i32, &SnssTab)> = tabs
            .iter()
            .filter(|(_, t)| t.window == Some(*w) && !t.navs.is_empty())
            .map(|(id, t)| (t.visual_index, *id, t))
            .collect();
        in_window.sort_by_key(|(vi, id, _)| (*vi, *id));
        for (_, _, t) in in_window {
            let nav = t
                .selected
                .and_then(|s| t.navs.get(&s))
                .or_else(|| t.navs.iter().max_by_key(|(k, _)| **k).map(|(_, v)| v));
            if let Some((url, title)) = nav {
                let title = if title.is_empty() { url.clone() } else { title.clone() };
                out.push(Tab { window: wn as u32 + 1, url: url.clone(), title });
            }
        }
    }
    out
}

// ──────────────────────── Un tout petit lecteur JSON ────────────────────────
// (Pour ne lire que ce qu'il faut dans la session de Firefox, sans dépendre d'autre chose.)

pub mod json {
    #[derive(Debug, Clone, PartialEq)]
    pub enum Value {
        Null,
        Bool(bool),
        Num(f64),
        Str(String),
        Arr(Vec<Value>),
        Obj(Vec<(String, Value)>),
    }

    impl Value {
        pub fn get(&self, key: &str) -> Option<&Value> {
            match self {
                Value::Obj(kv) => kv.iter().find(|(k, _)| k == key).map(|(_, v)| v),
                _ => None,
            }
        }
        pub fn as_array(&self) -> Option<&[Value]> {
            match self {
                Value::Arr(a) => Some(a),
                _ => None,
            }
        }
        pub fn as_str(&self) -> Option<&str> {
            match self {
                Value::Str(s) => Some(s),
                _ => None,
            }
        }
        pub fn as_f64(&self) -> Option<f64> {
            match self {
                Value::Num(n) => Some(*n),
                _ => None,
            }
        }
        pub fn as_bool(&self) -> Option<bool> {
            match self {
                Value::Bool(b) => Some(*b),
                _ => None,
            }
        }
    }

    pub fn parse(text: &str) -> Option<Value> {
        let b = text.as_bytes();
        let mut i = 0;
        let v = value(b, &mut i, 0)?;
        Some(v)
    }

    fn ws(b: &[u8], i: &mut usize) {
        while *i < b.len() && matches!(b[*i], b' ' | b'\n' | b'\r' | b'\t') {
            *i += 1;
        }
    }

    fn value(b: &[u8], i: &mut usize, depth: u32) -> Option<Value> {
        if depth > 200 {
            return None;
        }
        ws(b, i);
        match *b.get(*i)? {
            b'{' => {
                *i += 1;
                let mut kv = Vec::new();
                ws(b, i);
                if b.get(*i) == Some(&b'}') {
                    *i += 1;
                    return Some(Value::Obj(kv));
                }
                loop {
                    ws(b, i);
                    let k = string(b, i)?;
                    ws(b, i);
                    if b.get(*i) != Some(&b':') {
                        return None;
                    }
                    *i += 1;
                    let v = value(b, i, depth + 1)?;
                    kv.push((k, v));
                    ws(b, i);
                    match b.get(*i)? {
                        b',' => *i += 1,
                        b'}' => {
                            *i += 1;
                            return Some(Value::Obj(kv));
                        }
                        _ => return None,
                    }
                }
            }
            b'[' => {
                *i += 1;
                let mut arr = Vec::new();
                ws(b, i);
                if b.get(*i) == Some(&b']') {
                    *i += 1;
                    return Some(Value::Arr(arr));
                }
                loop {
                    arr.push(value(b, i, depth + 1)?);
                    ws(b, i);
                    match b.get(*i)? {
                        b',' => *i += 1,
                        b']' => {
                            *i += 1;
                            return Some(Value::Arr(arr));
                        }
                        _ => return None,
                    }
                }
            }
            b'"' => string(b, i).map(Value::Str),
            b't' if b[*i..].starts_with(b"true") => {
                *i += 4;
                Some(Value::Bool(true))
            }
            b'f' if b[*i..].starts_with(b"false") => {
                *i += 5;
                Some(Value::Bool(false))
            }
            b'n' if b[*i..].starts_with(b"null") => {
                *i += 4;
                Some(Value::Null)
            }
            _ => {
                let start = *i;
                while *i < b.len() && matches!(b[*i], b'-' | b'+' | b'.' | b'e' | b'E' | b'0'..=b'9') {
                    *i += 1;
                }
                std::str::from_utf8(&b[start..*i]).ok()?.parse::<f64>().ok().map(Value::Num)
            }
        }
    }

    fn hex4(b: &[u8], at: usize) -> Option<u32> {
        u32::from_str_radix(std::str::from_utf8(b.get(at..at + 4)?).ok()?, 16).ok()
    }

    fn string(b: &[u8], i: &mut usize) -> Option<String> {
        if b.get(*i) != Some(&b'"') {
            return None;
        }
        *i += 1;
        let mut out: Vec<u8> = Vec::new();
        loop {
            let c = *b.get(*i)?;
            *i += 1;
            match c {
                b'"' => return String::from_utf8(out).ok(),
                b'\\' => {
                    let e = *b.get(*i)?;
                    *i += 1;
                    match e {
                        b'n' => out.push(b'\n'),
                        b't' => out.push(b'\t'),
                        b'r' => out.push(b'\r'),
                        b'b' => out.push(8),
                        b'f' => out.push(12),
                        b'u' => {
                            let mut cp = hex4(b, *i)?;
                            *i += 4;
                            if (0xD800..0xDC00).contains(&cp) && b.get(*i) == Some(&b'\\') && b.get(*i + 1) == Some(&b'u') {
                                if let Some(lo) = hex4(b, *i + 2).filter(|lo| (0xDC00..0xE000).contains(lo)) {
                                    cp = 0x10000 + ((cp - 0xD800) << 10) + (lo - 0xDC00);
                                    *i += 6;
                                }
                            }
                            let ch = char::from_u32(cp).unwrap_or('\u{FFFD}');
                            let mut buf = [0u8; 4];
                            out.extend_from_slice(ch.encode_utf8(&mut buf).as_bytes());
                        }
                        other => out.push(other),
                    }
                }
                _ => out.push(c),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Compresse « à la main » en LZ4 : uniquement des littéraux (suffisant pour tester la lecture).
    fn lz4_literals(data: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        let n = data.len();
        if n < 15 {
            out.push((n as u8) << 4);
        } else {
            out.push(0xF0);
            let mut rest = n - 15;
            while rest >= 255 {
                out.push(255);
                rest -= 255;
            }
            out.push(rest as u8);
        }
        out.extend_from_slice(data);
        out
    }

    #[test]
    fn lz4_with_a_match() {
        // « abcabcabcabc » : 3 littéraux puis une copie de 9 octets à 3 en arrière.
        let block = [0x35, b'a', b'b', b'c', 3, 0];
        assert_eq!(lz4_block(&block, 12).unwrap(), b"abcabcabcabc");
        assert!(lz4_block(&[0x15, b'a', 9, 0], 10).is_none(), "décalage impossible refusé");
    }

    #[test]
    fn firefox_session() {
        let json = r#"{"version":["sessionrestore",1],"windows":[
          {"tabs":[
            {"entries":[{"url":"https://a.fr/","title":"A"},{"url":"https://a.fr/2","title":"A deux \u00e9t\u00e9"}],"index":2},
            {"entries":[{"url":"about:newtab","title":"Nouvel onglet"}],"index":1},
            {"entries":[{"url":"https://cache.fr","title":"x"}],"index":1,"hidden":true}
          ],"selected":1},
          {"tabs":[{"entries":[{"url":"https://b.fr/","title":""}],"index":1}]}
        ],"_closedWindows":[]}"#;
        let mut file = b"mozLz40\0".to_vec();
        file.extend_from_slice(&(json.len() as u32).to_le_bytes());
        file.extend_from_slice(&lz4_literals(json.as_bytes()));
        let tabs: Vec<Tab> = parse_firefox(&file).into_iter().filter(|t| keep_url(&t.url)).collect();
        assert_eq!(
            tabs,
            vec![
                Tab { window: 1, url: "https://a.fr/2".into(), title: "A deux été".into() },
                Tab { window: 2, url: "https://b.fr/".into(), title: "https://b.fr/".into() },
            ]
        );
    }

    fn cmd(id: u8, payload: &[u8]) -> Vec<u8> {
        let mut v = ((payload.len() + 1) as u16).to_le_bytes().to_vec();
        v.push(id);
        v.extend_from_slice(payload);
        v
    }
    fn ints(xs: &[i32]) -> Vec<u8> {
        xs.iter().flat_map(|x| x.to_le_bytes()).collect()
    }
    fn nav(tab: i32, index: i32, url: &str, title: &str) -> Vec<u8> {
        let mut body = ints(&[tab, index, url.len() as i32]);
        body.extend_from_slice(url.as_bytes());
        while body.len() % 4 != 0 {
            body.push(0);
        }
        let units: Vec<u16> = title.encode_utf16().collect();
        body.extend_from_slice(&(units.len() as i32).to_le_bytes());
        for u in &units {
            body.extend_from_slice(&u.to_le_bytes());
        }
        while body.len() % 4 != 0 {
            body.push(0);
        }
        body.extend_from_slice(&ints(&[0, 0])); // la suite (état de la page…), ignorée
        let mut p = (body.len() as u32).to_le_bytes().to_vec();
        p.extend_from_slice(&body);
        p
    }

    #[test]
    fn chromium_session() {
        let mut f = b"SNSS".to_vec();
        f.extend_from_slice(&3i32.to_le_bytes());
        // Fenêtre 7 : onglets 1 et 2 (2 affiché en premier). Fenêtre 9 : onglet 3. Fenêtre 11 fermée.
        f.extend(cmd(SET_TAB_WINDOW, &ints(&[7, 1])));
        f.extend(cmd(SET_TAB_WINDOW, &ints(&[7, 2])));
        f.extend(cmd(SET_TAB_WINDOW, &ints(&[9, 3])));
        f.extend(cmd(SET_TAB_WINDOW, &ints(&[11, 4])));
        f.extend(cmd(SET_TAB_INDEX_IN_WINDOW, &ints(&[1, 1])));
        f.extend(cmd(SET_TAB_INDEX_IN_WINDOW, &ints(&[2, 0])));
        f.extend(cmd(UPDATE_TAB_NAVIGATION, &nav(1, 0, "https://un.fr/", "Un")));
        f.extend(cmd(UPDATE_TAB_NAVIGATION, &nav(1, 1, "https://un.fr/suite", "Un, la suite")));
        f.extend(cmd(SET_SELECTED_NAVIGATION_INDEX, &ints(&[1, 1])));
        f.extend(cmd(UPDATE_TAB_NAVIGATION, &nav(2, 0, "https://deux.fr/", "Deux — été")));
        f.extend(cmd(UPDATE_TAB_NAVIGATION, &nav(3, 0, "https://trois.fr/", "")));
        f.extend(cmd(UPDATE_TAB_NAVIGATION, &nav(4, 0, "https://ferme.fr/", "Fermée")));
        f.extend(cmd(UPDATE_TAB_NAVIGATION, &nav(5, 0, "https://onglet-ferme.fr/", "x")));
        f.extend(cmd(SET_TAB_WINDOW, &ints(&[9, 5])));
        f.extend(cmd(TAB_CLOSED, &[ints(&[5]), vec![0; 8]].concat()));
        f.extend(cmd(WINDOW_CLOSED, &[ints(&[11]), vec![0; 8]].concat()));
        f.extend(cmd(255, &[1, 2, 3]));
        let tabs = parse_snss(&f);
        assert_eq!(
            tabs,
            vec![
                Tab { window: 1, url: "https://deux.fr/".into(), title: "Deux — été".into() },
                Tab { window: 1, url: "https://un.fr/suite".into(), title: "Un, la suite".into() },
                Tab { window: 2, url: "https://trois.fr/".into(), title: "https://trois.fr/".into() },
            ]
        );
        assert!(parse_snss(b"SNSS\x02\0\0\0").is_empty(), "version chiffrée ignorée");
    }

    #[test]
    fn recognises_browsers() {
        let n = |s: &str| browser_for(s).map(|b| b.name);
        assert_eq!(n(r"C:\Program Files\Google\Chrome\Application\chrome.exe"), Some("Chrome"));
        assert_eq!(n(r"C:\Users\moi\AppData\Local\Programs\Opera GX\opera.exe"), Some("Opera GX"));
        assert_eq!(n(r"C:\Users\moi\AppData\Local\Programs\Opera\opera.exe"), Some("Opera"));
        assert_eq!(n(r"C:\Users\moi\AppData\Local\Programs\Opera GX\opera_gx.exe"), Some("Opera GX"));
        assert_eq!(n(r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"), Some("Edge"));
        assert_eq!(n("firefox_firefox.desktop"), Some("Firefox"));
        assert_eq!(n("chromium_chromium.desktop"), Some("Chromium"));
        assert_eq!(n("google-chrome.desktop"), Some("Chrome"));
        assert_eq!(n(r"C:\Program Files\Google\Chrome\Application\chrome_proxy.exe"), None);
        assert_eq!(n(r"C:\Windows\notepad.exe"), None);
    }

    #[test]
    fn json_reader() {
        let v = json::parse(r#"{"a":[1,2.5,-3e2],"b":{"c":"\ud83d\ude00 ok"},"d":true,"e":null}"#).unwrap();
        assert_eq!(v.get("a").unwrap().as_array().unwrap().len(), 3);
        assert_eq!(v.get("b").unwrap().get("c").unwrap().as_str(), Some("😀 ok"));
        assert_eq!(v.get("d").unwrap().as_bool(), Some(true));
        assert!(json::parse("{\"a\":}").is_none());
    }
}
