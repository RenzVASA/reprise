//! Un seul point d'entrée pour tout ce qui dépend du système : capture, reprise,
//! surveillance de l'absence, ouverture de liens. Le reste de Reprise ne sait pas
//! s'il tourne sur Mac, Windows ou Linux.

#[cfg(target_os = "macos")]
mod imp {
    use crate::model::{RestoreReport, Snapshot};
    use crate::restore::{self, Action};
    use crate::script::{self, Osascript, RunningApp, Runner, ScriptError};
    use crate::{capture, watch};
    use std::path::Path;

    pub fn running_apps() -> Result<Vec<RunningApp>, ScriptError> {
        capture::running_apps(&Osascript::default())
    }
    pub fn full_capture(apps: &[RunningApp], own: &str, ignored: &[String]) -> Snapshot {
        capture::full_capture(&Osascript::default(), apps, own, ignored)
    }
    pub fn capture_all(own: &str, ignored: &[String]) -> Result<Snapshot, ScriptError> {
        capture::capture_all(&Osascript::default(), own, ignored)
    }
    pub fn execute(actions: &[Action], own: &str) -> RestoreReport {
        restore::execute(&Osascript::default(), actions, own, &restore::Timing::default())
    }
    pub fn screen_locked() -> Option<bool> {
        watch::screen_locked()
    }
    pub fn idle_seconds() -> Option<u64> {
        watch::idle_seconds()
    }
    pub fn display_kept_awake() -> Option<bool> {
        watch::display_kept_awake()
    }
    pub fn open_url(url: &str) -> Result<(), String> {
        script::open(&[url])
    }
    pub fn reveal(path: &Path) -> Result<(), String> {
        script::open(&["-R", &path.to_string_lossy()])
    }
    pub fn open_dir(path: &Path) -> Result<(), String> {
        script::open(&[&path.to_string_lossy()])
    }

    fn status(r: Result<String, ScriptError>) -> &'static str {
        match r {
            Ok(_) => "ok",
            Err(ScriptError::NotAuthorized) | Err(ScriptError::Accessibility) => "denied",
            Err(_) => "unknown",
        }
    }
    /// (Automatisation, Accessibilité)
    pub fn permissions() -> (&'static str, &'static str) {
        let r = Osascript::default();
        (status(r.run(script::PERM_AUTOMATION, &[])), status(r.run(script::PERM_ACCESSIBILITY, &[])))
    }
    pub fn open_privacy(kind: &str) -> Result<(), String> {
        let anchor = if kind == "accessibility" { "Privacy_Accessibility" } else { "Privacy_Automation" };
        script::open(&[&format!("x-apple.systempreferences:com.apple.preference.security?{anchor}")])
    }
}

#[cfg(target_os = "windows")]
mod imp {
    pub use crate::platform_windows::*;
    use std::path::Path;

    pub fn open_dir(path: &Path) -> Result<(), String> {
        crate::sys::spawn("explorer.exe", &[&path.to_string_lossy()])
    }
    /// Pas d'autorisation à donner sous Windows.
    pub fn permissions() -> (&'static str, &'static str) {
        ("na", "na")
    }
    pub fn open_privacy(_kind: &str) -> Result<(), String> {
        Ok(())
    }
}

#[cfg(target_os = "linux")]
mod imp {
    pub use crate::platform_linux::*;
    use std::path::Path;

    pub fn open_dir(path: &Path) -> Result<(), String> {
        crate::sys::spawn("xdg-open", &[&path.to_string_lossy()])
    }
    /// Pas d'autorisation à donner sous Linux.
    pub fn permissions() -> (&'static str, &'static str) {
        ("na", "na")
    }
    pub fn open_privacy(_kind: &str) -> Result<(), String> {
        Ok(())
    }
}

pub use imp::*;

/// "macos", "windows" ou "linux".
pub const OS: &str = std::env::consts::OS;
