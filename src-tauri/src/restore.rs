//! Remettre un contexte en place.

use crate::model::{Context, ItemKind, RestoreReport};
use crate::script::{self, BrowserFamily, Runner};
use std::collections::HashSet;
use std::path::Path;

#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    /// Masquer les autres apps avant de tout rouvrir (« faire place nette »).
    HideOthers,
    OpenApp { name: String, bundle_id: Option<String> },
    OpenFolder { path: String },
    OpenDocument { path: String, bundle_id: Option<String> },
    /// Une fenêtre de navigateur avec ses onglets, dans l'ordre.
    OpenTabs { browser: String, bundle_id: String, family: Option<BrowserFamily>, urls: Vec<String> },
}

/// Construit la liste des actions. `selected` = ids des éléments cochés (tout si `None`).
pub fn plan(ctx: &Context, selected: Option<&HashSet<String>>, tidy: bool) -> Vec<Action> {
    let picked = |id: &str| selected.map(|s| s.contains(id)).unwrap_or(true);
    let mut actions = Vec::new();
    if tidy {
        actions.push(Action::HideOthers);
    }

    // Fenêtres de navigateur : (bundle, n° de fenêtre) → URL, dans l'ordre d'apparition.
    let mut windows: Vec<(String, String, u32, Vec<String>)> = Vec::new();
    for it in ctx.items.iter().filter(|i| i.kind == ItemKind::Tab && picked(&i.id)) {
        let Some(bid) = it.bundle_id.clone() else { continue };
        let g = it.group.unwrap_or(1);
        match windows.iter_mut().find(|w| w.0 == bid && w.2 == g) {
            Some(w) => w.3.push(it.value.clone()),
            None => windows.push((bid, it.app_name.clone(), g, vec![it.value.clone()])),
        }
    }
    let browsers_with_tabs: HashSet<&str> = windows.iter().map(|w| w.0.as_str()).collect();

    // Les apps d'abord (sauf les navigateurs qui vont de toute façon s'ouvrir avec leurs onglets).
    for it in ctx.items.iter().filter(|i| i.kind == ItemKind::App && picked(&i.id)) {
        if let Some(b) = it.bundle_id.as_deref() {
            if browsers_with_tabs.contains(b) {
                continue;
            }
        }
        actions.push(Action::OpenApp { name: it.label.clone(), bundle_id: it.bundle_id.clone() });
    }
    for it in ctx.items.iter().filter(|i| i.kind == ItemKind::Folder && picked(&i.id)) {
        actions.push(Action::OpenFolder { path: it.value.clone() });
    }
    for it in ctx.items.iter().filter(|i| i.kind == ItemKind::Document && picked(&i.id)) {
        actions.push(Action::OpenDocument { path: it.value.clone(), bundle_id: it.bundle_id.clone() });
    }
    for (bid, name, _g, urls) in windows {
        let family = script::browser_family(&bid);
        actions.push(Action::OpenTabs { browser: name, bundle_id: bid, family, urls });
    }
    actions
}

/// Exécute le plan. Rien ne s'arrête sur une erreur : on note ce qui a raté et on continue.
pub fn execute(r: &dyn Runner, actions: &[Action], own_bundle: &str) -> RestoreReport {
    let mut report = RestoreReport::default();
    for a in actions {
        match a {
            Action::HideOthers => {
                let _ = r.run(script::HIDE_OTHERS, &[own_bundle.to_string()]);
            }
            Action::OpenApp { name, bundle_id } => {
                let res = match bundle_id.as_deref() {
                    Some(b) => script::open(&["-b", b]).or_else(|_| script::open(&["-a", name])),
                    None => script::open(&["-a", name]),
                };
                match res {
                    Ok(()) => report.opened += 1,
                    Err(_) => report.failures.push(format!("Impossible d'ouvrir {name}.")),
                }
            }
            Action::OpenFolder { path } => {
                if !Path::new(path).exists() {
                    report.failures.push(format!("Dossier introuvable : {}", script::file_name(path)));
                    continue;
                }
                match script::open(&[path]) {
                    Ok(()) => report.opened += 1,
                    Err(_) => report.failures.push(format!("Impossible d'ouvrir le dossier {}.", script::file_name(path))),
                }
            }
            Action::OpenDocument { path, bundle_id } => {
                if !Path::new(path).exists() {
                    report.failures.push(format!("Fichier introuvable : {}", script::file_name(path)));
                    continue;
                }
                let res = match bundle_id.as_deref() {
                    Some(b) => script::open(&["-b", b, path]).or_else(|_| script::open(&[path])),
                    None => script::open(&[path]),
                };
                match res {
                    Ok(()) => report.opened += 1,
                    Err(_) => report.failures.push(format!("Impossible d'ouvrir {}.", script::file_name(path))),
                }
            }
            Action::OpenTabs { browser, bundle_id, family, urls } => {
                if urls.is_empty() {
                    continue;
                }
                // 1er essai : recréer la fenêtre avec AppleScript (garde le regroupement).
                let scripted = family
                    .and_then(|f| script::open_tabs_script(f, bundle_id))
                    .map(|src| r.run(&src, urls).is_ok())
                    .unwrap_or(false);
                // Sinon : `open -b navigateur url1 url2 …` (marche avec presque tout, Firefox compris).
                let ok = scripted || {
                    let mut args: Vec<&str> = vec!["-b", bundle_id.as_str()];
                    args.extend(urls.iter().map(|u| u.as_str()));
                    script::open(&args).is_ok()
                };
                if ok {
                    report.opened += urls.len() as u32;
                } else {
                    report.failures.push(format!("Les onglets de {browser} n'ont pas pu être rouverts."));
                }
            }
        }
    }
    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Item;

    fn item(id: &str, kind: ItemKind, bundle: &str, value: &str, group: Option<u32>) -> Item {
        Item {
            id: id.into(),
            kind,
            app_name: bundle.into(),
            bundle_id: Some(bundle.into()),
            label: value.into(),
            value: value.into(),
            group,
        }
    }

    fn ctx() -> Context {
        Context {
            id: "c".into(),
            name: "Test".into(),
            note: String::new(),
            created_at: 0,
            updated_at: 0,
            last_restored_at: None,
            restore_count: 0,
            pinned: false,
            front_app: None,
            items: vec![
                item("a1", ItemKind::App, "com.apple.Safari", "com.apple.Safari", None),
                item("a2", ItemKind::App, "com.microsoft.VSCode", "com.microsoft.VSCode", None),
                item("t1", ItemKind::Tab, "com.apple.Safari", "https://a.fr", Some(1)),
                item("t2", ItemKind::Tab, "com.apple.Safari", "https://b.fr", Some(2)),
                item("t3", ItemKind::Tab, "com.apple.Safari", "https://c.fr", Some(1)),
                item("f1", ItemKind::Folder, "com.apple.finder", "/Users/moi", None),
            ],
        }
    }

    #[test]
    fn tabs_are_grouped_by_window_and_browser_app_is_not_opened_twice() {
        let p = plan(&ctx(), None, true);
        assert_eq!(p[0], Action::HideOthers);
        assert!(!p.iter().any(|a| matches!(a, Action::OpenApp { bundle_id: Some(b), .. } if b == "com.apple.Safari")));
        let tabs: Vec<_> = p
            .iter()
            .filter_map(|a| match a {
                Action::OpenTabs { urls, .. } => Some(urls.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(tabs, vec![vec!["https://a.fr".to_string(), "https://c.fr".into()], vec!["https://b.fr".into()]]);
    }

    #[test]
    fn selection_is_respected() {
        let sel: HashSet<String> = ["a1", "t2"].iter().map(|s| s.to_string()).collect();
        let p = plan(&ctx(), Some(&sel), false);
        assert_eq!(p.len(), 1);
        assert!(matches!(&p[0], Action::OpenTabs { urls, .. } if urls == &vec!["https://b.fr".to_string()]));
    }
}
