//! Les données manipulées par Reprise : un « contexte » est une photo de ton
//! espace de travail (apps, onglets, dossiers, fichiers) + une note « où j'en étais ».

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ItemKind {
    /// Une application ouverte.
    App,
    /// Un onglet de navigateur.
    Tab,
    /// Une fenêtre du Finder (un dossier).
    Folder,
    /// Un fichier ouvert dans une app.
    Document,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Item {
    pub id: String,
    pub kind: ItemKind,
    /// Nom de l'app concernée (l'app elle-même, le navigateur, l'app qui ouvre le fichier…).
    pub app_name: String,
    /// Identifiant de bundle macOS (ex. com.apple.Safari) quand on le connaît.
    #[serde(default)]
    pub bundle_id: Option<String>,
    /// Ce qu'on affiche : titre d'onglet, nom de dossier, nom de fichier, nom d'app.
    pub label: String,
    /// Ce qu'on rouvre : URL, chemin POSIX, ou bundle id pour une app.
    pub value: String,
    /// Pour les onglets : numéro de la fenêtre du navigateur (pour reconstruire les fenêtres).
    #[serde(default)]
    pub group: Option<u32>,
    /// Position et taille des fenêtres (une pour un onglet ou un dossier, plusieurs pour une app).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub frames: Vec<Frame>,
}

/// Une fenêtre à l'écran, en points macOS (origine en haut à gauche de l'écran principal).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Frame {
    /// Titre de la fenêtre, pour la retrouver (vide pour les navigateurs et le Finder).
    #[serde(default)]
    pub title: String,
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Frame {
    /// "x,y,w,h", le format échangé avec les scripts AppleScript.
    pub fn to_arg(&self) -> String {
        format!("{},{},{},{}", self.x, self.y, self.w, self.h)
    }

    /// Une fenêtre trop petite (ou réduite) ne vaut pas la peine d'être replacée.
    pub fn is_usable(&self) -> bool {
        self.w >= 80 && self.h >= 60 && self.w < 20000 && self.h < 20000
    }

    pub fn center(&self) -> (i32, i32) {
        (self.x + self.w / 2, self.y + self.h / 2)
    }
}

/// Un écran, en points.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Screen {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

/// La fenêtre est-elle sur un écran branché ? (Sans liste d'écrans, on fait confiance.)
pub fn on_screen(f: &Frame, screens: &[Screen]) -> bool {
    if screens.is_empty() {
        return true;
    }
    let (cx, cy) = f.center();
    screens.iter().any(|s| cx >= s.x && cx < s.x + s.w && cy >= s.y && cy < s.y + s.h)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Context {
    pub id: String,
    pub name: String,
    /// La phrase « où j'en étais ».
    #[serde(default)]
    pub note: String,
    pub created_at: u64,
    pub updated_at: u64,
    #[serde(default)]
    pub last_restored_at: Option<u64>,
    #[serde(default)]
    pub restore_count: u32,
    #[serde(default)]
    pub pinned: bool,
    /// L'app qui était au premier plan au moment de la capture.
    #[serde(default)]
    pub front_app: Option<String>,
    #[serde(default)]
    pub items: Vec<Item>,
    /// Sauvegarde faite toute seule par le filet de sécurité.
    #[serde(default)]
    pub auto: bool,
}

/// Résultat brut d'une capture, avant que l'utilisateur ne lui donne un nom.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Snapshot {
    pub front_app: Option<String>,
    pub items: Vec<Item>,
    /// Messages lisibles sur ce qui n'a pas pu être capturé (autorisation manquante…).
    pub warnings: Vec<String>,
    pub taken_at: u64,
}

/// Une capture en attente : la fenêtre de sauvegarde est ouverte, l'utilisateur écrit sa note.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingCapture {
    /// `None` tant que la capture complète tourne encore.
    pub snapshot: Option<Snapshot>,
    pub front_app: Option<String>,
    pub started_at: u64,
    /// Si on « met à jour » un contexte existant au lieu d'en créer un.
    pub replace_id: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ContextPatch {
    pub name: Option<String>,
    pub note: Option<String>,
    pub pinned: Option<bool>,
    /// `Some(false)` : garder une sauvegarde auto comme un contexte normal.
    pub auto: Option<bool>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RestoreReport {
    pub opened: u32,
    pub failures: Vec<String>,
}

pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

static COUNTER: AtomicU32 = AtomicU32::new(0);

/// Identifiant court, unique dans la vie de l'app, triable dans le temps.
pub fn new_id() -> String {
    let n = COUNTER.fetch_add(1, Ordering::Relaxed) & 0xffff;
    format!("{:x}{:04x}", now_ms(), n)
}

impl Context {
    pub fn count(&self, kind: ItemKind) -> usize {
        self.items.iter().filter(|i| i.kind == kind).count()
    }

    /// Dernière fois qu'on s'est servi de ce contexte.
    pub fn last_used(&self) -> u64 {
        self.created_at.max(self.updated_at).max(self.last_restored_at.unwrap_or(0))
    }

    /// Empreinte du contenu, pour savoir si deux sauvegardes sont identiques.
    pub fn signature(items: &[Item]) -> Vec<(ItemKind, String)> {
        let mut v: Vec<(ItemKind, String)> = items.iter().map(|i| (i.kind, i.value.clone())).collect();
        v.sort_by(|a, b| (a.0 as u8).cmp(&(b.0 as u8)).then(a.1.cmp(&b.1)));
        v
    }
}

/// "v1.10.0" est-elle plus récente que "1.9.2" ? Compare les nombres un par un.
pub fn version_is_newer(latest: &str, current: &str) -> bool {
    fn parts(v: &str) -> Vec<u64> {
        v.trim()
            .trim_start_matches(['v', 'V'])
            .split(['-', '+'])
            .next()
            .unwrap_or("")
            .split('.')
            .map(|p| p.parse().unwrap_or(0))
            .collect()
    }
    let (a, b) = (parts(latest), parts(current));
    for i in 0..a.len().max(b.len()) {
        let (x, y) = (a.get(i).copied().unwrap_or(0), b.get(i).copied().unwrap_or(0));
        if x != y {
            return x > y;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frames_and_screens() {
        let f = Frame { title: String::new(), x: 100, y: 50, w: 800, h: 600 };
        assert_eq!(f.to_arg(), "100,50,800,600");
        assert!(f.is_usable());
        let main = Screen { x: 0, y: 0, w: 1440, h: 900 };
        assert!(on_screen(&f, &[main]));
        let far = Frame { x: 2000, ..f.clone() };
        assert!(!on_screen(&far, &[main]));
        let right = Screen { x: 1440, y: 0, w: 1920, h: 1080 };
        assert!(on_screen(&far, &[main, right]));
        assert!(on_screen(&far, &[]));
        assert!(!Frame { w: 10, ..f }.is_usable());
    }

    #[test]
    fn versions() {
        assert!(version_is_newer("v1.1.0", "1.0.0"));
        assert!(version_is_newer("1.10.0", "1.9.2"));
        assert!(!version_is_newer("v1.0.0", "1.0.0"));
        assert!(!version_is_newer("1.0", "1.0.0"));
        assert!(!version_is_newer("0.9.9", "1.0.0"));
        assert!(version_is_newer("2.0.0-beta", "1.5.0"));
    }
}
