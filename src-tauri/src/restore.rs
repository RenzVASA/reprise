//! Remettre un contexte en place.

use crate::model::{on_screen, Context, Frame, ItemKind, RestoreReport, Screen};
use crate::script::{self, BrowserFamily, Runner};
use std::collections::HashSet;
use std::path::Path;
use std::time::Duration;

#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    /// Masquer les autres apps avant de tout rouvrir (« faire place nette »).
    HideOthers,
    OpenApp { name: String, bundle_id: Option<String> },
    OpenFolder { path: String, frame: Option<Frame> },
    OpenDocument { path: String, bundle_id: Option<String> },
    /// Une fenêtre de navigateur avec ses onglets, dans l'ordre, et sa place à l'écran.
    OpenTabs { browser: String, bundle_id: String, family: Option<BrowserFamily>, urls: Vec<String>, frame: Option<Frame> },
    /// Remettre les fenêtres d'une app à leur place, une fois qu'elles sont ouvertes.
    PlaceWindows { bundle_id: String, frames: Vec<Frame> },
}

/// Construit la liste des actions. `selected` = ids des éléments cochés (tout si `None`).
/// `screens` : les écrans branchés ; une fenêtre qui tomberait hors écran n'est pas déplacée.
pub fn plan(ctx: &Context, selected: Option<&HashSet<String>>, tidy: bool, screens: &[Screen]) -> Vec<Action> {
    let keep = |f: &Frame| f.is_usable() && on_screen(f, screens);
    let picked = |id: &str| selected.map(|s| s.contains(id)).unwrap_or(true);
    let mut actions = Vec::new();
    if tidy {
        actions.push(Action::HideOthers);
    }

    // Fenêtres de navigateur : (bundle, n° de fenêtre) → URL, dans l'ordre d'apparition.
    let mut windows: Vec<(String, String, u32, Vec<String>, Option<Frame>)> = Vec::new();
    for it in ctx.items.iter().filter(|i| i.kind == ItemKind::Tab && picked(&i.id) && reopenable(&i.value)) {
        let Some(bid) = it.bundle_id.clone() else { continue };
        let g = it.group.unwrap_or(1);
        let frame = it.frames.first().filter(|f| keep(f)).cloned();
        match windows.iter_mut().find(|w| w.0 == bid && w.2 == g) {
            Some(w) => {
                w.3.push(it.value.clone());
                if w.4.is_none() {
                    w.4 = frame;
                }
            }
            None => windows.push((bid, it.app_name.clone(), g, vec![it.value.clone()], frame)),
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
        actions.push(Action::OpenFolder { path: it.value.clone(), frame: it.frames.first().filter(|f| keep(f)).cloned() });
    }
    for it in ctx.items.iter().filter(|i| i.kind == ItemKind::Document && picked(&i.id)) {
        actions.push(Action::OpenDocument { path: it.value.clone(), bundle_id: it.bundle_id.clone() });
    }
    for (bid, name, _g, urls, frame) in windows {
        let family = script::browser_family(&bid);
        actions.push(Action::OpenTabs { browser: name, bundle_id: bid, family, urls, frame });
    }

    // En dernier : replacer les fenêtres des apps rouvertes (ou dont un fichier est rouvert).
    for app in ctx.items.iter().filter(|i| i.kind == ItemKind::App && !i.frames.is_empty()) {
        let Some(bid) = app.bundle_id.clone() else { continue };
        let concerned = picked(&app.id)
            || ctx.items.iter().any(|d| d.kind == ItemKind::Document && picked(&d.id) && d.bundle_id.as_deref() == Some(bid.as_str()));
        let frames: Vec<Frame> = app.frames.iter().filter(|f| keep(f)).cloned().collect();
        if concerned && !frames.is_empty() {
            actions.push(Action::PlaceWindows { bundle_id: bid, frames });
        }
    }
    actions
}

/// Les pages internes des navigateurs (nouvel onglet, favoris, réglages…) ne se rouvrent
/// pas depuis l'extérieur, et une seule d'entre elles suffit à faire échouer `open` pour
/// tous les onglets. On ne rouvre que les vraies pages.
pub fn reopenable(url: &str) -> bool {
    let u = url.trim().to_ascii_lowercase();
    u.starts_with("http://") || u.starts_with("https://") || u.starts_with("file://")
}

/// Combien de temps on laisse aux apps pour ouvrir leurs fenêtres avant de les replacer.
pub struct Timing {
    pub first_wait: Duration,
    pub retry_wait: Duration,
    pub attempts: u32,
}

impl Default for Timing {
    fn default() -> Self {
        Timing { first_wait: Duration::from_millis(1200), retry_wait: Duration::from_millis(1500), attempts: 3 }
    }
}

fn place_args(bundle_id: &str, frames: &[Frame]) -> Vec<String> {
    let mut args = vec![bundle_id.to_string()];
    for f in frames {
        args.extend([f.title.clone(), f.x.to_string(), f.y.to_string(), f.w.to_string(), f.h.to_string()]);
    }
    args
}

/// Exécute le plan. Rien ne s'arrête sur une erreur : on note ce qui a raté et on continue.
pub fn execute(r: &dyn Runner, actions: &[Action], own_bundle: &str, timing: &Timing) -> RestoreReport {
    let mut report = RestoreReport::default();
    let mut to_place: Vec<(&str, &[Frame])> = Vec::new();
    for a in actions {
        match a {
            Action::PlaceWindows { bundle_id, frames } => to_place.push((bundle_id.as_str(), frames.as_slice())),
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
            Action::OpenFolder { path, frame } => {
                if !Path::new(path).exists() {
                    report.failures.push(format!("Dossier introuvable : {}", script::file_name(path)));
                    continue;
                }
                // Avec le Finder piloté : la fenêtre revient à sa place. Sinon, simple `open`.
                let bounds = frame.as_ref().map(|f| f.to_arg()).unwrap_or_default();
                if r.run(script::FINDER_OPEN, &[path.clone(), bounds]).is_ok() {
                    report.opened += 1;
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
            Action::OpenTabs { browser, bundle_id, family, urls, frame } => {
                if urls.is_empty() {
                    continue;
                }
                // 1er essai : recréer la fenêtre avec AppleScript (garde le regroupement et la place).
                // Le script dit quelles URL n'ont pas pu être ouvertes : on ne rouvre que celles-là.
                let mut args = vec![frame.as_ref().map(|f| f.to_arg()).unwrap_or_default()];
                args.extend(urls.iter().cloned());
                let scripted: Option<Vec<String>> = family
                    .and_then(|f| script::open_tabs_script(f, bundle_id))
                    .and_then(|src| r.run(&src, &args).ok())
                    .map(|out| {
                        script::parse_failed(&out)
                            .into_iter()
                            .filter_map(|n| urls.get(n - 1).cloned())
                            .collect()
                    });
                // Sinon (ou pour les URL restantes) : `open -b navigateur url1 url2 …`,
                // qui marche avec presque tout, Firefox compris.
                let rest: Vec<String> = scripted.clone().unwrap_or_else(|| urls.clone());
                let rest_ok = rest.is_empty() || {
                    let mut args: Vec<&str> = vec!["-b", bundle_id.as_str()];
                    args.extend(rest.iter().map(|u| u.as_str()));
                    script::open(&args).is_ok()
                };
                if rest_ok {
                    report.opened += urls.len() as u32;
                } else {
                    let missing = rest.len() as u32;
                    report.opened += urls.len() as u32 - missing;
                    report.failures.push(if missing as usize == urls.len() {
                        format!("Les onglets de {browser} n'ont pas pu être rouverts.")
                    } else {
                        format!("{missing} onglet(s) de {browser} n'ont pas pu être rouverts.")
                    });
                }
            }
        }
    }

    // Les apps mettent un peu de temps à afficher leurs fenêtres : on attend, puis on
    // réessaie pour celles qui n'étaient pas encore prêtes. Sans Accessibilité, rien ne
    // se passe et ce n'est pas une erreur : les fenêtres restent où macOS les a mises.
    if !to_place.is_empty() {
        std::thread::sleep(timing.first_wait);
        let mut pending: Vec<(&str, &[Frame])> = to_place;
        for attempt in 0..timing.attempts {
            pending.retain(|(bid, frames)| {
                let placed = r
                    .run(script::PLACE_WINDOWS, &place_args(bid, frames))
                    .ok()
                    .and_then(|o| o.trim().parse::<usize>().ok())
                    .unwrap_or(0);
                placed < frames.len()
            });
            if pending.is_empty() || attempt + 1 == timing.attempts {
                break;
            }
            std::thread::sleep(timing.retry_wait);
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
            frames: Vec::new(),
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
            auto: false,
        }
    }

    #[test]
    fn tabs_are_grouped_by_window_and_browser_app_is_not_opened_twice() {
        let p = plan(&ctx(), None, true, &[]);
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

    fn fr(title: &str, x: i32) -> Frame {
        Frame { title: title.into(), x, y: 40, w: 900, h: 700 }
    }

    #[test]
    fn frames_follow_their_windows_and_skip_missing_screens() {
        let mut c = ctx();
        c.items[1].frames = vec![fr("main.rs", 10), fr("Loin", 5000)];
        c.items[2].frames = vec![fr("", 20)];
        c.items[5].frames = vec![fr("", 30)];
        let screens = [Screen { x: 0, y: 0, w: 1440, h: 900 }];
        let p = plan(&c, None, false, &screens);
        let place: Vec<_> = p.iter().filter_map(|a| match a {
            Action::PlaceWindows { bundle_id, frames } => Some((bundle_id.clone(), frames.len())),
            _ => None,
        }).collect();
        assert_eq!(place, vec![("com.microsoft.VSCode".to_string(), 1)]);
        assert!(matches!(p.last(), Some(Action::PlaceWindows { .. })), "le placement vient en dernier");
        assert!(p.iter().any(|a| matches!(a, Action::OpenTabs { frame: Some(f), .. } if f.x == 20)));
        assert!(p.iter().any(|a| matches!(a, Action::OpenFolder { frame: Some(f), .. } if f.x == 30)));
        assert_eq!(place_args("b", &[fr("T", 1)]), vec!["b", "T", "1", "40", "900", "700"]);
    }

    #[test]
    fn internal_pages_are_skipped() {
        let mut c = ctx();
        c.items[2].value = "chrome://newtab/".into();
        c.items.push(item("t4", ItemKind::Tab, "com.apple.Safari", "favorites://", Some(2)));
        let p = plan(&c, None, false, &[]);
        let tabs: Vec<_> = p
            .iter()
            .filter_map(|a| match a {
                Action::OpenTabs { urls, .. } => Some(urls.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(tabs, vec![vec!["https://b.fr".to_string()], vec!["https://c.fr".to_string()]]);
        assert!(reopenable(" HTTPS://x.fr") && reopenable("file:///Users/a.pdf") && !reopenable("about:blank"));
    }

    struct Fake(std::cell::RefCell<Vec<Vec<String>>>, Result<String, script::ScriptError>);
    impl Runner for Fake {
        fn run(&self, _s: &str, a: &[String]) -> Result<String, script::ScriptError> {
            self.0.borrow_mut().push(a.to_vec());
            self.1.clone()
        }
    }

    #[test]
    fn tab_script_reports_what_it_opened() {
        let a = vec![Action::OpenTabs {
            browser: "Chrome".into(),
            bundle_id: "com.google.Chrome".into(),
            family: Some(BrowserFamily::Chromium),
            urls: vec!["https://a.fr".into(), "https://b.fr".into()],
            frame: None,
        }];
        let f = Fake(Default::default(), Ok(String::new()));
        let r = execute(&f, &a, "me", &Timing::default());
        assert_eq!((r.opened, r.failures.len()), (2, 0));
        assert_eq!(f.0.borrow()[0], vec!["", "https://a.fr", "https://b.fr"]);
        assert_eq!(script::parse_failed("2"), vec![2]);
        assert_eq!(script::parse_failed(" 1,3\n"), vec![1, 3]);
        assert!(script::parse_failed("").is_empty());
    }

    #[test]
    fn selection_is_respected() {
        let sel: HashSet<String> = ["a1", "t2"].iter().map(|s| s.to_string()).collect();
        let p = plan(&ctx(), Some(&sel), false, &[]);
        assert_eq!(p.len(), 1);
        assert!(matches!(&p[0], Action::OpenTabs { urls, .. } if urls == &vec!["https://b.fr".to_string()]));
    }
}
