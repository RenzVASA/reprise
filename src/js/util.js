// Petits outils partagés par les fenêtres.

export const esc = (s) =>
  String(s ?? "").replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[c]));

const P = {
  app: '<rect x="3.5" y="3.5" width="7" height="7" rx="2"/><rect x="13.5" y="3.5" width="7" height="7" rx="2"/><rect x="3.5" y="13.5" width="7" height="7" rx="2"/><rect x="13.5" y="13.5" width="7" height="7" rx="2"/>',
  tab: '<rect x="3" y="4.5" width="18" height="15" rx="2.5"/><path d="M3 9h18M7 4.5v4.5"/>',
  folder: '<path d="M3.5 7.5a2 2 0 0 1 2-2h3.6l2 2.2h7.4a2 2 0 0 1 2 2v7.8a2 2 0 0 1-2 2h-13a2 2 0 0 1-2-2z"/>',
  document: '<path d="M7 3.5h7l4.5 4.5v11a1.5 1.5 0 0 1-1.5 1.5H7A1.5 1.5 0 0 1 5.5 19V5A1.5 1.5 0 0 1 7 3.5z"/><path d="M13.5 3.5V8.5h5"/>',
  pin: '<path d="M9 3.5h6l-1 5.5 3.5 3.5h-11L10 9z"/><path d="M12 12.5V20.5"/>',
  trash: '<path d="M4.5 7h15M10 3.5h4M6.5 7l1 12.5a1.5 1.5 0 0 0 1.5 1.5h6a1.5 1.5 0 0 0 1.5-1.5L17.5 7"/>',
  refresh: '<path d="M19.5 12a7.5 7.5 0 1 1-2.2-5.3"/><path d="M19.5 4v4.5H15"/>',
  gear: '<path d="M4 7h9M17 7h3M4 17h3M11 17h9"/><circle cx="15" cy="7" r="2.2"/><circle cx="9" cy="17" r="2.2"/>',
  search: '<circle cx="10.5" cy="10.5" r="6"/><path d="M15 15l5 5"/>',
  close: '<path d="M6 6l12 12M18 6L6 18"/>',
  check: '<path d="M5 12.5l4.5 4.5L19 7.5"/>',
  edit: '<path d="M4 20h4L19 9l-4-4L4 16z"/><path d="M13.5 6.5l4 4"/>',
  open: '<path d="M13.5 4.5h6v6M19.5 4.5L11 13M17 14v4.5a1.5 1.5 0 0 1-1.5 1.5h-10A1.5 1.5 0 0 1 4 18.5v-10A1.5 1.5 0 0 1 5.5 7H10"/>',
  warn: '<path d="M12 4l9 16H3z"/><path d="M12 10v4.5M12 17.5v.01"/>',
  info: '<circle cx="12" cy="12" r="8.5"/><path d="M12 11v5.5M12 7.8v.01"/>',
  download: '<path d="M12 4v11M7.5 10.5L12 15l4.5-4.5M5 19.5h14"/>',
  plus: '<path d="M12 5v14M5 12h14"/>',
  plusCircle: '<circle cx="12" cy="12" r="8.5"/><path d="M12 8.5v7M8.5 12h7"/>',
};

export const icon = (name, cls = "") => `<svg class="icon ${cls}" viewBox="0 0 24 24" aria-hidden="true">${P[name] || ""}</svg>`;

/** Le signe de reprise (‖:), dessiné en SVG. */
export const repeatSign = (cls = "") =>
  `<svg class="repeat-sign ${cls}" viewBox="0 0 48 64" aria-hidden="true"><rect x="4" y="6" width="11" height="52" rx="2" fill="currentColor"/><rect x="19.5" y="6" width="4" height="52" rx="1.5" fill="currentColor"/><circle class="dot" cx="35" cy="23" r="5.5"/><circle class="dot" cx="35" cy="41" r="5.5"/></svg>`;

const rtf = new Intl.RelativeTimeFormat("fr", { numeric: "auto" });
const dayFmt = new Intl.DateTimeFormat("fr-FR", { weekday: "long" });
const dateFmt = new Intl.DateTimeFormat("fr-FR", { day: "numeric", month: "long" });
const timeFmt = new Intl.DateTimeFormat("fr-FR", { hour: "2-digit", minute: "2-digit" });

export function ago(ms) {
  const diff = (ms - Date.now()) / 1000;
  const abs = Math.abs(diff);
  if (abs < 45) return "à l'instant";
  if (abs < 3600) return rtf.format(Math.round(diff / 60), "minute");
  if (abs < 86400) return rtf.format(Math.round(diff / 3600), "hour");
  if (abs < 86400 * 7) return rtf.format(Math.round(diff / 86400), "day");
  return "le " + dateFmt.format(new Date(ms));
}

export function when(ms) {
  const d = new Date(ms);
  const today = new Date();
  const sameDay = d.toDateString() === today.toDateString();
  const yest = new Date(today); yest.setDate(today.getDate() - 1);
  const day = sameDay ? "aujourd'hui" : d.toDateString() === yest.toDateString() ? "hier" : dayFmt.format(d) + " " + dateFmt.format(d);
  return `${day} à ${timeFmt.format(d).replace(":", " h ")}`;
}

/** Nom proposé pour un nouveau contexte : « Code, samedi matin ». */
export function suggestName(app) {
  const d = new Date();
  const h = d.getHours();
  const moment = h < 6 ? "dans la nuit" : h < 12 ? "matin" : h < 14 ? "midi" : h < 18 ? "après-midi" : "soir";
  const day = dayFmt.format(d);
  return `${app || "Contexte"}, ${day} ${moment}`;
}

export function bucket(ms) {
  const d = new Date(ms);
  const now = new Date();
  const start = new Date(now.getFullYear(), now.getMonth(), now.getDate()).getTime();
  if (ms >= start) return "Aujourd'hui";
  if (ms >= start - 86400000) return "Hier";
  if (ms >= start - 6 * 86400000) return "Cette semaine";
  if (d.getFullYear() === now.getFullYear()) return "Plus tôt cette année";
  return "Avant";
}

export function domain(url) {
  try { return new URL(url).hostname.replace(/^www\./, ""); } catch { return url; }
}

/** Couleur stable dérivée d'un texte (pour les pastilles d'initiales). */
export function hue(text) {
  let h = 0;
  for (const c of String(text)) h = (h * 31 + c.codePointAt(0)) % 360;
  return h;
}

export function initial(text) {
  const t = String(text || "?").replace(/^www\./, "").trim();
  return (t.match(/[\p{L}\p{N}]/u) || ["?"])[0].toUpperCase();
}

/** "Alt+Command+S" → "⌥⌘S" */
export function prettyShortcut(s) {
  const mods = new Set();
  let key = "";
  for (const part of String(s || "").split("+")) {
    const p = part.trim().toLowerCase();
    if (p === "control" || p === "ctrl") mods.add("⌃");
    else if (p === "alt" || p === "option") mods.add("⌥");
    else if (p === "shift") mods.add("⇧");
    else if (["command", "cmd", "super", "meta", "commandorcontrol", "cmdorctrl"].includes(p)) mods.add("⌘");
    else key = p.replace(/^key/, "").replace(/^digit/, "").toUpperCase();
  }
  return ["⌃", "⌥", "⇧", "⌘"].filter((m) => mods.has(m)).join("") + key;
}

export function plural(n, one, many) {
  return `${n} ${n > 1 ? many : one}`;
}

export function debounce(fn, ms) {
  let t;
  return (...a) => { clearTimeout(t); t = setTimeout(() => fn(...a), ms); };
}
