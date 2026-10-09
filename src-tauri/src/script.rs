//! Tout ce qui parle à macOS passe par AppleScript (`osascript`) et `open`.
//! Les scripts renvoient du texte découpé avec deux caractères de contrôle
//! (séparateur de champ 0x1F, séparateur d'enregistrement 0x1E) pour ne jamais
//! se faire piéger par une virgule ou un retour à la ligne dans un titre d'onglet.

use crate::model::Frame;
use std::io::{Read, Write};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

pub const US: char = '\u{1f}';
pub const RS: char = '\u{1e}';

/// Pourquoi un script a échoué, dans des catégories utiles pour l'utilisateur.
#[derive(Debug, Clone, PartialEq)]
pub enum ScriptError {
    /// macOS refuse que Reprise pilote cette app (Automatisation, erreur -1743).
    NotAuthorized,
    /// Accès « Accessibilité » manquant (erreurs -1719 / -25211).
    Accessibility,
    /// L'app ciblée n'est pas lancée (-600).
    NotRunning,
    /// Le script a mis trop longtemps (app bloquée, dialogue resté ouvert…).
    Timeout,
    Other(String),
}

impl ScriptError {
    pub fn classify(stderr: &str) -> ScriptError {
        let s = stderr.to_lowercase();
        if s.contains("-1743") || s.contains("not authorized") || s.contains("pas autoris") {
            ScriptError::NotAuthorized
        } else if s.contains("-1719")
            || s.contains("-25211")
            || s.contains("assistive")
            || s.contains("accès d’aide")
            || s.contains("accès d'aide")
        {
            ScriptError::Accessibility
        } else if s.contains("-600") {
            ScriptError::NotRunning
        } else {
            ScriptError::Other(stderr.trim().to_string())
        }
    }
}

/// Abstraction pour pouvoir tester la logique sans macOS.
pub trait Runner {
    fn run(&self, script: &str, args: &[String]) -> Result<String, ScriptError>;
}

/// Le vrai lanceur : `osascript - arg1 arg2 …` avec le script envoyé sur l'entrée standard.
/// Les arguments arrivent dans `on run argv` : aucune chaîne n'est jamais collée dans le
/// code AppleScript, donc pas de problème d'échappement avec les URL ou les chemins.
pub struct Osascript {
    pub timeout: Duration,
}

impl Default for Osascript {
    fn default() -> Self {
        Osascript { timeout: Duration::from_secs(45) }
    }
}

impl Runner for Osascript {
    fn run(&self, script: &str, args: &[String]) -> Result<String, ScriptError> {
        let mut child = Command::new("/usr/bin/osascript")
            .arg("-")
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| ScriptError::Other(format!("osascript introuvable : {e}")))?;

        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(script.as_bytes())
                .map_err(|e| ScriptError::Other(e.to_string()))?;
            // `stdin` est fermé ici, osascript peut lire la fin du script.
        }

        // On lit la sortie dans des fils à part : sinon, avec beaucoup d'onglets, le tuyau
        // se remplit, osascript attend qu'on le vide et tout reste bloqué.
        let mut out_pipe = child.stdout.take();
        let mut err_pipe = child.stderr.take();
        let out_reader = std::thread::spawn(move || {
            let mut buf = Vec::new();
            if let Some(p) = out_pipe.as_mut() {
                let _ = p.read_to_end(&mut buf);
            }
            buf
        });
        let err_reader = std::thread::spawn(move || {
            let mut buf = Vec::new();
            if let Some(p) = err_pipe.as_mut() {
                let _ = p.read_to_end(&mut buf);
            }
            buf
        });

        let start = Instant::now();
        let status = loop {
            match child.try_wait() {
                Ok(Some(st)) => break st,
                Ok(None) => {
                    if start.elapsed() > self.timeout {
                        let _ = child.kill();
                        let _ = child.wait();
                        return Err(ScriptError::Timeout);
                    }
                    std::thread::sleep(Duration::from_millis(25));
                }
                Err(e) => return Err(ScriptError::Other(e.to_string())),
            }
        };

        let stdout = out_reader.join().unwrap_or_default();
        let stderr = err_reader.join().unwrap_or_default();
        if status.success() {
            let mut s = String::from_utf8_lossy(&stdout).into_owned();
            if s.ends_with('\n') {
                s.pop();
            }
            Ok(s)
        } else {
            Err(ScriptError::classify(&String::from_utf8_lossy(&stderr)))
        }
    }
}

/// Lance `open` (ouvrir une app, un dossier, un fichier, une URL).
pub fn open(args: &[&str]) -> Result<(), String> {
    let out = Command::new("/usr/bin/open")
        .args(args)
        .output()
        .map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

// ───────────────────────────── Scripts de capture ─────────────────────────────

/// Les apps « visibles » (celles qui ont une icône dans le Dock).
/// Champs : nom affiché, bundle id, premier plan (true/false).
pub const APPS: &str = r#"on run argv
	set US to character id 31
	set RS to character id 30
	set out to ""
	tell application "System Events"
		set procs to every application process whose background only is false
		repeat with p in procs
			try
				set n to ""
				try
					set n to displayed name of p
				end try
				if n is missing value or n is "" then set n to name of p
				set b to bundle identifier of p
				if b is missing value then set b to ""
				set f to frontmost of p
				set out to out & n & US & b & US & (f as text) & RS
			end try
		end repeat
	end tell
	return out
end run"#;

/// Onglets de Safari. Champs : n° de fenêtre, URL, titre, cadre de la fenêtre « x,y,l,h ».
const SAFARI_TABS: &str = r#"on run argv
	set US to character id 31
	set RS to character id 30
	set out to ""
	tell application id "__BUNDLE__"
		set wi to 0
		repeat with w in windows
			set wi to wi + 1
			try
				set bd to ""
				try
					set b to bounds of w
					set bd to ((item 1 of b) as text) & "," & ((item 2 of b) as text) & "," & (((item 3 of b) - (item 1 of b)) as text) & "," & (((item 4 of b) - (item 2 of b)) as text)
				end try
				repeat with t in tabs of w
					try
						set u to URL of t
						if u is not missing value then
							set ti to name of t
							if ti is missing value then set ti to u
							set out to out & wi & US & u & US & ti & US & bd & RS
						end if
					end try
				end repeat
			end try
		end repeat
	end tell
	return out
end run"#;

/// Onglets des navigateurs Chromium (Chrome, Brave, Edge, Arc, Vivaldi, Opera…). Mêmes champs.
const CHROMIUM_TABS: &str = r#"on run argv
	set US to character id 31
	set RS to character id 30
	set out to ""
	tell application id "__BUNDLE__"
		set wi to 0
		repeat with w in windows
			set wi to wi + 1
			try
				set bd to ""
				try
					set b to bounds of w
					set bd to ((item 1 of b) as text) & "," & ((item 2 of b) as text) & "," & (((item 3 of b) - (item 1 of b)) as text) & "," & (((item 4 of b) - (item 2 of b)) as text)
				end try
				repeat with t in tabs of w
					try
						set u to URL of t
						if u is not missing value then
							set ti to title of t
							if ti is missing value then set ti to u
							set out to out & wi & US & u & US & ti & US & bd & RS
						end if
					end try
				end repeat
			end try
		end repeat
	end tell
	return out
end run"#;

/// Dossiers ouverts dans le Finder. Champs : chemin POSIX, cadre « x,y,l,h ».
pub const FINDER_FOLDERS: &str = r#"on run argv
	set US to character id 31
	set RS to character id 30
	set out to ""
	tell application "Finder"
		repeat with w in Finder windows
			try
				set p to POSIX path of (target of w as alias)
				set bd to ""
				try
					set b to bounds of w
					set bd to ((item 1 of b) as text) & "," & ((item 2 of b) as text) & "," & (((item 3 of b) - (item 1 of b)) as text) & "," & (((item 4 of b) - (item 2 of b)) as text)
				end try
				set out to out & p & US & bd & RS
			end try
		end repeat
	end tell
	return out
end run"#;

/// Toutes les fenêtres des apps, lues par l'Accessibilité : le fichier ouvert (AXDocument),
/// le titre, la position et la taille. Renvoie « !AX » si l'accès Accessibilité manque.
/// Champs : bundle id, nom de l'app, URL du document, titre, x, y, largeur, hauteur, réduite.
pub const WINDOWS: &str = r#"on run argv
	set US to character id 31
	set RS to character id 30
	set out to ""
	tell application "System Events"
		try
			set fp to first application process whose frontmost is true
			count windows of fp
		on error errMsg number errNum
			if errNum is -1719 or errNum is -25211 then return "!AX"
		end try
		repeat with p in (every application process whose background only is false)
			try
				set b to bundle identifier of p
				if b is missing value then set b to ""
				set n to ""
				try
					set n to displayed name of p
				end try
				if n is missing value or n is "" then set n to name of p
				repeat with w in (every window of p)
					try
						set d to ""
						try
							set d to value of attribute "AXDocument" of w
							if d is missing value then set d to ""
						end try
						set t to ""
						try
							set t to name of w
							if t is missing value then set t to ""
						end try
						set mn to false
						try
							set mn to value of attribute "AXMinimized" of w
						end try
						set ps to position of w
						set sz to size of w
						set out to out & b & US & n & US & d & US & t & US & ((item 1 of ps) as text) & US & ((item 2 of ps) as text) & US & ((item 1 of sz) as text) & US & ((item 2 of sz) as text) & US & (mn as text) & RS
					end try
				end repeat
			end try
		end repeat
	end tell
	return out
end run"#;

// ──────────────────────────── Scripts de restauration ───────────────────────────

// Les deux scripts suivants reçoivent : argument 1 = cadre « x,y,l,h » ou vide, puis les URL.
// Ils lancent le navigateur et attendent qu'il soit prêt (sinon la première commande échoue
// quand il était fermé), ouvrent chaque onglet indépendamment (une URL refusée n'empêche
// pas les suivantes) et renvoient les numéros des URL qui n'ont pas pu être ouvertes,
// séparés par des virgules, pour que Reprise les rouvre autrement.

/// Attend qu'un navigateur soit lancé et prêt à recevoir des commandes.
const WAIT_LAUNCH: &str = r#"	tell application id "__BUNDLE__" to launch
	repeat 50 times
		if application id "__BUNDLE__" is running then exit repeat
		delay 0.2
	end repeat
	delay 0.4
"#;

/// Place la fenêtre `w` si un cadre est donné.
const SET_BOUNDS: &str = r#"		if (item 1 of argv) is not "" then
			try
				set AppleScript's text item delimiters to ","
				set f to text items of (item 1 of argv)
				set AppleScript's text item delimiters to ""
				set x to (item 1 of f) as integer
				set y to (item 2 of f) as integer
				set bounds of w to {x, y, x + ((item 3 of f) as integer), y + ((item 4 of f) as integer)}
			end try
		end if
"#;

const RETURN_FAILED: &str = r#"	set AppleScript's text item delimiters to ","
	set out to failed as text
	set AppleScript's text item delimiters to ""
	return out
"#;

/// Ouvre une nouvelle fenêtre Safari avec ses onglets.
/// Chaque ordre n'est donné qu'une seule fois : certains navigateurs renvoient une erreur
/// alors qu'ils ont bien obéi, et réessayer ouvrirait des fenêtres en boucle. On vérifie
/// donc en comptant les fenêtres et les onglets plutôt qu'en se fiant à l'erreur.
const SAFARI_OPEN: &str = r#"on run argv
	set failed to {}
__WAIT__	tell application id "__BUNDLE__"
__READY__		try
			make new document with properties {URL:(item 2 of argv)}
		end try
__NEWWIN__		repeat with i from 3 to count of argv
			set tc to count of tabs of w
			try
				tell w to make new tab at end of tabs with properties {URL:(item i of argv)}
			on error
				delay 0.3
				if (count of tabs of w) is tc then set end of failed to (i - 1)
			end try
		end repeat
__BOUNDS__		activate
	end tell
__RETURN__end run"#;

/// Ouvre une nouvelle fenêtre Chromium (Chrome, Brave, Edge, Arc, Vivaldi, Opera…) avec ses onglets.
const CHROMIUM_OPEN: &str = r#"on run argv
	set failed to {}
__WAIT__	tell application id "__BUNDLE__"
__READY__		try
			make new window
		end try
__NEWWIN__		try
			set URL of active tab of w to (item 2 of argv)
		on error
			set end of failed to 1
		end try
		repeat with i from 3 to count of argv
			set tc to count of tabs of w
			try
				tell w to make new tab with properties {URL:(item i of argv)}
			on error
				delay 0.3
				if (count of tabs of w) is tc then set end of failed to (i - 1)
			end try
		end repeat
__BOUNDS__		activate
	end tell
__RETURN__end run"#;

/// Attend que le navigateur réponde, et note combien il a de fenêtres.
const READY: &str = r#"		set n0 to -1
		repeat 40 times
			try
				set n0 to count of windows
				exit repeat
			on error
				delay 0.25
			end try
		end repeat
		if n0 is -1 then error "Le navigateur ne répond pas." number 9001
"#;

/// Vérifie qu'une fenêtre est bien apparue (sans jamais redemander), puis la garde dans `w`.
const NEW_WINDOW: &str = r#"		set made to false
		repeat 30 times
			if (count of windows) > n0 then
				set made to true
				exit repeat
			end if
			delay 0.2
		end repeat
		if not made then error "Le navigateur n'a pas ouvert de fenêtre." number 9001
		-- La nouvelle fenêtre passe devant : c'est la n° 1. On garde une référence fixe.
		set w to window 1
		try
			set w to window id (id of w)
		end try
"#;

/// Ouvre un dossier dans une nouvelle fenêtre du Finder, à sa place.
/// Arguments : chemin POSIX, cadre « x,y,l,h » ou vide.
pub const FINDER_OPEN: &str = r#"on run argv
	set p to (POSIX file (item 1 of argv)) as alias
	tell application "Finder"
		set w to make new Finder window
		set target of w to p
		if (item 2 of argv) is not "" then
			try
				set AppleScript's text item delimiters to ","
				set f to text items of (item 2 of argv)
				set AppleScript's text item delimiters to ""
				set x to (item 1 of f) as integer
				set y to (item 2 of f) as integer
				set bounds of w to {x, y, x + ((item 3 of f) as integer), y + ((item 4 of f) as integer)}
			end try
		end if
		activate
	end tell
end run"#;

/// Remet les fenêtres d'une app à leur place (Accessibilité).
/// Arguments : bundle id, puis par fenêtre : titre, x, y, largeur, hauteur.
/// Une fenêtre est retrouvée par son titre ; s'il n'y en a qu'une de chaque côté, on la prend.
/// Renvoie le nombre de fenêtres replacées.
pub const PLACE_WINDOWS: &str = r#"on run argv
	set b to item 1 of argv
	set total to ((count of argv) - 1) div 5
	set placed to 0
	tell application "System Events"
		set ps to (every application process whose bundle identifier is b)
		if (count of ps) is 0 then return "0"
		set p to item 1 of ps
		set wc to count of windows of p
		set i to 2
		repeat while (i + 4) is less than or equal to (count of argv)
			set t to item i of argv
			set target to missing value
			if t is not "" then
				try
					set target to (first window of p whose name is t)
				end try
			end if
			if target is missing value and total is 1 and wc is 1 then
				try
					set target to window 1 of p
				end try
			end if
			if target is not missing value then
				try
					set position of target to {(item (i + 1) of argv) as integer, (item (i + 2) of argv) as integer}
					set size of target to {(item (i + 3) of argv) as integer, (item (i + 4) of argv) as integer}
					set placed to placed + 1
				end try
			end if
			set i to i + 5
		end repeat
	end tell
	return placed as text
end run"#;

/// Masque toutes les apps visibles sauf celle dont le bundle id est passé en argument.
pub const HIDE_OTHERS: &str = r#"on run argv
	set keep to item 1 of argv
	tell application "System Events"
		repeat with p in (every application process whose background only is false and visible is true)
			try
				if bundle identifier of p is not keep then set visible of p to false
			end try
		end repeat
	end tell
end run"#;

/// Teste l'accès à System Events (Automatisation).
pub const PERM_AUTOMATION: &str = r#"tell application "System Events" to return (count of application processes) as text"#;

/// Teste l'accès Accessibilité (lecture des fenêtres d'une autre app).
pub const PERM_ACCESSIBILITY: &str = r#"tell application "System Events"
	set p to first application process whose frontmost is true
	return (count of windows of p) as text
end tell"#;

// ───────────────────────────── Navigateurs connus ─────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrowserFamily {
    Safari,
    Chromium,
}

pub fn browser_family(bundle_id: &str) -> Option<BrowserFamily> {
    match bundle_id {
        "com.apple.Safari" | "com.apple.SafariTechnologyPreview" => Some(BrowserFamily::Safari),
        "com.google.Chrome"
        | "com.google.Chrome.beta"
        | "com.google.Chrome.dev"
        | "com.google.Chrome.canary"
        | "org.chromium.Chromium"
        | "com.brave.Browser"
        | "com.brave.Browser.beta"
        | "com.brave.Browser.nightly"
        | "com.microsoft.edgemac"
        | "com.microsoft.edgemac.Beta"
        | "com.microsoft.edgemac.Dev"
        | "com.vivaldi.Vivaldi"
        | "company.thebrowser.Browser"
        | "com.operasoftware.Opera"
        | "com.operasoftware.OperaGX"
        | "com.operasoftware.OperaAir"
        | "com.operasoftware.OperaNext"
        | "com.operasoftware.OperaDeveloper" => Some(BrowserFamily::Chromium),
        _ => None,
    }
}

/// Les bundle ids n'ont que des lettres, chiffres, points et tirets : on vérifie quand même
/// avant de les insérer dans un script.
pub fn safe_bundle(bundle_id: &str) -> Option<&str> {
    if !bundle_id.is_empty()
        && bundle_id.len() < 200
        && bundle_id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_')
    {
        Some(bundle_id)
    } else {
        None
    }
}

pub fn tabs_script(family: BrowserFamily, bundle_id: &str) -> Option<String> {
    let b = safe_bundle(bundle_id)?;
    let tpl = match family {
        BrowserFamily::Safari => SAFARI_TABS,
        BrowserFamily::Chromium => CHROMIUM_TABS,
    };
    Some(tpl.replace("__BUNDLE__", b))
}

pub fn open_tabs_script(family: BrowserFamily, bundle_id: &str) -> Option<String> {
    let b = safe_bundle(bundle_id)?;
    let tpl = match family {
        BrowserFamily::Safari => SAFARI_OPEN,
        BrowserFamily::Chromium => CHROMIUM_OPEN,
    };
    Some(
        tpl.replace("__WAIT__", WAIT_LAUNCH)
            .replace("__READY__", READY)
            .replace("__NEWWIN__", NEW_WINDOW)
            .replace("__BOUNDS__", SET_BOUNDS)
            .replace("__RETURN__", RETURN_FAILED)
            .replace("__BUNDLE__", b),
    )
}

/// Lit la réponse des scripts d'ouverture d'onglets : les numéros (à partir de 1) des URL
/// qui n'ont pas pu être ouvertes.
pub fn parse_failed(out: &str) -> Vec<usize> {
    out.split(|c: char| c == ',' || c.is_whitespace())
        .filter_map(|s| s.trim().parse::<usize>().ok())
        .filter(|&n| n >= 1)
        .collect()
}

// ─────────────────────────────── Lecture des sorties ───────────────────────────

/// Découpe une sortie en enregistrements puis en champs.
pub fn records(out: &str) -> Vec<Vec<String>> {
    out.split(RS)
        .map(|r| r.trim_matches(|c| c == '\n' || c == '\r'))
        .filter(|r| !r.is_empty())
        .map(|r| r.split(US).map(|f| f.to_string()).collect())
        .collect()
}

#[derive(Debug, Clone, PartialEq)]
pub struct RunningApp {
    pub name: String,
    pub bundle_id: Option<String>,
    pub frontmost: bool,
}

pub fn parse_apps(out: &str) -> Vec<RunningApp> {
    records(out)
        .into_iter()
        .filter_map(|f| {
            let name = f.first()?.trim().to_string();
            if name.is_empty() {
                return None;
            }
            let bundle_id = f
                .get(1)
                .map(|b| b.trim().to_string())
                .filter(|b| !b.is_empty() && b != "missing value");
            let frontmost = f.get(2).map(|s| s.trim() == "true").unwrap_or(false);
            Some(RunningApp { name, bundle_id, frontmost })
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq)]
pub struct RawTab {
    pub window: u32,
    pub url: String,
    pub title: String,
    pub frame: Option<Frame>,
}

/// "x,y,l,h" → cadre (titre vide).
pub fn parse_frame(s: &str) -> Option<Frame> {
    let n: Vec<i32> = s.split(',').map(|p| p.trim().parse::<i32>()).collect::<Result<_, _>>().ok()?;
    if n.len() != 4 {
        return None;
    }
    let f = Frame { title: String::new(), x: n[0], y: n[1], w: n[2], h: n[3] };
    f.is_usable().then_some(f)
}

pub fn parse_tabs(out: &str) -> Vec<RawTab> {
    records(out)
        .into_iter()
        .filter_map(|f| {
            if f.len() < 2 {
                return None;
            }
            let window = f[0].trim().parse().unwrap_or(1);
            let url = f[1].trim().to_string();
            let title = f.get(2).map(|s| s.trim().to_string()).unwrap_or_default();
            if !is_reopenable_url(&url) {
                return None;
            }
            let title = if title.is_empty() { url.clone() } else { title };
            let frame = f.get(3).and_then(|s| parse_frame(s));
            Some(RawTab { window, url, title, frame })
        })
        .collect()
}

/// On ne garde que ce qu'on sait rouvrir proprement : pas les pages internes
/// (about:blank, chrome://newtab, favoris de Safari…).
pub fn is_reopenable_url(url: &str) -> bool {
    let u = url.to_ascii_lowercase();
    u.starts_with("http://") || u.starts_with("https://") || u.starts_with("file://")
}

pub fn parse_folders(out: &str) -> Vec<(String, Option<Frame>)> {
    records(out)
        .into_iter()
        .filter_map(|f| {
            let path = f.first()?.trim().to_string();
            if !path.starts_with('/') {
                return None;
            }
            let frame = f.get(1).and_then(|s| parse_frame(s));
            Some((path, frame))
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq)]
pub struct RawWindow {
    pub bundle_id: Option<String>,
    pub app_name: String,
    /// Chemin du fichier affiché dans la fenêtre, s'il y en a un.
    pub document: Option<String>,
    /// Cadre de la fenêtre (avec son titre), sauf si elle est réduite ou minuscule.
    pub frame: Option<Frame>,
}

/// Renvoie les fenêtres trouvées, et `true` si l'accès Accessibilité manque.
pub fn parse_windows(out: &str) -> (Vec<RawWindow>, bool) {
    if out.trim_start().starts_with("!AX") {
        return (Vec::new(), true);
    }
    let wins = records(out)
        .into_iter()
        .filter_map(|f| {
            if f.len() < 3 {
                return None;
            }
            let bundle_id = Some(f[0].trim().to_string()).filter(|b| !b.is_empty());
            let app_name = f[1].trim().to_string();
            let document = file_url_to_path(f[2].trim());
            let title = f.get(3).map(|t| t.trim().to_string()).unwrap_or_default();
            let num = |i: usize| f.get(i).and_then(|v| v.trim().parse::<i32>().ok());
            let minimized = f.get(8).map(|m| m.trim() == "true").unwrap_or(false);
            let frame = match (num(4), num(5), num(6), num(7)) {
                (Some(x), Some(y), Some(w), Some(h)) if !minimized => {
                    Some(Frame { title, x, y, w, h }).filter(|fr| fr.is_usable())
                }
                _ => None,
            };
            Some(RawWindow { bundle_id, app_name, document, frame })
        })
        .collect();
    (wins, false)
}

/// `file:///Users/moi/Mon%20Doc.txt` → `/Users/moi/Mon Doc.txt`.
pub fn file_url_to_path(s: &str) -> Option<String> {
    let rest = if let Some(r) = s.strip_prefix("file://localhost") {
        r
    } else if let Some(r) = s.strip_prefix("file://") {
        r
    } else if s.starts_with('/') {
        return Some(s.trim_end_matches('/').to_string()).filter(|p| !p.is_empty());
    } else {
        return None;
    };
    let decoded = percent_decode(rest);
    let trimmed = if decoded.len() > 1 { decoded.trim_end_matches('/').to_string() } else { decoded };
    if trimmed.starts_with('/') {
        Some(trimmed)
    } else {
        None
    }
}

pub fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && bytes[i + 1].is_ascii_hexdigit()
            && bytes[i + 2].is_ascii_hexdigit()
        {
            let hex = [bytes[i + 1], bytes[i + 2]];
            // Les deux octets sont des chiffres hexadécimaux ASCII : conversion sûre.
            if let Ok(v) = u8::from_str_radix(std::str::from_utf8(&hex).unwrap_or("zz"), 16) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Dernier élément d'un chemin, pour l'affichage.
pub fn file_name(path: &str) -> String {
    let p = path.trim_end_matches('/');
    if p.is_empty() {
        return "/".into();
    }
    p.rsplit('/').next().unwrap_or(p).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec(fields: &[&str]) -> String {
        let mut s = fields.join(&US.to_string());
        s.push(RS);
        s
    }

    #[test]
    fn apps_are_parsed() {
        let out = format!(
            "{}{}{}",
            rec(&["Safari", "com.apple.Safari", "false"]),
            rec(&["Visual Studio Code", "com.microsoft.VSCode", "true"]),
            rec(&["Truc", "", "false"])
        );
        let apps = parse_apps(&out);
        assert_eq!(apps.len(), 3);
        assert!(apps[1].frontmost);
        assert_eq!(apps[1].bundle_id.as_deref(), Some("com.microsoft.VSCode"));
        assert_eq!(apps[2].bundle_id, None);
    }

    #[test]
    fn tabs_keep_titles_with_commas_and_skip_internal_pages() {
        let out = format!(
            "{}{}{}\n",
            rec(&["1", "https://example.com/a?b=1,2", "Titre, avec virgule", "0,25,1280,800"]),
            rec(&["1", "about:blank", "Nouvel onglet"]),
            rec(&["2", "https://rust-lang.org", ""])
        );
        let tabs = parse_tabs(&out);
        assert_eq!(tabs.len(), 2);
        assert_eq!(tabs[0].title, "Titre, avec virgule");
        assert_eq!(tabs[0].frame.as_ref().map(|f| (f.y, f.w)), Some((25, 1280)));
        assert!(tabs[1].frame.is_none());
        assert_eq!(tabs[1].window, 2);
        assert_eq!(tabs[1].title, "https://rust-lang.org");
    }

    #[test]
    fn windows_and_ax_flag() {
        let (w, denied) = parse_windows("!AX");
        assert!(w.is_empty() && denied);
        let out = format!(
            "{}{}{}",
            rec(&["com.apple.Preview", "Aperçu", "file:///Users/moi/Mes%20cours/chap%C3%AEtre%201.pdf", "chapître 1.pdf", "40", "60", "900", "700", "false"]),
            rec(&["com.apple.Notes", "Notes", "", "Notes", "-1200", "30", "600", "500", "false"]),
            rec(&["com.apple.TextEdit", "TextEdit", "", "Sans titre", "0", "0", "500", "400", "true"])
        );
        let (w, denied) = parse_windows(&out);
        assert!(!denied);
        assert_eq!(w.len(), 3);
        assert_eq!(w[0].document.as_deref(), Some("/Users/moi/Mes cours/chapître 1.pdf"));
        assert_eq!(w[0].frame.as_ref().unwrap().title, "chapître 1.pdf");
        assert_eq!(w[1].document, None);
        assert_eq!(w[1].frame.as_ref().unwrap().x, -1200);
        assert!(w[2].frame.is_none(), "une fenêtre réduite n'a pas de cadre");
    }

    #[test]
    fn file_urls() {
        assert_eq!(file_url_to_path("file:///Users/a/b/").as_deref(), Some("/Users/a/b"));
        assert_eq!(file_url_to_path("file://localhost/tmp/x").as_deref(), Some("/tmp/x"));
        assert_eq!(file_url_to_path("/Users/a").as_deref(), Some("/Users/a"));
        assert_eq!(file_url_to_path("https://x"), None);
        assert_eq!(percent_decode("100%"), "100%");
        assert_eq!(percent_decode("a%2"), "a%2");
        assert_eq!(file_name("/Users/a/Projet/"), "Projet");
    }

    #[test]
    fn folders() {
        let out = format!("/Users/moi/Projets/{US}10,20,800,500{RS}/Applications/{RS}");
        let f = parse_folders(&out);
        assert_eq!(f[0].0, "/Users/moi/Projets/");
        assert_eq!(f[0].1.as_ref().unwrap().w, 800);
        assert_eq!(f[1], ("/Applications/".to_string(), None));
    }

    #[test]
    fn errors_are_classified() {
        assert_eq!(
            ScriptError::classify("execution error: Not authorized to send Apple events to Safari. (-1743)"),
            ScriptError::NotAuthorized
        );
        assert_eq!(
            ScriptError::classify("System Events got an error: osascript is not allowed assistive access. (-1719)"),
            ScriptError::Accessibility
        );
    }

    #[test]
    fn bundle_ids_are_sanitized() {
        assert!(safe_bundle("com.apple.Safari").is_some());
        assert!(safe_bundle("evil\" & do shell script \"rm").is_none());
        assert!(tabs_script(BrowserFamily::Safari, "com.apple.Safari").unwrap().contains("\"com.apple.Safari\""));
        assert_eq!(browser_family("com.operasoftware.OperaGX"), Some(BrowserFamily::Chromium));
        // Jamais de « réessayer d'ouvrir une fenêtre » : un seul ordre, puis on compte.
        for f in [BrowserFamily::Safari, BrowserFamily::Chromium] {
            let src = open_tabs_script(f, "com.google.Chrome").unwrap();
            assert_eq!(src.matches("make new window").count() + src.matches("make new document").count(), 1);
            assert!(!src.contains("__"), "tous les morceaux sont remplacés");
        }
        assert!(!open_tabs_script(BrowserFamily::Chromium, "com.google.Chrome").unwrap().contains("__BUNDLE__"));
    }
}
