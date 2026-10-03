//! Stockage des contextes : un simple fichier JSON dans le dossier de données de l'app
//! (~/Library/Application Support/io.github.renzvasa.reprise/contexts.json).
//! Écriture atomique (fichier temporaire puis renommage) pour ne jamais rien perdre.

use crate::model::{now_ms, Context, ContextPatch};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Default, Serialize, Deserialize)]
struct FileFormat {
    version: u32,
    contexts: Vec<Context>,
}

pub struct Store {
    path: PathBuf,
    pub contexts: Vec<Context>,
}

pub fn write_atomic(path: &Path, data: &[u8]) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, data)?;
    fs::rename(&tmp, path)
}

impl Store {
    pub fn load(path: PathBuf) -> Store {
        let contexts = match fs::read_to_string(&path) {
            Ok(txt) => match serde_json::from_str::<FileFormat>(&txt) {
                Ok(f) => f.contexts,
                Err(_) => {
                    // Fichier abîmé : on le met de côté au lieu de l'écraser.
                    let _ = fs::copy(&path, path.with_extension(format!("json.abime-{}", now_ms())));
                    Vec::new()
                }
            },
            Err(_) => Vec::new(),
        };
        Store { path, contexts }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn save(&self) -> Result<(), String> {
        let data = serde_json::to_vec_pretty(&FileFormat { version: 1, contexts: self.contexts.clone() })
            .map_err(|e| e.to_string())?;
        write_atomic(&self.path, &data).map_err(|e| format!("Enregistrement impossible : {e}"))
    }

    /// Du plus récent au plus ancien.
    pub fn list(&self) -> Vec<Context> {
        let mut v = self.contexts.clone();
        v.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        v
    }

    pub fn get(&self, id: &str) -> Option<&Context> {
        self.contexts.iter().find(|c| c.id == id)
    }

    pub fn get_mut(&mut self, id: &str) -> Option<&mut Context> {
        self.contexts.iter_mut().find(|c| c.id == id)
    }

    pub fn insert(&mut self, ctx: Context) -> Result<(), String> {
        self.contexts.retain(|c| c.id != ctx.id);
        self.contexts.push(ctx);
        self.save()
    }

    pub fn update(&mut self, id: &str, patch: ContextPatch) -> Result<Context, String> {
        let c = self.get_mut(id).ok_or("Ce contexte n'existe plus.")?;
        if let Some(n) = patch.name {
            let n = n.trim();
            if !n.is_empty() {
                c.name = n.to_string();
            }
        }
        if let Some(note) = patch.note {
            c.note = note.trim().to_string();
        }
        if let Some(p) = patch.pinned {
            c.pinned = p;
        }
        c.updated_at = now_ms();
        let out = c.clone();
        self.save()?;
        Ok(out)
    }

    pub fn delete(&mut self, id: &str) -> Result<Context, String> {
        let pos = self.contexts.iter().position(|c| c.id == id).ok_or("Ce contexte n'existe plus.")?;
        let removed = self.contexts.remove(pos);
        self.save()?;
        Ok(removed)
    }

    pub fn mark_restored(&mut self, id: &str) -> Result<(), String> {
        if let Some(c) = self.get_mut(id) {
            c.last_restored_at = Some(now_ms());
            c.restore_count += 1;
        }
        self.save()
    }

    /// Les contextes à proposer dans la barre des menus : épinglés puis récents.
    pub fn recent(&self, n: usize) -> Vec<Context> {
        let mut v = self.list();
        v.sort_by(|a, b| {
            b.pinned
                .cmp(&a.pinned)
                .then(b.last_restored_at.unwrap_or(b.created_at).cmp(&a.last_restored_at.unwrap_or(a.created_at)))
        });
        v.truncate(n);
        v
    }
}
