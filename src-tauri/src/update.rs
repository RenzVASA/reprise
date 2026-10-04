//! Recherche de nouvelle version sur GitHub (Releases du dépôt RenzVASA/reprise).
//! On passe par `curl`, présent sur tous les Mac : pas besoin d'ajouter une bibliothèque réseau.
//! Rien n'est installé automatiquement : on propose juste d'ouvrir la page de téléchargement.

use crate::model::version_is_newer;
use serde::Serialize;
use std::process::Command;

pub const REPO: &str = "RenzVASA/reprise";

#[derive(Debug, Clone, Serialize)]
pub struct UpdateInfo {
    pub current: String,
    pub latest: String,
    pub available: bool,
    /// Page de la Release sur GitHub.
    pub url: String,
    /// Lien direct vers le .dmg, s'il existe.
    pub dmg_url: Option<String>,
    /// Notes de version (texte de la Release).
    pub notes: String,
    /// La mise à jour peut s'installer toute seule (sinon : page de téléchargement).
    pub can_install: bool,
}

pub fn check(current: &str) -> Result<UpdateInfo, String> {
    let api = format!("https://api.github.com/repos/{REPO}/releases/latest");
    let out = Command::new("/usr/bin/curl")
        .args([
            "-sSL",
            "--max-time",
            "15",
            "-H",
            "Accept: application/vnd.github+json",
            "-H",
            "User-Agent: Reprise",
            "-w",
            "\n%{http_code}",
            &api,
        ])
        .output()
        .map_err(|e| format!("Impossible de lancer curl : {e}"))?;
    if !out.status.success() {
        return Err("Pas de connexion à Internet, ou GitHub ne répond pas.".into());
    }
    let text = String::from_utf8_lossy(&out.stdout).into_owned();
    let (body, code) = text.rsplit_once('\n').unwrap_or((&text, ""));
    match code.trim() {
        "200" => {}
        "404" => return Err("Aucune version n'est encore publiée sur GitHub.".into()),
        "403" | "429" => return Err("GitHub limite les vérifications pour l'instant. Réessaie dans une heure.".into()),
        c => return Err(format!("GitHub a répondu avec une erreur ({c}).")),
    }
    parse_release(body, current)
}

pub fn parse_release(json: &str, current: &str) -> Result<UpdateInfo, String> {
    let v: serde_json::Value = serde_json::from_str(json).map_err(|_| "Réponse de GitHub illisible.".to_string())?;
    let latest = v["tag_name"].as_str().unwrap_or("").trim().to_string();
    if latest.is_empty() {
        return Err("Réponse de GitHub incomplète.".into());
    }
    let url = v["html_url"]
        .as_str()
        .filter(|u| u.starts_with("https://github.com/"))
        .map(String::from)
        .unwrap_or_else(|| format!("https://github.com/{REPO}/releases/latest"));
    let dmg_url = v["assets"].as_array().and_then(|assets| {
        assets
            .iter()
            .filter_map(|a| a["browser_download_url"].as_str())
            .find(|u| u.ends_with(".dmg") && u.starts_with("https://github.com/"))
            .map(String::from)
    });
    let notes = v["body"].as_str().unwrap_or("").chars().take(2000).collect();
    Ok(UpdateInfo {
        available: version_is_newer(&latest, current),
        current: current.to_string(),
        latest: latest.trim_start_matches(['v', 'V']).to_string(),
        url,
        dmg_url,
        notes,
        can_install: false,
    })
}
