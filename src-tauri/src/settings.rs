//! Réglages de l'app, dans settings.json à côté des contextes.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

pub const DEFAULT_SAVE_SHORTCUT: &str = "Alt+Command+S";
pub const DEFAULT_OPEN_SHORTCUT: &str = "Alt+Command+R";
pub const DEFAULT_RESUME_SHORTCUT: &str = "Alt+Shift+Command+R";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Raccourci global pour sauvegarder le contexte.
    pub shortcut_save: String,
    /// Raccourci global pour ouvrir Reprise.
    pub shortcut_open: String,
    /// Raccourci global pour reprendre le dernier contexte.
    pub shortcut_resume: String,
    /// Police de lecture : "atkinson", "lexend", "opendyslexic", "system".
    pub font: String,
    /// Taille du texte en pourcentage (90 à 140).
    pub text_scale: u32,
    /// Masquer les autres apps avant de reprendre un contexte.
    pub tidy_by_default: bool,
    /// Afficher le post-it « Tu en étais là » après une reprise.
    pub show_note: bool,
    /// Afficher l'icône dans le Dock (sinon : seulement dans la barre des menus).
    pub show_in_dock: bool,
    /// Lancer Reprise à l'ouverture de session.
    pub launch_at_login: bool,
    /// Thème : "auto", "light", "dark".
    pub theme: String,
    /// L'accueil a déjà été vu.
    pub onboarded: bool,
    /// Apps à ne jamais enregistrer (ni l'app, ni ses onglets, ni ses fichiers).
    pub ignored_apps: Vec<IgnoredApp>,
    /// Chercher une nouvelle version au démarrage (une fois par jour au plus).
    pub check_updates: bool,
    /// Dernière recherche de mise à jour (ms depuis 1970).
    pub last_update_check: u64,
    /// Filet de sécurité : sauvegarder tout seul quand on s'absente.
    pub auto_save: bool,
    /// Proposer de reprendre en revenant après une absence.
    pub welcome_back: bool,
    /// Durée d'absence (minutes) avant de proposer de reprendre.
    pub welcome_minutes: u32,
    /// Astuces et visites déjà vues (« tour », « capture »…).
    pub seen: Vec<String>,
    /// Dernière version lancée, pour montrer les nouveautés après une mise à jour.
    pub last_version: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IgnoredApp {
    pub bundle_id: String,
    pub name: String,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            shortcut_save: DEFAULT_SAVE_SHORTCUT.into(),
            shortcut_open: DEFAULT_OPEN_SHORTCUT.into(),
            font: "atkinson".into(),
            text_scale: 100,
            tidy_by_default: false,
            show_note: true,
            show_in_dock: true,
            launch_at_login: false,
            theme: "auto".into(),
            onboarded: false,
            ignored_apps: Vec::new(),
            check_updates: true,
            last_update_check: 0,
            shortcut_resume: DEFAULT_RESUME_SHORTCUT.into(),
            auto_save: true,
            welcome_back: true,
            welcome_minutes: 20,
            seen: Vec::new(),
            last_version: String::new(),
        }
    }
}

impl Settings {
    pub fn load(path: &Path) -> Settings {
        fs::read_to_string(path)
            .ok()
            .and_then(|t| serde_json::from_str::<Settings>(&t).ok())
            .map(|s| s.sanitized())
            .unwrap_or_default()
    }

    pub fn save(&self, path: &Path) -> Result<(), String> {
        let data = serde_json::to_vec_pretty(self).map_err(|e| e.to_string())?;
        crate::store::write_atomic(path, &data).map_err(|e| e.to_string())
    }

    pub fn sanitized(mut self) -> Settings {
        self.text_scale = self.text_scale.clamp(90, 140);
        if !["atkinson", "lexend", "opendyslexic", "system"].contains(&self.font.as_str()) {
            self.font = "atkinson".into();
        }
        if !["auto", "light", "dark"].contains(&self.theme.as_str()) {
            self.theme = "auto".into();
        }
        if self.shortcut_save.trim().is_empty() {
            self.shortcut_save = DEFAULT_SAVE_SHORTCUT.into();
        }
        if self.shortcut_open.trim().is_empty() {
            self.shortcut_open = DEFAULT_OPEN_SHORTCUT.into();
        }
        if self.shortcut_resume.trim().is_empty() {
            self.shortcut_resume = DEFAULT_RESUME_SHORTCUT.into();
        }
        self.welcome_minutes = self.welcome_minutes.clamp(5, 240);
        self.seen.sort();
        self.seen.dedup();
        let mut seen = std::collections::HashSet::new();
        self.ignored_apps.retain(|a| !a.bundle_id.trim().is_empty() && seen.insert(a.bundle_id.clone()));
        self
    }

    pub fn ignored_ids(&self) -> Vec<String> {
        self.ignored_apps.iter().map(|a| a.bundle_id.clone()).collect()
    }
}
