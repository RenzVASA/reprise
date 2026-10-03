//! Reprise — sauvegarde ton contexte de travail en un raccourci, reprends-le en un clic.

mod capture;
mod model;
mod restore;
mod script;
mod settings;
mod store;
mod update;

use model::{now_ms, new_id, Context, ContextPatch, PendingCapture, RestoreReport, Snapshot};
use script::{Osascript, Runner, ScriptError};
use serde::Serialize;
use settings::{IgnoredApp, Settings};
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};
use store::Store;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, WindowEvent};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

const TRAY_ID: &str = "reprise-tray";

// ─────────────────────────────────── État ───────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct NotePayload {
    pub id: String,
    pub name: String,
    pub note: String,
}

#[derive(Default)]
struct Shortcuts {
    save: Option<Shortcut>,
    open: Option<Shortcut>,
}

struct AppState {
    store: Mutex<Store>,
    settings: Mutex<Settings>,
    settings_path: PathBuf,
    pending: Mutex<Option<PendingCapture>>,
    /// La capture en attente vient-elle de la fenêtre principale (et pas d'un raccourci) ?
    pending_from_app: Mutex<bool>,
    note: Mutex<Option<NotePayload>>,
    shortcuts: Mutex<Shortcuts>,
    /// Dernier résultat de la recherche de mise à jour.
    update: Mutex<Option<update::UpdateInfo>>,
}

/// Verrou qui survit à un panic dans un autre fil (on préfère des données un peu
/// vieilles à une app figée).
fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

fn own_bundle(app: &AppHandle) -> String {
    app.config().identifier.clone()
}

// ───────────────────────────────── Fenêtres ─────────────────────────────────

fn show_window(app: &AppHandle, label: &str) {
    if let Some(w) = app.get_webview_window(label) {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
    }
}

fn hide_window(app: &AppHandle, label: &str) {
    if let Some(w) = app.get_webview_window(label) {
        let _ = w.hide();
    }
}

/// Rend la main à l'app qu'on utilisait avant (macOS : masque Reprise entièrement).
fn give_focus_back(app: &AppHandle) {
    #[cfg(target_os = "macos")]
    {
        let _ = app.hide();
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = app;
    }
}

fn notify_changed(app: &AppHandle) {
    let _ = app.emit("contexts-changed", ());
    refresh_tray(app);
}

// ───────────────────────────────── Capture ──────────────────────────────────

fn script_error_text(e: &ScriptError) -> String {
    match e {
        ScriptError::NotAuthorized => "Reprise n'a pas le droit de lister tes apps. Autorise « System Events » dans Réglages Système › Confidentialité et sécurité › Automatisation.".into(),
        ScriptError::Accessibility => "Reprise a besoin de l'accès Accessibilité (Réglages Système › Confidentialité et sécurité).".into(),
        ScriptError::Timeout => "macOS a mis trop de temps à répondre. Réessaie dans un instant.".into(),
        ScriptError::NotRunning => "Une app ne répondait pas pendant la capture.".into(),
        ScriptError::Other(m) => format!("La capture a échoué : {m}"),
    }
}

/// Lance une capture : photo rapide des apps, ouverture de la fenêtre de sauvegarde,
/// puis le reste (onglets, dossiers, fichiers) en arrière-plan.
fn begin_capture(app: AppHandle, replace_id: Option<String>, from_app: bool) {
    std::thread::spawn(move || {
        let state = app.state::<AppState>();
        let own = own_bundle(&app);
        let ignored = lock(&state.settings).ignored_ids();
        let runner = Osascript::default();

        let (apps, first_error) = match capture::running_apps(&runner) {
            Ok(a) => (a, None),
            Err(e) => (Vec::new(), Some(e)),
        };
        let started_at = now_ms();
        *lock(&state.pending) = Some(PendingCapture {
            snapshot: None,
            front_app: capture::frontmost(&apps, &own),
            started_at,
            replace_id,
        });
        *lock(&state.pending_from_app) = from_app;

        let _ = app.emit_to("capture", "capture-start", ());
        show_window(&app, "capture");

        let mut snap: Snapshot = capture::full_capture(&runner, &apps, &own, &ignored);
        if let Some(e) = first_error {
            snap.warnings.insert(0, script_error_text(&e));
        }
        if let Some(p) = lock(&state.pending).as_mut() {
            if p.started_at == started_at {
                p.snapshot = Some(snap);
            }
        }
        let _ = app.emit_to("capture", "capture-ready", ());
    });
}

// ──────────────────────────────── Reprise ───────────────────────────────────

fn restore_internal(app: &AppHandle, id: &str, items: Option<Vec<String>>, tidy: Option<bool>) -> Result<RestoreReport, String> {
    let state = app.state::<AppState>();
    let ctx = lock(&state.store).get(id).cloned().ok_or("Ce contexte n'existe plus.")?;
    let (tidy_default, show_note) = {
        let s = lock(&state.settings);
        (s.tidy_by_default, s.show_note)
    };
    let selected: Option<HashSet<String>> = items.map(|v| v.into_iter().collect());
    let actions = restore::plan(&ctx, selected.as_ref(), tidy.unwrap_or(tidy_default));
    let report = restore::execute(&Osascript::default(), &actions, &own_bundle(app));

    lock(&state.store).mark_restored(id)?;
    notify_changed(app);

    if report.failures.is_empty() {
        hide_window(app, "main");
    }
    if show_note && !ctx.note.trim().is_empty() {
        *lock(&state.note) = Some(NotePayload { id: ctx.id.clone(), name: ctx.name.clone(), note: ctx.note.clone() });
        show_note_window(app);
    }
    Ok(report)
}

fn show_note_window(app: &AppHandle) {
    let Some(w) = app.get_webview_window("note") else { return };
    if let Ok(Some(m)) = w.primary_monitor() {
        let area = m.work_area();
        let scale = m.scale_factor();
        let size = w.outer_size().map(|s| s.width).unwrap_or((380.0 * scale) as u32);
        let margin = (20.0 * scale) as i32;
        let x = area.position.x + area.size.width as i32 - size as i32 - margin;
        let y = area.position.y + margin;
        let _ = w.set_position(PhysicalPosition::new(x, y));
    }
    let _ = app.emit_to("note", "note-show", ());
    let _ = w.show();
}

// ──────────────────────────── Barre des menus ───────────────────────────────

fn build_tray_menu(app: &AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
    let state = app.state::<AppState>();
    let shortcut = lock(&state.settings).shortcut_save.clone();
    let recent = lock(&state.store).recent(6);

    let menu = Menu::new(app)?;
    menu.append(&MenuItem::with_id(
        app,
        "save",
        format!("Sauvegarder le contexte…   {}", pretty_shortcut(&shortcut)),
        true,
        None::<&str>,
    )?)?;
    menu.append(&PredefinedMenuItem::separator(app)?)?;
    if recent.is_empty() {
        menu.append(&MenuItem::with_id(app, "none", "Aucun contexte pour l'instant", false, None::<&str>)?)?;
    } else {
        menu.append(&MenuItem::with_id(app, "title", "Reprendre", false, None::<&str>)?)?;
        for c in recent {
            let label = if c.pinned { format!("★  {}", c.name) } else { format!("    {}", c.name) };
            menu.append(&MenuItem::with_id(app, format!("restore:{}", c.id), label, true, None::<&str>)?)?;
        }
    }
    menu.append(&PredefinedMenuItem::separator(app)?)?;
    let update_label = match lock(&state.update).as_ref() {
        Some(u) if u.available => format!("Nouvelle version {} disponible…", u.latest),
        _ => "Rechercher les mises à jour…".to_string(),
    };
    menu.append(&MenuItem::with_id(app, "update", update_label, true, None::<&str>)?)?;
    menu.append(&MenuItem::with_id(app, "open", "Ouvrir Reprise", true, None::<&str>)?)?;
    menu.append(&MenuItem::with_id(app, "quit", "Quitter Reprise", true, None::<&str>)?)?;
    Ok(menu)
}

fn refresh_tray(app: &AppHandle) {
    if let (Some(tray), Ok(menu)) = (app.tray_by_id(TRAY_ID), build_tray_menu(app)) {
        let _ = tray.set_menu(Some(menu));
    }
}

/// "Alt+Command+S" → "⌥⌘S".
fn pretty_shortcut(s: &str) -> String {
    let mut mods = String::new();
    let mut key = String::new();
    for part in s.split('+') {
        match part.trim().to_ascii_lowercase().as_str() {
            "control" | "ctrl" => mods.push('⌃'),
            "alt" | "option" => mods.push('⌥'),
            "shift" => mods.push('⇧'),
            "command" | "cmd" | "super" | "meta" | "commandorcontrol" | "cmdorctrl" => mods.push('⌘'),
            other => {
                let k = other.trim_start_matches("key").trim_start_matches("digit");
                key = k.to_uppercase();
            }
        }
    }
    let order = ['⌃', '⌥', '⇧', '⌘'];
    let mut sorted: String = order.iter().filter(|c| mods.contains(**c)).collect();
    sorted.push_str(&key);
    sorted
}

fn setup_tray(app: &AppHandle) -> tauri::Result<()> {
    let menu = build_tray_menu(app)?;
    let icon = tauri::image::Image::from_bytes(include_bytes!("../icons/tray.png"))?;
    TrayIconBuilder::<tauri::Wry>::with_id(TRAY_ID)
        .icon(icon)
        .icon_as_template(true)
        .tooltip("Reprise")
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| {
            let id = event.id().0.clone();
            match id.as_str() {
                "save" => begin_capture(app.clone(), None, false),
                "open" => show_window(app, "main"),
                "quit" => app.exit(0),
                "update" => {
                    show_window(app, "main");
                    let app = app.clone();
                    std::thread::spawn(move || match run_update_check(&app) {
                        Ok(info) if !info.available => {
                            let _ = app.emit("toast", format!("Reprise est à jour (version {}).", info.current));
                        }
                        Ok(_) => {}
                        Err(e) => {
                            let _ = app.emit("toast", e);
                        }
                    });
                }
                other => {
                    if let Some(ctx_id) = other.strip_prefix("restore:") {
                        let app = app.clone();
                        let ctx_id = ctx_id.to_string();
                        std::thread::spawn(move || {
                            if let Err(e) = restore_internal(&app, &ctx_id, None, None) {
                                let _ = app.emit("toast", e);
                            }
                        });
                    }
                }
            }
        })
        .build(app)?;
    Ok(())
}

// ───────────────────────────── Raccourcis globaux ────────────────────────────

fn apply_shortcuts(app: &AppHandle, save: &str, open: &str) -> Result<(), String> {
    let save_sc: Shortcut = save.parse().map_err(|_| format!("Raccourci invalide : {save}"))?;
    let open_sc: Shortcut = open.parse().map_err(|_| format!("Raccourci invalide : {open}"))?;
    if save_sc == open_sc {
        return Err("Les deux raccourcis doivent être différents.".into());
    }
    let gs = app.global_shortcut();
    let _ = gs.unregister_all();
    gs.register(save_sc)
        .map_err(|_| format!("Le raccourci {} est déjà pris par une autre app.", pretty_shortcut(save)))?;
    gs.register(open_sc)
        .map_err(|_| format!("Le raccourci {} est déjà pris par une autre app.", pretty_shortcut(open)))?;
    let state = app.state::<AppState>();
    *lock(&state.shortcuts) = Shortcuts { save: Some(save_sc), open: Some(open_sc) };
    Ok(())
}

fn on_shortcut(app: &AppHandle, shortcut: &Shortcut) {
    let state = app.state::<AppState>();
    let (is_save, is_open) = {
        let s = lock(&state.shortcuts);
        (s.save.as_ref() == Some(shortcut), s.open.as_ref() == Some(shortcut))
    };
    if is_save {
        // Fenêtre déjà ouverte : on la remet juste au premier plan.
        let open_already = app
            .get_webview_window("capture")
            .and_then(|w| w.is_visible().ok())
            .unwrap_or(false);
        if open_already {
            show_window(app, "capture");
        } else {
            begin_capture(app.clone(), None, false);
        }
    } else if is_open {
        show_window(app, "main");
    }
}

#[cfg(target_os = "macos")]
fn apply_dock(app: &AppHandle, show: bool) {
    let policy = if show { tauri::ActivationPolicy::Regular } else { tauri::ActivationPolicy::Accessory };
    let _ = app.set_activation_policy(policy);
}

#[cfg(not(target_os = "macos"))]
fn apply_dock(_app: &AppHandle, _show: bool) {}

// ───────────────────────────────── Commandes ─────────────────────────────────

#[tauri::command]
fn list_contexts(app: AppHandle) -> Vec<Context> {
    lock(&app.state::<AppState>().store).list()
}

#[tauri::command]
fn update_context(app: AppHandle, id: String, patch: ContextPatch) -> Result<Context, String> {
    let out = lock(&app.state::<AppState>().store).update(&id, patch)?;
    notify_changed(&app);
    Ok(out)
}

#[tauri::command]
fn delete_context(app: AppHandle, id: String) -> Result<Context, String> {
    let out = lock(&app.state::<AppState>().store).delete(&id)?;
    notify_changed(&app);
    Ok(out)
}

/// Pour « Annuler » après une suppression.
#[tauri::command]
fn put_back_context(app: AppHandle, context: Context) -> Result<(), String> {
    lock(&app.state::<AppState>().store).insert(context)?;
    notify_changed(&app);
    Ok(())
}

#[tauri::command]
fn start_capture(app: AppHandle, replace: Option<String>) {
    begin_capture(app, replace, true);
}

#[derive(Serialize)]
struct PendingView {
    ready: bool,
    front_app: Option<String>,
    snapshot: Option<Snapshot>,
    replace: Option<Context>,
}

#[tauri::command]
fn get_pending_capture(app: AppHandle) -> Option<PendingView> {
    let state = app.state::<AppState>();
    let p = lock(&state.pending).clone()?;
    let replace = p.replace_id.as_deref().and_then(|id| lock(&state.store).get(id).cloned());
    Some(PendingView { ready: p.snapshot.is_some(), front_app: p.front_app, snapshot: p.snapshot, replace })
}

#[tauri::command]
fn save_capture(app: AppHandle, name: String, note: String, exclude: Option<Vec<String>>) -> Result<Context, String> {
    let state = app.state::<AppState>();
    let pending = lock(&state.pending).clone().ok_or("Aucune capture en cours.")?;
    let mut snap = pending.snapshot.ok_or("La photo n'est pas encore prête.")?;
    // Ce que l'utilisateur a décoché dans la fenêtre de sauvegarde n'est pas gardé.
    if let Some(ex) = exclude {
        let ex: HashSet<String> = ex.into_iter().collect();
        snap.items.retain(|i| !ex.contains(&i.id));
    }
    let now = now_ms();
    let name = {
        let n = name.trim();
        if n.is_empty() { pending.front_app.clone().unwrap_or_else(|| "Sans titre".into()) } else { n.to_string() }
    };

    let saved = {
        let mut store = lock(&state.store);
        match pending.replace_id.as_deref().and_then(|id| store.get(id).cloned()) {
            Some(mut c) => {
                c.name = name;
                c.note = note.trim().to_string();
                c.items = snap.items;
                c.front_app = snap.front_app;
                c.updated_at = now;
                store.insert(c.clone())?;
                c
            }
            None => {
                let c = Context {
                    id: new_id(),
                    name,
                    note: note.trim().to_string(),
                    created_at: now,
                    updated_at: now,
                    last_restored_at: None,
                    restore_count: 0,
                    pinned: false,
                    front_app: snap.front_app,
                    items: snap.items,
                };
                store.insert(c.clone())?;
                c
            }
        }
    };

    *lock(&state.pending) = None;
    hide_window(&app, "capture");
    if !*lock(&state.pending_from_app) {
        give_focus_back(&app);
    }
    notify_changed(&app);
    let _ = app.emit("context-saved", saved.id.clone());
    Ok(saved)
}

#[tauri::command]
fn cancel_capture(app: AppHandle) {
    let state = app.state::<AppState>();
    *lock(&state.pending) = None;
    hide_window(&app, "capture");
    if !*lock(&state.pending_from_app) {
        give_focus_back(&app);
    }
}

#[tauri::command]
async fn restore_context(app: AppHandle, id: String, items: Option<Vec<String>>, tidy: Option<bool>) -> Result<RestoreReport, String> {
    let handle = app.clone();
    tauri::async_runtime::spawn_blocking(move || restore_internal(&handle, &id, items, tidy))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
fn get_note(app: AppHandle) -> Option<NotePayload> {
    lock(&app.state::<AppState>().note).clone()
}

#[tauri::command]
fn close_note(app: AppHandle) {
    *lock(&app.state::<AppState>().note) = None;
    hide_window(&app, "note");
}

#[tauri::command]
fn open_main(app: AppHandle) {
    show_window(&app, "main");
}

/// « Ne jamais enregistrer cette app ».
#[tauri::command]
fn ignore_app(app: AppHandle, bundle_id: String, name: String) -> Result<Settings, String> {
    let state = app.state::<AppState>();
    let mut s = lock(&state.settings).clone();
    if !s.ignored_apps.iter().any(|a| a.bundle_id == bundle_id) {
        s.ignored_apps.push(IgnoredApp { bundle_id, name });
    }
    let s = s.sanitized();
    s.save(&state.settings_path)?;
    *lock(&state.settings) = s.clone();
    let _ = app.emit("settings-changed", s.clone());
    Ok(s)
}

fn run_update_check(app: &AppHandle) -> Result<update::UpdateInfo, String> {
    let current = app.package_info().version.to_string();
    let info = update::check(&current)?;
    let state = app.state::<AppState>();
    *lock(&state.update) = Some(info.clone());
    {
        let mut s = lock(&state.settings);
        s.last_update_check = now_ms();
        let _ = s.save(&state.settings_path);
    }
    refresh_tray(app);
    if info.available {
        let _ = app.emit("update-available", info.clone());
    }
    Ok(info)
}

/// Bouton « Vérifier les mises à jour ».
#[tauri::command]
async fn check_updates(app: AppHandle) -> Result<update::UpdateInfo, String> {
    let handle = app.clone();
    tauri::async_runtime::spawn_blocking(move || run_update_check(&handle))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
fn get_update(app: AppHandle) -> Option<update::UpdateInfo> {
    lock(&app.state::<AppState>().update).clone()
}

/// Pendant qu'on enregistre une nouvelle combinaison dans les réglages, on coupe les
/// raccourcis globaux (sinon appuyer sur l'ancien lancerait une capture).
#[tauri::command]
fn pause_shortcuts(app: AppHandle, paused: bool) -> Result<(), String> {
    if paused {
        app.global_shortcut().unregister_all().map_err(|e| e.to_string())
    } else {
        let s = lock(&app.state::<AppState>().settings).clone();
        apply_shortcuts(&app, &s.shortcut_save, &s.shortcut_open)
    }
}

#[tauri::command]
fn get_settings(app: AppHandle) -> Settings {
    let state = app.state::<AppState>();
    let mut s = lock(&state.settings).clone();
    if let Ok(enabled) = app.autolaunch().is_enabled() {
        s.launch_at_login = enabled;
    }
    s
}

#[tauri::command]
fn set_settings(app: AppHandle, settings: Settings) -> Result<Settings, String> {
    let state = app.state::<AppState>();
    let new = settings.sanitized();
    let old = lock(&state.settings).clone();

    if new.shortcut_save != old.shortcut_save || new.shortcut_open != old.shortcut_open {
        if let Err(e) = apply_shortcuts(&app, &new.shortcut_save, &new.shortcut_open) {
            // On remet les anciens pour ne jamais rester sans raccourci.
            let _ = apply_shortcuts(&app, &old.shortcut_save, &old.shortcut_open);
            return Err(e);
        }
    }
    if new.show_in_dock != old.show_in_dock {
        apply_dock(&app, new.show_in_dock);
        show_window(&app, "main");
    }
    let autostart = app.autolaunch();
    if new.launch_at_login != autostart.is_enabled().unwrap_or(false) {
        let res = if new.launch_at_login { autostart.enable() } else { autostart.disable() };
        res.map_err(|e| format!("Impossible de régler le lancement au démarrage : {e}"))?;
    }

    new.save(&state.settings_path)?;
    *lock(&state.settings) = new.clone();
    refresh_tray(&app);
    let _ = app.emit("settings-changed", new.clone());
    Ok(new)
}

#[derive(Serialize)]
struct Permissions {
    automation: &'static str,
    accessibility: &'static str,
}

fn perm_status(r: Result<String, ScriptError>) -> &'static str {
    match r {
        Ok(_) => "ok",
        Err(ScriptError::NotAuthorized) | Err(ScriptError::Accessibility) => "denied",
        Err(_) => "unknown",
    }
}

/// Vérifie les autorisations (et déclenche les demandes de macOS la première fois).
#[tauri::command]
async fn check_permissions() -> Permissions {
    tauri::async_runtime::spawn_blocking(|| {
        let r = Osascript::default();
        Permissions {
            automation: perm_status(r.run(script::PERM_AUTOMATION, &[])),
            accessibility: perm_status(r.run(script::PERM_ACCESSIBILITY, &[])),
        }
    })
    .await
    .unwrap_or(Permissions { automation: "unknown", accessibility: "unknown" })
}

#[tauri::command]
fn open_privacy_pane(kind: String) -> Result<(), String> {
    let anchor = match kind.as_str() {
        "accessibility" => "Privacy_Accessibility",
        _ => "Privacy_Automation",
    };
    script::open(&[&format!("x-apple.systempreferences:com.apple.preference.security?{anchor}")])
}

#[tauri::command]
fn reveal_data(app: AppHandle) -> Result<(), String> {
    let path = lock(&app.state::<AppState>().store).path().to_path_buf();
    if path.exists() {
        script::open(&["-R", &path.to_string_lossy()])
    } else if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        script::open(&[&dir.to_string_lossy()])
    } else {
        Err("Dossier de données introuvable.".into())
    }
}

#[tauri::command]
fn open_link(url: String) -> Result<(), String> {
    if !url.starts_with("https://") {
        return Err("Lien refusé.".into());
    }
    script::open(&[&url])
}

/// Ouvre un seul élément d'un contexte (clic sur une ligne dans le détail).
#[tauri::command]
async fn open_item(app: AppHandle, id: String, item: String) -> Result<RestoreReport, String> {
    let handle = app.clone();
    tauri::async_runtime::spawn_blocking(move || -> Result<RestoreReport, String> {
        let state = handle.state::<AppState>();
        let ctx = lock(&state.store).get(&id).cloned().ok_or("Ce contexte n'existe plus.")?;
        let sel: HashSet<String> = [item].into_iter().collect();
        let actions = restore::plan(&ctx, Some(&sel), false);
        Ok(restore::execute(&Osascript::default(), &actions, &own_bundle(&handle)))
    })
    .await
    .map_err(|e| e.to_string())?
}

#[derive(Serialize)]
struct AppInfo {
    version: String,
    data_path: String,
}

#[tauri::command]
fn app_info(app: AppHandle) -> AppInfo {
    AppInfo {
        version: app.package_info().version.to_string(),
        data_path: lock(&app.state::<AppState>().store).path().to_string_lossy().into_owned(),
    }
}

// ───────────────────────────────── Démarrage ─────────────────────────────────

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let started_hidden = std::env::args().any(|a| a == "--hidden");

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            show_window(app, "main");
        }))
        .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, Some(vec!["--hidden"])))
        .plugin(
            tauri_plugin_global_shortcut::Builder::<tauri::Wry>::new()
                .with_handler(|app, shortcut, event| {
                    if event.state() == ShortcutState::Pressed {
                        on_shortcut(app, shortcut);
                    }
                })
                .build(),
        )
        .setup(move |app| {
            let handle = app.handle().clone();
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            let settings_path = data_dir.join("settings.json");
            let settings = Settings::load(&settings_path);
            let store = Store::load(data_dir.join("contexts.json"));

            app.manage(AppState {
                store: Mutex::new(store),
                settings: Mutex::new(settings.clone()),
                settings_path,
                pending: Mutex::new(None),
                pending_from_app: Mutex::new(false),
                note: Mutex::new(None),
                shortcuts: Mutex::new(Shortcuts::default()),
                update: Mutex::new(None),
            });

            if let Err(e) = apply_shortcuts(&handle, &settings.shortcut_save, &settings.shortcut_open) {
                // Raccourci personnalisé indisponible : on retombe sur ceux par défaut.
                eprintln!("{e}");
                let _ = apply_shortcuts(&handle, settings::DEFAULT_SAVE_SHORTCUT, settings::DEFAULT_OPEN_SHORTCUT);
            }
            apply_dock(&handle, settings.show_in_dock);
            setup_tray(&handle)?;

            if !started_hidden {
                show_window(&handle, "main");
            }

            // Mise à jour : au plus une vérification par jour, en arrière-plan.
            if settings.check_updates && now_ms().saturating_sub(settings.last_update_check) > 20 * 3600 * 1000 {
                let h = handle.clone();
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_secs(8));
                    let _ = run_update_check(&h);
                });
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                // Fermer une fenêtre ne quitte pas Reprise : elle reste dans la barre des menus.
                api.prevent_close();
                let _ = window.hide();
                if window.label() == "capture" {
                    let app = window.app_handle();
                    *lock(&app.state::<AppState>().pending) = None;
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            list_contexts,
            update_context,
            delete_context,
            put_back_context,
            start_capture,
            get_pending_capture,
            save_capture,
            cancel_capture,
            restore_context,
            open_item,
            get_note,
            close_note,
            open_main,
            get_settings,
            set_settings,
            pause_shortcuts,
            ignore_app,
            check_updates,
            get_update,
            check_permissions,
            open_privacy_pane,
            reveal_data,
            open_link,
            app_info,
        ])
        .build(tauri::generate_context!())
        .expect("impossible de démarrer Reprise")
        .run(|app, event| {
            #[cfg(target_os = "macos")]
            if let tauri::RunEvent::Reopen { .. } = event {
                // Clic sur l'icône du Dock : on rouvre la fenêtre principale.
                show_window(app, "main");
            }
            #[cfg(not(target_os = "macos"))]
            {
                let _ = (app, event);
            }
        });
}
