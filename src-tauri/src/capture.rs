//! Prendre la photo du contexte de travail.

use crate::model::{new_id, now_ms, Item, ItemKind, Snapshot};
use crate::script::{self, BrowserFamily, RunningApp, Runner, ScriptError};
use std::collections::HashSet;

/// Apps qu'on ne remet jamais dans un contexte : elles sont toujours là, ou ce sont des
/// morceaux du système.
const IGNORED_APPS: &[&str] = &[
    "com.apple.finder", // ses fenêtres sont capturées comme dossiers
    "com.apple.dock",
    "com.apple.loginwindow",
    "com.apple.controlcenter",
    "com.apple.notificationcenterui",
    "com.apple.Spotlight",
];

/// Première étape, rapide (~0,2 s) : la liste des apps et celle au premier plan.
pub fn running_apps(r: &dyn Runner) -> Result<Vec<RunningApp>, ScriptError> {
    r.run(script::APPS, &[]).map(|out| script::parse_apps(&out))
}

pub fn frontmost(apps: &[RunningApp], own_bundle: &str) -> Option<String> {
    apps.iter()
        .find(|a| a.frontmost && a.bundle_id.as_deref() != Some(own_bundle))
        .map(|a| a.name.clone())
}

fn automation_warning(app: &str) -> String {
    format!(
        "Reprise n'a pas le droit de piloter {app}. Autorise-le dans Réglages Système › Confidentialité et sécurité › Automatisation."
    )
}

const AX_WARNING: &str = "Les fichiers ouverts n'ont pas été capturés : autorise Reprise dans Réglages Système › Confidentialité et sécurité › Accessibilité.";

fn push_warning(w: String, warnings: &mut Vec<String>) {
    if !warnings.contains(&w) {
        warnings.push(w);
    }
}

/// Deuxième étape : onglets, dossiers du Finder, fichiers ouverts.
/// Chaque morceau est indépendant : si Safari refuse, on garde quand même le reste.
/// `ignored` : bundle ids des apps que l'utilisateur ne veut jamais enregistrer
/// (ni l'app, ni ses onglets, ni ses fichiers).
pub fn full_capture(r: &dyn Runner, apps: &[RunningApp], own_bundle: &str, ignored: &[String]) -> Snapshot {
    let is_ignored = |bid: &str| ignored.iter().any(|i| i == bid);
    let mut items: Vec<Item> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();

    // 1. Les applications.
    for a in apps {
        let bid = a.bundle_id.as_deref().unwrap_or("");
        if bid == own_bundle || IGNORED_APPS.contains(&bid) || is_ignored(bid) {
            continue;
        }
        items.push(Item {
            id: new_id(),
            kind: ItemKind::App,
            app_name: a.name.clone(),
            bundle_id: a.bundle_id.clone(),
            label: a.name.clone(),
            value: a.bundle_id.clone().unwrap_or_else(|| a.name.clone()),
            group: None,
        });
    }

    // 2. Les onglets des navigateurs lancés.
    for a in apps {
        let Some(bid) = a.bundle_id.as_deref() else { continue };
        if is_ignored(bid) {
            continue;
        }
        let Some(family) = script::browser_family(bid) else { continue };
        let Some(src) = script::tabs_script(family, bid) else { continue };
        match r.run(&src, &[]) {
            Ok(out) => {
                for t in script::parse_tabs(&out) {
                    items.push(Item {
                        id: new_id(),
                        kind: ItemKind::Tab,
                        app_name: a.name.clone(),
                        bundle_id: Some(bid.to_string()),
                        label: t.title,
                        value: t.url,
                        group: Some(t.window),
                    });
                }
            }
            Err(ScriptError::NotAuthorized) => push_warning(automation_warning(&a.name), &mut warnings),
            Err(ScriptError::NotRunning) => {}
            Err(ScriptError::Timeout) => push_warning(
                format!("{} n'a pas répondu à temps, ses onglets n'ont pas été capturés.", a.name),
                &mut warnings,
            ),
            Err(_) => push_warning(format!("Les onglets de {} n'ont pas pu être lus.", a.name), &mut warnings),
        }
    }

    // 3. Les dossiers ouverts dans le Finder.
    let mut seen_paths: HashSet<String> = HashSet::new();
    if apps.iter().any(|a| a.bundle_id.as_deref() == Some("com.apple.finder")) {
        match r.run(script::FINDER_FOLDERS, &[]) {
            Ok(out) => {
                for p in script::parse_folders(&out) {
                    let path = if p.len() > 1 { p.trim_end_matches('/').to_string() } else { p };
                    if seen_paths.insert(path.clone()) {
                        items.push(Item {
                            id: new_id(),
                            kind: ItemKind::Folder,
                            app_name: "Finder".into(),
                            bundle_id: Some("com.apple.finder".into()),
                            label: script::file_name(&path),
                            value: path,
                            group: None,
                        });
                    }
                }
            }
            Err(ScriptError::NotAuthorized) => push_warning(automation_warning("le Finder"), &mut warnings),
            Err(_) => {}
        }
    }

    // 4. Les fichiers ouverts (via l'Accessibilité).
    match r.run(script::DOCUMENTS, &[]) {
        Ok(out) => {
            let (docs, ax_denied) = script::parse_documents(&out);
            if ax_denied {
                push_warning(AX_WARNING.into(), &mut warnings);
            }
            for d in docs {
                if d.bundle_id.as_deref() == Some(own_bundle) || d.bundle_id.as_deref().is_some_and(is_ignored) {
                    continue;
                }
                if seen_paths.insert(d.path.clone()) {
                    items.push(Item {
                        id: new_id(),
                        kind: ItemKind::Document,
                        app_name: d.app_name,
                        bundle_id: d.bundle_id,
                        label: script::file_name(&d.path),
                        value: d.path,
                        group: None,
                    });
                }
            }
        }
        Err(ScriptError::Accessibility) => push_warning(AX_WARNING.into(), &mut warnings),
        Err(ScriptError::NotAuthorized) => push_warning(automation_warning("System Events"), &mut warnings),
        Err(_) => {}
    }

    Snapshot {
        front_app: frontmost(apps, own_bundle),
        items,
        warnings,
        taken_at: now_ms(),
    }
}

/// Capture complète en une fois (utilisée par « Mettre à jour avec l'état actuel »).
pub fn capture_all(r: &dyn Runner, own_bundle: &str, ignored: &[String]) -> Result<Snapshot, ScriptError> {
    let apps = running_apps(r)?;
    Ok(full_capture(r, &apps, own_bundle, ignored))
}

#[allow(dead_code)]
pub fn is_browser(bundle_id: &str) -> Option<BrowserFamily> {
    script::browser_family(bundle_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::script::{RS, US};

    struct Fake;
    impl Runner for Fake {
        fn run(&self, src: &str, _args: &[String]) -> Result<String, ScriptError> {
            let r = |f: &[&str]| format!("{}{}", f.join(&US.to_string()), RS);
            if src == script::APPS {
                Ok([
                    r(&["Safari", "com.apple.Safari", "false"]),
                    r(&["Google Chrome", "com.google.Chrome", "false"]),
                    r(&["Finder", "com.apple.finder", "false"]),
                    r(&["Code", "com.microsoft.VSCode", "true"]),
                    r(&["Reprise", "io.github.renzvasa.reprise", "false"]),
                ]
                .concat())
            } else if src.contains("com.apple.Safari") {
                Ok([r(&["1", "https://a.fr", "A"]), r(&["2", "https://b.fr", "B"])].concat())
            } else if src.contains("com.google.Chrome") {
                Err(ScriptError::NotAuthorized)
            } else if src == script::FINDER_FOLDERS {
                Ok(format!("/Users/moi/Projet/{RS}/Users/moi/Projet/{RS}"))
            } else if src == script::DOCUMENTS {
                Ok([
                    r(&["com.microsoft.VSCode", "Code", "file:///Users/moi/Projet/main.rs"]),
                    r(&["com.apple.finder", "Finder", "file:///Users/moi/Projet/"]),
                ]
                .concat())
            } else {
                Err(ScriptError::Other("?".into()))
            }
        }
    }

    #[test]
    fn full_capture_assembles_everything() {
        let own = "io.github.renzvasa.reprise";
        let snap = capture_all(&Fake, own, &[]).unwrap();
        assert_eq!(snap.front_app.as_deref(), Some("Code"));
        let apps: Vec<_> = snap.items.iter().filter(|i| i.kind == ItemKind::App).map(|i| i.label.as_str()).collect();
        assert_eq!(apps, vec!["Safari", "Google Chrome", "Code"]);
        assert_eq!(snap.items.iter().filter(|i| i.kind == ItemKind::Tab).count(), 2);
        // Le dossier n'apparaît qu'une fois, même s'il est aussi vu comme « document ».
        assert_eq!(snap.items.iter().filter(|i| i.kind == ItemKind::Folder).count(), 1);
        assert_eq!(snap.items.iter().filter(|i| i.kind == ItemKind::Document).count(), 1);
        assert_eq!(snap.warnings.len(), 1);
        assert!(snap.warnings[0].contains("Google Chrome"));
    }

    #[test]
    fn ignored_apps_leave_no_trace() {
        let own = "io.github.renzvasa.reprise";
        let ignored = vec!["com.apple.Safari".to_string(), "com.microsoft.VSCode".to_string()];
        let snap = capture_all(&Fake, own, &ignored).unwrap();
        assert!(!snap.items.iter().any(|i| i.bundle_id.as_deref() == Some("com.apple.Safari")));
        assert!(!snap.items.iter().any(|i| i.bundle_id.as_deref() == Some("com.microsoft.VSCode")));
        assert_eq!(snap.items.iter().filter(|i| i.kind == ItemKind::Folder).count(), 1);
    }
}
