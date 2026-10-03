// Applique police, taille et thème à la fenêtre, et suit les changements.
import { api, on } from "./api.js";

export function applyPrefs(s) {
  if (!s) return;
  const root = document.documentElement;
  root.dataset.font = s.font || "atkinson";
  root.style.setProperty("--scale", String((s.text_scale || 100) / 100));
  if (s.theme === "light" || s.theme === "dark") root.dataset.theme = s.theme;
  else delete root.dataset.theme;
}

export async function initPrefs() {
  let s = null;
  try { s = await api.settings(); applyPrefs(s); } catch { /* valeurs par défaut */ }
  on("settings-changed", applyPrefs);
  return s;
}
