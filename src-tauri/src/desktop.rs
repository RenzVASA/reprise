//! Ce qui est commun à Windows et Linux : construire la photo à partir de ce que le système
//! nous a appris (apps, fichiers passés aux apps, dossiers), et remettre tout en place.
//!
//! Chaque système fournit un `Launcher` (comment lancer une app ou ouvrir un fichier) ;
//! ici on ne fait que décider quoi lancer, dans quel ordre.

use crate::model::{new_id, now_ms, Item, ItemKind, RestoreReport, Snapshot};
use crate::restore::Action;
use crate::sessions::{self, Kind};
use crate::sys;
use std::collections::HashSet;
use std::path::Path;
use std::time::Duration;

/// Une app trouvée sur le système.
#[derive(Debug, Clone, PartialEq)]
pub struct FoundApp {
    /// Nom affiché.
    pub name: String,
    /// Identité stable : chemin du programme (Windows) ou nom du raccourci d'app (Linux).
    pub id: String,
    pub frontmost: bool,
    /// Fichiers ou dossiers que l'app a reçus au lancement (« Exposé.docx », un dossier de projet…).
    pub files: Vec<String>,
}

fn push_warning(w: String, warnings: &mut Vec<String>) {
    if !warnings.contains(&w) {
        warnings.push(w);
    }
}

/// Assemble la photo : apps, onglets (lus dans les sessions des navigateurs), dossiers, fichiers.
pub fn build_snapshot(apps: &[FoundApp], folders: &[String], ignored: &[String], mut warnings: Vec<String>) -> Snapshot {
    let is_ignored = |id: &str| ignored.iter().any(|i| i.eq_ignore_ascii_case(id));
    let mut items: Vec<Item> = Vec::new();
    let mut seen_apps: HashSet<String> = HashSet::new();
    let mut seen_paths: HashSet<String> = HashSet::new();
    let mut seen_browsers: HashSet<&'static str> = HashSet::new();

    for a in apps {
        if is_ignored(&a.id) || !seen_apps.insert(a.id.to_ascii_lowercase()) {
            continue;
        }
        items.push(Item {
            id: new_id(),
            kind: ItemKind::App,
            app_name: a.name.clone(),
            bundle_id: Some(a.id.clone()),
            label: a.name.clone(),
            value: a.id.clone(),
            group: None,
            frames: Vec::new(),
        });

        // Onglets : une seule lecture par navigateur, même s'il a plusieurs fenêtres.
        if let Some(b) = sessions::browser_for(&a.id) {
            if seen_browsers.insert(b.name) {
                match sessions::read_tabs(b) {
                    Ok(tabs) => {
                        for t in tabs {
                            items.push(Item {
                                id: new_id(),
                                kind: ItemKind::Tab,
                                app_name: a.name.clone(),
                                bundle_id: Some(a.id.clone()),
                                label: t.title,
                                value: t.url,
                                group: Some(t.window),
                                frames: Vec::new(),
                            });
                        }
                    }
                    Err(e) => push_warning(e, &mut warnings),
                }
            }
            continue; // les « fichiers » d'un navigateur sont des pages, pas des documents
        }

        for f in &a.files {
            if seen_paths.insert(f.to_ascii_lowercase()) {
                items.push(Item {
                    id: new_id(),
                    kind: ItemKind::Document,
                    app_name: a.name.clone(),
                    bundle_id: Some(a.id.clone()),
                    label: sys::file_name(f),
                    value: f.clone(),
                    group: None,
                    frames: Vec::new(),
                });
            }
        }
    }

    for f in folders {
        if seen_paths.insert(f.to_ascii_lowercase()) {
            items.push(Item {
                id: new_id(),
                kind: ItemKind::Folder,
                app_name: if cfg!(windows) { "Explorateur".into() } else { "Fichiers".into() },
                bundle_id: None,
                label: sys::file_name(f),
                value: f.clone(),
                group: None,
                frames: Vec::new(),
            });
        }
    }

    Snapshot {
        front_app: apps.iter().find(|a| a.frontmost).map(|a| a.name.clone()),
        items,
        warnings,
        taken_at: now_ms(),
    }
}

/// Comment ce système lance les choses.
pub trait Launcher {
    /// Lance l'app `id` avec ces arguments.
    fn launch(&self, id: &str, args: &[String]) -> Result<(), String>;
    /// Ouvre un fichier, un dossier ou une adresse avec l'app par défaut.
    fn open(&self, target: &str) -> Result<(), String>;
}

/// Les arguments pour ouvrir une fenêtre de navigateur avec ses onglets. Firefox veut deux
/// appels : la nouvelle fenêtre, puis les autres onglets dedans.
pub fn tab_commands(kind: Kind, urls: &[String]) -> Vec<Vec<String>> {
    if urls.is_empty() {
        return Vec::new();
    }
    match kind {
        Kind::Chromium => {
            let mut a = vec!["--new-window".to_string()];
            a.extend(urls.iter().cloned());
            vec![a]
        }
        Kind::Firefox => {
            let mut calls = vec![vec!["-new-window".to_string(), urls[0].clone()]];
            if urls.len() > 1 {
                calls.push(urls[1..].iter().flat_map(|u| ["-new-tab".to_string(), u.clone()]).collect());
            }
            calls
        }
    }
}

/// Exécute le plan de reprise. Rien ne s'arrête sur une erreur : on note et on continue.
/// « Faire place nette » et la position des fenêtres n'existent que sur Mac pour l'instant.
pub fn execute(l: &dyn Launcher, actions: &[Action]) -> RestoreReport {
    let mut report = RestoreReport::default();
    for a in actions {
        match a {
            Action::HideOthers | Action::PlaceWindows { .. } => {}
            Action::OpenApp { name, bundle_id } => {
                let res = match bundle_id.as_deref() {
                    Some(id) => l.launch(id, &[]),
                    None => Err("inconnue".into()),
                };
                match res {
                    Ok(()) => report.opened += 1,
                    Err(_) => report.failures.push(format!("Impossible d'ouvrir {name}.")),
                }
            }
            Action::OpenFolder { path, .. } => {
                if !Path::new(path).exists() {
                    report.failures.push(format!("Dossier introuvable : {}", sys::file_name(path)));
                    continue;
                }
                match l.open(path) {
                    Ok(()) => report.opened += 1,
                    Err(_) => report.failures.push(format!("Impossible d'ouvrir le dossier {}.", sys::file_name(path))),
                }
            }
            Action::OpenDocument { path, bundle_id } => {
                if !Path::new(path).exists() {
                    report.failures.push(format!("Fichier introuvable : {}", sys::file_name(path)));
                    continue;
                }
                let res = match bundle_id.as_deref() {
                    Some(id) => l.launch(id, &[path.clone()]).or_else(|_| l.open(path)),
                    None => l.open(path),
                };
                match res {
                    Ok(()) => report.opened += 1,
                    Err(_) => report.failures.push(format!("Impossible d'ouvrir {}.", sys::file_name(path))),
                }
            }
            Action::OpenTabs { browser, bundle_id, urls, .. } => {
                if urls.is_empty() {
                    continue;
                }
                let kind = sessions::browser_for(bundle_id).map(|b| b.kind).unwrap_or(Kind::Chromium);
                let mut ok = true;
                for (n, args) in tab_commands(kind, urls).iter().enumerate() {
                    if n > 0 {
                        // Laisser Firefox ouvrir sa fenêtre avant d'y ajouter les onglets.
                        std::thread::sleep(Duration::from_millis(900));
                    }
                    ok &= l.launch(bundle_id, args).is_ok();
                }
                // Plan B : chaque page avec le navigateur par défaut.
                if !ok {
                    ok = urls.iter().all(|u| l.open(u).is_ok());
                }
                if ok {
                    report.opened += urls.len() as u32;
                } else {
                    report.failures.push(format!("Les onglets de {browser} n'ont pas pu être rouverts."));
                }
            }
        }
        // Un petit temps entre deux lancements : certaines apps n'aiment pas être pressées.
        std::thread::sleep(Duration::from_millis(120));
    }
    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    #[derive(Default)]
    struct Fake {
        calls: RefCell<Vec<String>>,
    }
    impl Launcher for Fake {
        fn launch(&self, id: &str, args: &[String]) -> Result<(), String> {
            self.calls.borrow_mut().push(format!("launch {id} {}", args.join(" ")));
            Ok(())
        }
        fn open(&self, t: &str) -> Result<(), String> {
            self.calls.borrow_mut().push(format!("open {t}"));
            Ok(())
        }
    }

    #[test]
    fn tabs_per_browser_family() {
        let urls: Vec<String> = vec!["https://a.fr".into(), "https://b.fr".into(), "https://c.fr".into()];
        assert_eq!(tab_commands(Kind::Chromium, &urls), vec![vec!["--new-window", "https://a.fr", "https://b.fr", "https://c.fr"]]);
        assert_eq!(
            tab_commands(Kind::Firefox, &urls),
            vec![vec!["-new-window", "https://a.fr"], vec!["-new-tab", "https://b.fr", "-new-tab", "https://c.fr"]]
        );
        assert!(tab_commands(Kind::Firefox, &[]).is_empty());
    }

    #[test]
    fn executes_a_plan() {
        let f = Fake::default();
        let tmp = std::env::temp_dir();
        let actions = vec![
            Action::HideOthers,
            Action::OpenApp { name: "Bloc-notes".into(), bundle_id: Some("C:/Windows/notepad.exe".into()) },
            Action::OpenFolder { path: tmp.to_string_lossy().into_owned(), frame: None },
            Action::OpenFolder { path: "/n/existe/pas".into(), frame: None },
            Action::OpenTabs {
                browser: "Chrome".into(),
                bundle_id: "C:/Program Files/Google/Chrome/Application/chrome.exe".into(),
                family: None,
                urls: vec!["https://a.fr".into(), "https://b.fr".into()],
                frame: None,
            },
        ];
        let r = execute(&f, &actions);
        assert_eq!(r.opened, 4);
        assert_eq!(r.failures.len(), 1);
        let calls = f.calls.borrow();
        assert_eq!(calls[0], "launch C:/Windows/notepad.exe ");
        assert!(calls[1].starts_with("open "));
        assert_eq!(calls[2], "launch C:/Program Files/Google/Chrome/Application/chrome.exe --new-window https://a.fr https://b.fr");
    }

    #[test]
    fn snapshot_from_found_apps() {
        let apps = vec![
            FoundApp { name: "Bloc-notes".into(), id: r"C:\Windows\notepad.exe".into(), frontmost: true, files: vec![r"C:\Users\moi\note.txt".into()] },
            FoundApp { name: "Bloc-notes".into(), id: r"C:\WINDOWS\notepad.exe".into(), frontmost: false, files: vec![] },
            FoundApp { name: "Spotify".into(), id: r"C:\Spotify\Spotify.exe".into(), frontmost: false, files: vec![] },
        ];
        let s = build_snapshot(&apps, &[r"C:\Users\moi\Documents".into()], &[r"c:\spotify\spotify.exe".into()], vec![]);
        let kinds: Vec<_> = s.items.iter().map(|i| (i.kind, i.label.as_str())).collect();
        assert_eq!(kinds, vec![(ItemKind::App, "Bloc-notes"), (ItemKind::Document, "note.txt"), (ItemKind::Folder, "Documents")]);
        assert_eq!(s.front_app.as_deref(), Some("Bloc-notes"));
    }
}
