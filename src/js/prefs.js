// Applique police, taille, mode clair/sombre et habillage à la fenêtre, et suit les changements.
import { api, on, inApp } from "./api.js";

/** Les habillages de saison et leurs dates (mois de 1 à 12, jours inclus). */
const SEASONS = [
  { style: "halloween", from: [10, 15], to: [11, 2] },
  { style: "noel", from: [12, 1], to: [1, 6] },
];

const md = (m, d) => m * 100 + d;

/** L'habillage de saison du jour, ou null. */
export function seasonFor(date = new Date()) {
  const today = md(date.getMonth() + 1, date.getDate());
  for (const s of SEASONS) {
    const a = md(...s.from), b = md(...s.to);
    const inside = a <= b ? today >= a && today <= b : today >= a || today <= b;
    if (inside) return s.style;
  }
  return null;
}

/** L'habillage à afficher : la saison si elle est activée, sinon celui choisi. */
export function effectiveStyle(s, date = new Date()) {
  if (!s) return "classique";
  return (s.seasonal !== false && seasonFor(date)) || s.style || "classique";
}

let lastIcon = null;
let current = null;

export function applyPrefs(s) {
  if (!s) return;
  current = s;
  const root = document.documentElement;
  root.dataset.font = s.font || "atkinson";
  root.style.setProperty("--scale", String((s.text_scale || 100) / 100));
  if (s.theme === "light" || s.theme === "dark") root.dataset.theme = s.theme;
  else delete root.dataset.theme;
  const style = effectiveStyle(s);
  if (style === "classique") delete root.dataset.style;
  else root.dataset.style = style;
  // L'icône du Dock suit l'habillage. Seule la fenêtre principale s'en occupe.
  if (inApp && document.body?.dataset.window === "main" && style !== lastIcon) {
    lastIcon = style;
    api.setAppIcon(style).catch(() => { lastIcon = null; });
  }
}

export async function initPrefs() {
  let s = null;
  try { s = await api.settings(); applyPrefs(s); } catch { /* valeurs par défaut */ }
  on("settings-changed", applyPrefs);
  // Le jour change (Halloween commence à minuit…) : on revérifie de temps en temps.
  setInterval(() => applyPrefs(current), 30 * 60 * 1000);
  return s;
}
