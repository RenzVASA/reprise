// Fenêtre principale de Reprise.
import { api, on } from "./api.js";
import { applyPrefs, initPrefs } from "./prefs.js";
import { esc, icon, repeatSign, ago, when, bucket, domain, hue, initial, prettyShortcut, plural } from "./util.js";

const $ = (s, el = document) => el.querySelector(s);

const state = {
  contexts: [],
  selected: null,
  query: "",
  /** Éléments décochés, par contexte : Map<idContexte, Set<idÉlément>> */
  off: new Map(),
  /** « Faire place nette » choisi à la main pour ce contexte (sinon : réglage par défaut). */
  tidy: new Map(),
  editingNote: false,
  restoring: false,
  settings: null,
};

const listEl = $("#list");
const detailEl = $("#detail");

// ───────────────────────────── Données ─────────────────────────────

async function load() {
  try {
    state.contexts = await api.list();
  } catch (e) {
    toast(String(e));
    state.contexts = [];
  }
  const vis = visible();
  if (!vis.some((c) => c.id === state.selected)) state.selected = vis[0]?.id ?? null;
  render();
}

function haystack(c) {
  const parts = [c.name, c.note, c.front_app || ""];
  for (const it of c.items) parts.push(it.label, it.value, it.app_name);
  return parts.join("\n").toLowerCase();
}

function visible() {
  const q = state.query.trim().toLowerCase();
  const words = q.split(/\s+/).filter(Boolean);
  return state.contexts
    .filter((c) => !words.length || words.every((w) => haystack(c).includes(w)))
    .sort((a, b) => (b.pinned - a.pinned) || (b.created_at - a.created_at));
}

const current = () => state.contexts.find((c) => c.id === state.selected) || null;
const offSet = (id) => {
  if (!state.off.has(id)) state.off.set(id, new Set());
  return state.off.get(id);
};
const counts = (c) => {
  const k = { app: 0, tab: 0, folder: 0, document: 0 };
  for (const it of c.items) k[it.kind]++;
  return k;
};

// ───────────────────────────── Rendu ───────────────────────────────

function render() {
  renderList();
  renderDetail();
}

function rowHtml(c) {
  const k = counts(c);
  const meta = [
    k.app && `<span class="count" title="${plural(k.app, "app", "apps")}">${icon("app")}${k.app}</span>`,
    k.tab && `<span class="count" title="${plural(k.tab, "onglet", "onglets")}">${icon("tab")}${k.tab}</span>`,
    (k.folder + k.document) && `<span class="count" title="Dossiers et fichiers">${icon("folder")}${k.folder + k.document}</span>`,
  ].filter(Boolean).join("");
  return `
    <button class="row ${c.id === state.selected ? "selected" : ""}" data-id="${esc(c.id)}" aria-current="${c.id === state.selected}">
      ${c.pinned ? '<span class="ribbon-flag" title="Épinglé"></span>' : ""}
      <div class="row-name">${esc(c.name)}</div>
      ${c.note ? `<div class="row-note"><span class="marker">${esc(c.note)}</span></div>` : ""}
      <div class="row-meta">${meta}<span class="time">${esc(ago(c.created_at))}</span></div>
    </button>`;
}

function renderList() {
  const vis = visible();
  if (!state.contexts.length) {
    listEl.innerHTML = `<p class="list-empty">Tes contextes sauvegardés apparaîtront ici.</p>`;
    return;
  }
  if (!vis.length) {
    listEl.innerHTML = `<p class="list-empty">Rien ne correspond à « ${esc(state.query)} ».</p>`;
    return;
  }
  let html = "";
  if (state.query.trim()) {
    html += `<h2 class="group-title">${plural(vis.length, "résultat", "résultats")}</h2>` + vis.map(rowHtml).join("");
  } else {
    const pinned = vis.filter((c) => c.pinned);
    if (pinned.length) html += `<h2 class="group-title">Épinglés</h2>` + pinned.map(rowHtml).join("");
    let last = null;
    for (const c of vis.filter((c) => !c.pinned)) {
      const b = bucket(c.created_at);
      if (b !== last) { html += `<h2 class="group-title">${b}</h2>`; last = b; }
      html += rowHtml(c);
    }
  }
  listEl.innerHTML = html;
}

function avatar(text, round = false) {
  return `<span class="avatar ${round ? "round" : ""}" style="--h:${hue(text)}">${esc(initial(text))}</span>`;
}

function itemRow(ctx, it, sub) {
  const off = offSet(ctx.id).has(it.id);
  const lead = it.kind === "tab" ? avatar(domain(it.value), true)
    : `<span class="avatar glyph">${icon(it.kind === "folder" ? "folder" : "document")}</span>`;
  return `
    <li class="item ${off ? "off" : ""}">
      <label>
        <input type="checkbox" data-item="${esc(it.id)}" ${off ? "" : "checked"}>
        ${lead}
        <span class="item-text">
          <span class="item-title" title="${esc(it.label)}">${esc(it.label)}</span>
          <span class="item-sub" title="${esc(it.value)}">${esc(sub)}</span>
        </span>
      </label>
      <button class="btn ghost item-open" data-action="open-item" data-item="${esc(it.id)}">Ouvrir</button>
    </li>`;
}

function section(ctx, kind, title, body, total) {
  const off = offSet(ctx.id);
  const allOff = ctx.items.filter((i) => i.kind === kind).every((i) => off.has(i.id));
  return `
    <section class="section">
      <div class="section-head">
        <h2 class="section-title">${title}</h2><span class="section-count">${total}</span>
        <button class="section-toggle" data-action="toggle-kind" data-kind="${kind}">${allOff ? "Tout cocher" : "Tout décocher"}</button>
      </div>
      ${body}
    </section>`;
}

function shortPath(p) {
  return p.replace(/^\/Users\/[^/]+/, "~");
}

function renderDetail() {
  const c = current();
  if (!state.contexts.length) {
    const sc = prettyShortcut(state.settings?.shortcut_save || "Alt+Command+S");
    detailEl.innerHTML = `
      <div class="empty"><div class="empty-card">
        ${repeatSign("empty-sign")}
        <h2>Aucun contexte pour l'instant</h2>
        <p>Pendant que tu travailles, appuie sur ce raccourci. Reprise photographie tes apps, tes onglets, tes dossiers et tes fichiers ouverts, et te demande où tu en étais.</p>
        <div class="keys">${[...sc].map((k) => `<span class="key">${esc(k)}</span>`).join("")}</div>
        <div><button class="btn primary" data-action="capture">Sauvegarder ce qui est ouvert maintenant</button></div>
      </div></div>`;
    return;
  }
  if (!c) {
    detailEl.innerHTML = `<div class="empty"><div class="empty-card"><p>Choisis un contexte dans la liste.</p></div></div>`;
    return;
  }

  const off = offSet(c.id);
  const byKind = (k) => c.items.filter((i) => i.kind === k);
  const apps = byKind("app"), tabs = byKind("tab"), folders = byKind("folder"), docs = byKind("document");
  const selectedCount = c.items.filter((i) => !off.has(i.id)).length;
  const tidy = state.tidy.has(c.id) ? state.tidy.get(c.id) : !!state.settings?.tidy_by_default;

  const metaBits = [`Sauvegardé ${when(c.created_at)}`];
  if (c.updated_at - c.created_at > 60000) metaBits.push(`mis à jour ${ago(c.updated_at)}`);
  if (c.restore_count) metaBits.push(`repris ${c.restore_count === 1 ? "une fois" : c.restore_count + " fois"}`);

  let noteHtml;
  if (state.editingNote) {
    noteHtml = `
      <textarea id="note-edit" class="field note-edit" placeholder="Ce que tu faisais, la prochaine étape, ce qui bloquait…">${esc(c.note)}</textarea>
      <div class="note-edit-actions">
        <button class="btn primary" data-action="save-note">Enregistrer</button>
        <button class="btn ghost" data-action="cancel-note">Annuler</button>
        <span class="muted small"><kbd>⌘</kbd> <kbd>↵</kbd> pour enregistrer</span>
      </div>`;
  } else if (c.note) {
    noteHtml = `<button class="note-text" data-action="edit-note" title="Modifier la note"><span class="marker">${esc(c.note)}</span></button>`;
  } else {
    noteHtml = `<button class="note-empty" data-action="edit-note">Ajoute une phrase pour savoir quoi faire en revenant.</button>`;
  }

  // Onglets regroupés par navigateur puis par fenêtre.
  let tabsBody = "";
  if (tabs.length) {
    const browsers = [...new Set(tabs.map((t) => t.app_name))];
    for (const b of browsers) {
      const bt = tabs.filter((t) => t.app_name === b);
      const wins = [...new Set(bt.map((t) => t.group ?? 1))];
      wins.forEach((w, i) => {
        const label = browsers.length > 1 || wins.length > 1
          ? `${b}${wins.length > 1 ? `, fenêtre ${i + 1}` : ""}` : "";
        if (label) tabsBody += `<div class="subhead">${esc(label)}</div>`;
        tabsBody += `<ul class="items">${bt.filter((t) => (t.group ?? 1) === w).map((t) => itemRow(c, t, domain(t.value))).join("")}</ul>`;
      });
    }
  }

  detailEl.innerHTML = `
    <div class="detail-inner">
      <h1 class="title" id="title" contenteditable="plaintext-only" spellcheck="false" title="Clique pour renommer">${esc(c.name)}</h1>
      <p class="meta">${esc(metaBits.join(", "))}.</p>

      <div class="note-block">
        <div class="note-label">Où tu en étais</div>
        ${noteHtml}
      </div>

      <div class="resume-bar">
        <button class="btn primary resume" data-action="restore" ${selectedCount ? "" : "disabled"}>
          ${repeatSign()}<span>${state.restoring ? "Reprise en cours…" : "Reprendre"}</span>
        </button>
        <div class="resume-side">
          <label class="switch">
            <input type="checkbox" id="tidy" ${tidy ? "checked" : ""}>
            <span class="track"></span>
            <span>Faire place nette</span>
          </label>
          <span class="resume-count">${tidy ? "Les autres apps seront masquées. " : ""}${selectedCount === c.items.length ? `Tout sera rouvert (${plural(c.items.length, "élément", "éléments")}).` : `${selectedCount} élément${selectedCount > 1 ? "s" : ""} sur ${c.items.length} seront rouverts.`}</span>
        </div>
      </div>

      ${apps.length ? section(c, "app", "Applications", `<div class="apps">${apps.map((a) => `
          <label class="app-tile ${off.has(a.id) ? "off" : ""}">
            <input type="checkbox" data-item="${esc(a.id)}" ${off.has(a.id) ? "" : "checked"}>
            ${avatar(a.label)}<span>${esc(a.label)}</span>
          </label>`).join("")}</div>`, apps.length) : ""}

      ${tabs.length ? section(c, "tab", "Onglets", tabsBody, tabs.length) : ""}

      ${folders.length ? section(c, "folder", "Dossiers", `<ul class="items">${folders.map((f) => itemRow(c, f, shortPath(f.value))).join("")}</ul>`, folders.length) : ""}

      ${docs.length ? section(c, "document", "Fichiers", `<ul class="items">${docs.map((d) => itemRow(c, d, `${d.app_name}, ${shortPath(d.value)}`)).join("")}</ul>`, docs.length) : ""}

      ${!c.items.length ? `<p class="muted" style="margin-top:2rem">Ce contexte ne contient rien à rouvrir. Mets-le à jour pendant que tes apps sont ouvertes.</p>` : ""}

      <div class="footer-actions">
        <button class="btn ghost" data-action="pin">${icon("pin")}${c.pinned ? "Désépingler" : "Épingler"}</button>
        <button class="btn ghost" data-action="recapture" title="Remplacer le contenu par ce qui est ouvert maintenant">${icon("refresh")}Mettre à jour avec ce qui est ouvert</button>
        <span class="spacer"></span>
        <button class="btn ghost danger" data-action="delete">${icon("trash")}Supprimer</button>
      </div>
    </div>`;

  if (state.editingNote) {
    const ta = $("#note-edit");
    ta.focus();
    ta.setSelectionRange(ta.value.length, ta.value.length);
  }
}

// ───────────────────────────── Actions ─────────────────────────────

function select(id) {
  if (id === state.selected) return;
  state.selected = id;
  state.editingNote = false;
  detailEl.scrollTop = 0;
  render();
}

async function restore() {
  const c = current();
  if (!c || state.restoring) return;
  const off = offSet(c.id);
  const items = off.size ? c.items.filter((i) => !off.has(i.id)).map((i) => i.id) : null;
  const tidy = state.tidy.has(c.id) ? state.tidy.get(c.id) : null;
  state.restoring = true;
  renderDetail();
  try {
    const r = await api.restore(c.id, items, tidy);
    if (r.failures.length) {
      toast(r.failures.length === 1 ? r.failures[0] : `${r.failures[0]} (et ${r.failures.length - 1} autre${r.failures.length > 2 ? "s" : ""} souci${r.failures.length > 2 ? "s" : ""})`, { timeout: 7000 });
    }
  } catch (e) {
    toast(String(e));
  } finally {
    state.restoring = false;
    renderDetail();
  }
}

async function saveNote() {
  const c = current();
  const ta = $("#note-edit");
  if (!c || !ta) return;
  const note = ta.value;
  state.editingNote = false;
  try {
    const u = await api.update(c.id, { note });
    Object.assign(c, u);
  } catch (e) { toast(String(e)); }
  render();
}

async function rename(name) {
  const c = current();
  if (!c) return;
  const clean = name.replace(/\s+/g, " ").trim();
  if (!clean) { const t = $("#title"); if (t) t.textContent = c.name; return; }
  if (clean === c.name) return;
  try {
    const u = await api.update(c.id, { name: clean });
    Object.assign(c, u);
  } catch (e) { toast(String(e)); }
  render();
}

async function togglePin() {
  const c = current();
  if (!c) return;
  const u = await api.update(c.id, { pinned: !c.pinned });
  Object.assign(c, u);
  render();
}

async function remove() {
  const c = current();
  if (!c) return;
  const vis = visible();
  const idx = vis.findIndex((x) => x.id === c.id);
  try {
    const removed = await api.remove(c.id);
    state.contexts = state.contexts.filter((x) => x.id !== c.id);
    state.selected = (vis[idx + 1] || vis[idx - 1])?.id ?? null;
    render();
    toast(`« ${removed.name} » a été supprimé.`, {
      action: "Annuler",
      onAction: async () => {
        await api.putBack(removed);
        state.selected = removed.id;
        await load();
      },
    });
  } catch (e) { toast(String(e)); }
}

function toggleKind(kind) {
  const c = current();
  const off = offSet(c.id);
  const ids = c.items.filter((i) => i.kind === kind).map((i) => i.id);
  const allOff = ids.every((id) => off.has(id));
  ids.forEach((id) => (allOff ? off.delete(id) : off.add(id)));
  renderDetail();
}

// ───────────────────────────── Bulle ───────────────────────────────

let toastTimer;
function toast(msg, { action, onAction, timeout = 5000 } = {}) {
  const el = $("#toast");
  el.innerHTML = `<span>${esc(msg)}</span>${action ? `<button class="btn">${esc(action)}</button>` : ""}`;
  el.hidden = false;
  if (action) {
    el.querySelector("button").onclick = () => { el.hidden = true; onAction?.(); };
  }
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => (el.hidden = true), timeout);
}

// ───────────────────────────── Événements ──────────────────────────

listEl.addEventListener("click", (e) => {
  const row = e.target.closest(".row");
  if (row) select(row.dataset.id);
});

detailEl.addEventListener("click", (e) => {
  const btn = e.target.closest("[data-action]");
  if (!btn) return;
  const c = current();
  switch (btn.dataset.action) {
    case "restore": restore(); break;
    case "edit-note": state.editingNote = true; renderDetail(); break;
    case "save-note": saveNote(); break;
    case "cancel-note": state.editingNote = false; renderDetail(); break;
    case "pin": togglePin(); break;
    case "delete": remove(); break;
    case "recapture": api.startCapture(c.id); break;
    case "capture": api.startCapture(); break;
    case "toggle-kind": toggleKind(btn.dataset.kind); break;
    case "open-item":
      api.openItem(c.id, btn.dataset.item).then((r) => r.failures.length && toast(r.failures[0])).catch((err) => toast(String(err)));
      break;
  }
});

detailEl.addEventListener("change", (e) => {
  const c = current();
  if (!c) return;
  if (e.target.id === "tidy") {
    state.tidy.set(c.id, e.target.checked);
    renderDetail();
  } else if (e.target.dataset.item) {
    const off = offSet(c.id);
    e.target.checked ? off.delete(e.target.dataset.item) : off.add(e.target.dataset.item);
    renderDetail();
  }
});

detailEl.addEventListener("keydown", (e) => {
  if (e.target.id === "title") {
    if (e.key === "Enter") { e.preventDefault(); e.target.blur(); }
    if (e.key === "Escape") { e.target.textContent = current()?.name ?? ""; e.target.blur(); }
  }
  if (e.target.id === "note-edit") {
    if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) { e.preventDefault(); saveNote(); }
    if (e.key === "Escape") { e.stopPropagation(); state.editingNote = false; renderDetail(); }
  }
});

detailEl.addEventListener("focusout", (e) => {
  if (e.target.id === "title") rename(e.target.textContent);
});

const searchEl = $("#search");
searchEl.addEventListener("input", () => {
  state.query = searchEl.value;
  const vis = visible();
  if (!vis.some((c) => c.id === state.selected)) state.selected = vis[0]?.id ?? null;
  render();
});
searchEl.addEventListener("keydown", (e) => {
  if (e.key === "Escape") { searchEl.value = ""; state.query = ""; render(); searchEl.blur(); }
  if (e.key === "ArrowDown") { e.preventDefault(); searchEl.blur(); moveSelection(0); }
  if (e.key === "Enter") { e.preventDefault(); restore(); }
});

function moveSelection(delta) {
  const vis = visible();
  if (!vis.length) return;
  const i = vis.findIndex((c) => c.id === state.selected);
  const next = vis[Math.max(0, Math.min(vis.length - 1, (i < 0 ? 0 : i) + delta))];
  select(next.id);
  listEl.querySelector(`[data-id="${CSS.escape(next.id)}"]`)?.scrollIntoView({ block: "nearest" });
}

document.addEventListener("keydown", (e) => {
  const typing = e.target.closest("input, textarea, [contenteditable='plaintext-only'], [contenteditable='true']");
  const dialogOpen = document.querySelector("dialog[open]");
  if (dialogOpen) return;
  if (e.metaKey && e.key === "f") { e.preventDefault(); searchEl.focus(); searchEl.select(); return; }
  if (e.metaKey && e.key === ",") { e.preventDefault(); openSettings(); return; }
  if (e.metaKey && e.key === "n") { e.preventDefault(); api.startCapture(); return; }
  if (e.key === "?" && !typing) { e.preventDefault(); openInfo(); return; }
  if (typing) return;
  if (e.key === "ArrowDown") { e.preventDefault(); moveSelection(1); }
  else if (e.key === "ArrowUp") { e.preventDefault(); moveSelection(-1); }
  else if (e.key === "Enter" && !e.target.closest("button")) { e.preventDefault(); restore(); }
  else if (e.metaKey && e.key === "Backspace") { e.preventDefault(); remove(); }
});

$("#btn-capture").addEventListener("click", () => api.startCapture());
$("#btn-settings").addEventListener("click", () => openSettings());
$("#btn-info").addEventListener("click", () => openInfo());

on("contexts-changed", () => load());
on("context-saved", (id) => { state.selected = id; state.query = ""; searchEl.value = ""; load(); });
on("toast", (msg) => toast(String(msg)));
on("settings-changed", (s) => { state.settings = s; updateShortcutHints(); renderDetail(); });
window.addEventListener("focus", () => load());

function updateShortcutHints() {
  $("#kbd-save").textContent = prettyShortcut(state.settings?.shortcut_save || "Alt+Command+S");
}

// ───────────────────────────── Réglages ────────────────────────────

const settingsEl = $("#settings");
let recording = null;
let settingsError = "";
let perms = null;
let info = null;

const FONTS = [
  ["atkinson", "Atkinson Hyperlegible", "'Atkinson'"],
  ["lexend", "Lexend", "'Lexend'"],
  ["opendyslexic", "OpenDyslexic", "'OpenDyslexic'"],
  ["system", "Police du système", "-apple-system, system-ui"],
];

async function openSettings() {
  settingsError = "";
  try { state.settings = await api.settings(); } catch { /* garde la valeur connue */ }
  try { info = await api.info(); } catch { info = null; }
  renderSettings();
  if (!settingsEl.open) settingsEl.showModal();
}

function permLine(kind, label, why) {
  const st = perms?.[kind];
  const txt = st === "ok" ? "Autorisé" : st === "denied" ? "Refusé" : "Pas encore vérifié";
  return `
    <div class="set-row">
      <div class="label"><span>${label}</span><span class="hint">${why}</span></div>
      <div class="perm"><span class="perm-dot ${st || ""}"></span><span class="small">${txt}</span>
        <button class="btn ghost" data-set="open-pane" data-kind="${kind}">Ouvrir</button></div>
    </div>`;
}

function toggleRow(key, label, hint) {
  const s = state.settings;
  return `
    <div class="set-row">
      <div class="label"><span>${label}</span>${hint ? `<span class="hint">${hint}</span>` : ""}</div>
      <label class="switch"><input type="checkbox" data-toggle="${key}" ${s[key] ? "checked" : ""}><span class="track"></span></label>
    </div>`;
}

function renderSettings() {
  const s = state.settings;
  if (!s) return;
  const rec = (key, label) => `
    <div class="set-row">
      <div class="label"><span>${label}</span></div>
      <button class="btn recorder ${recording === key ? "listening" : ""}" data-set="record" data-key="${key}">
        ${recording === key ? "Appuie sur ta combinaison…" : esc(prettyShortcut(s[key]))}
      </button>
    </div>`;
  settingsEl.innerHTML = `
    <div class="sheet-head">
      <h2 id="settings-title">Réglages</h2>
      <button class="btn ghost icon-only" data-set="close" aria-label="Fermer">${icon("close")}</button>
    </div>
    <div class="sheet-body">
      <div class="set-group">
        <h3>Raccourcis</h3>
        ${rec("shortcut_save", "Sauvegarder le contexte")}
        ${rec("shortcut_open", "Ouvrir Reprise")}
        ${settingsError ? `<div class="set-error">${esc(settingsError)}</div>` : ""}
      </div>

      <div class="set-group">
        <h3>Lecture</h3>
        <div class="fonts" role="radiogroup" aria-label="Police">
          ${FONTS.map(([id, name, family]) => `
            <button class="font-card" role="radio" aria-checked="${s.font === id}" data-set="font" data-value="${id}">
              <div class="name">${name}</div>
              <div class="sample" style="font-family:${family}">Il reste la conclusion à écrire.</div>
            </button>`).join("")}
        </div>
        <div class="set-row" style="margin-top:.6rem">
          <div class="label"><span>Taille du texte</span><span class="hint">${s.text_scale} %</span></div>
          <input type="range" min="90" max="140" step="5" value="${s.text_scale}" data-set="scale" aria-label="Taille du texte">
        </div>
        <div class="set-row">
          <div class="label"><span>Apparence</span></div>
          <div class="segmented" role="radiogroup" aria-label="Apparence">
            ${[["auto", "Auto"], ["light", "Clair"], ["dark", "Sombre"]].map(([v, l]) =>
              `<button role="radio" aria-checked="${s.theme === v}" data-set="theme" data-value="${v}">${l}</button>`).join("")}
          </div>
        </div>
      </div>

      <div class="set-group">
        <h3>Comportement</h3>
        ${toggleRow("tidy_by_default", "Faire place nette par défaut", "Masque les autres apps quand tu reprends un contexte.")}
        ${toggleRow("show_note", "Afficher le post-it « Tu en étais là »", "Un petit rappel de ta note, en haut à droite de l'écran.")}
        ${toggleRow("launch_at_login", "Lancer Reprise à l'ouverture de session", "Discrètement, dans la barre des menus.")}
        ${toggleRow("show_in_dock", "Icône dans le Dock", "Sinon, Reprise vit seulement dans la barre des menus.")}
      </div>

      <div class="set-group">
        <h3>Apps jamais enregistrées</h3>
        ${s.ignored_apps?.length ? `<ul class="ignored">${s.ignored_apps.map((a) => `
          <li><span class="avatar" style="--h:${hue(a.name)}">${esc(initial(a.name))}</span><span class="ig-name">${esc(a.name)}</span>
            <button class="btn ghost" data-set="unignore" data-bundle="${esc(a.bundle_id)}">Retirer</button></li>`).join("")}</ul>`
          : `<p class="hint small" style="margin:0">Aucune pour l'instant. Dans la fenêtre de sauvegarde, clique sur « 4 apps » puis sur « Ne jamais enregistrer » à côté d'une app (Spotify, Messages, Discord…).</p>`}
      </div>

      <div class="set-group">
        <h3>Autorisations macOS</h3>
        ${permLine("automation", "Automatisation", "Pour lire et rouvrir tes onglets, tes dossiers et tes apps.")}
        ${permLine("accessibility", "Accessibilité", "Pour savoir quels fichiers sont ouverts dans tes apps.")}
        <div class="set-row"><span class="hint">La vérification peut faire apparaître les demandes de macOS.</span>
          <button class="btn" data-set="check-perms">${perms === "loading" ? "Vérification…" : "Vérifier"}</button></div>
      </div>

      <div class="set-group">
        <h3>Mises à jour</h3>
        ${toggleRow("check_updates", "Chercher les mises à jour au démarrage", "Une fois par jour au plus, sur la page GitHub de Reprise. Rien d'autre n'est envoyé.")}
        <div class="set-row">
          <div class="label"><span>Version installée : ${esc(info?.version || "")}</span><span class="hint">${esc(updateLine())}</span></div>
          <button class="btn" data-set="check-update">${updateChecking ? "Vérification…" : "Vérifier maintenant"}</button>
        </div>
      </div>

      <div class="set-group">
        <h3>Tes données</h3>
        <p class="hint small" style="margin:0">Tout reste sur ton Mac, dans un simple fichier. Rien n'est envoyé sur Internet.</p>
        ${info ? `<div class="path">${esc(info.data_path)}</div>` : ""}
        <button class="btn" data-set="reveal">Montrer dans le Finder</button>
      </div>

      <div class="set-group">
        <h3>À propos</h3>
        <div class="about">
          <img src="assets/icon-180.png" alt="">
          <div>
            <p><strong>Reprise ${esc(info?.version || "")}</strong></p>
            <p>Application vibe codée par RenzVASA avec Claude, l'IA d'Anthropic. Licence MIT.</p>
            <button class="btn" data-set="info">Comment marche Reprise</button>
            <button class="btn" data-set="github">Code source sur GitHub</button>
          </div>
        </div>
      </div>
    </div>`;
}

async function saveSettings(patch) {
  const prev = state.settings;
  const next = { ...prev, ...patch };
  state.settings = next;
  applyPrefs(next);
  try {
    state.settings = await api.saveSettings(next);
    settingsError = "";
  } catch (e) {
    state.settings = prev;
    applyPrefs(prev);
    settingsError = String(e);
  }
  updateShortcutHints();
  renderSettings();
  renderDetail();
}

settingsEl.addEventListener("click", async (e) => {
  const t = e.target.closest("[data-set]");
  if (!t) return;
  switch (t.dataset.set) {
    case "close": settingsEl.close(); break;
    case "record":
      recording = recording === t.dataset.key ? null : t.dataset.key;
      settingsError = "";
      api.pauseShortcuts(Boolean(recording)).catch(() => {});
      renderSettings();
      break;
    case "font": saveSettings({ font: t.dataset.value }); break;
    case "theme": saveSettings({ theme: t.dataset.value }); break;
    case "check-perms":
      perms = "loading"; renderSettings();
      try { perms = await api.permissions(); } catch { perms = null; }
      renderSettings();
      break;
    case "open-pane": api.openPrivacy(t.dataset.kind).catch((err) => toast(String(err))); break;
    case "reveal": api.revealData().catch((err) => toast(String(err))); break;
    case "github": api.openLink("https://github.com/RenzVASA/reprise").catch(() => {}); break;
    case "info": settingsEl.close(); openInfo(); break;
    case "check-update": checkForUpdates(true); break;
    case "unignore":
      saveSettings({ ignored_apps: state.settings.ignored_apps.filter((a) => a.bundle_id !== t.dataset.bundle) });
      break;
  }
});

settingsEl.addEventListener("change", (e) => {
  const key = e.target.dataset.toggle;
  if (key) saveSettings({ [key]: e.target.checked });
  if (e.target.dataset.set === "scale") saveSettings({ text_scale: Number(e.target.value) });
});
settingsEl.addEventListener("input", (e) => {
  if (e.target.dataset.set === "scale") applyPrefs({ ...state.settings, text_scale: Number(e.target.value) });
});
settingsEl.addEventListener("close", () => {
  if (recording) { recording = null; api.pauseShortcuts(false).catch(() => {}); }
});

/** Enregistre une combinaison de touches pour un raccourci. */
document.addEventListener("keydown", (e) => {
  if (!recording || !settingsEl.open) return;
  e.preventDefault();
  e.stopPropagation();
  if (e.key === "Escape") {
    recording = null;
    api.pauseShortcuts(false).catch(() => {});
    renderSettings();
    return;
  }
  const key = /^(Key[A-Z]|Digit[0-9])$/.test(e.code) ? e.code : null;
  if (!key) return; // on attend une lettre ou un chiffre
  if (!(e.metaKey || e.ctrlKey || e.altKey)) {
    settingsError = "Ajoute au moins ⌘, ⌥ ou ⌃ à ta combinaison.";
    renderSettings();
    return;
  }
  const parts = [];
  if (e.ctrlKey) parts.push("Control");
  if (e.altKey) parts.push("Alt");
  if (e.shiftKey) parts.push("Shift");
  if (e.metaKey) parts.push("Command");
  parts.push(key);
  const field = recording;
  recording = null;
  api.pauseShortcuts(false).catch(() => {}).finally(() => saveSettings({ [field]: parts.join("+") }));
}, true);

// ───────────────────────────── Mises à jour ────────────────────────

let updateInfo = null;
let updateChecking = false;
let updateError = "";

function updateLine() {
  if (updateChecking) return "Recherche en cours…";
  if (updateError) return updateError;
  if (!updateInfo) return "Pas encore vérifié depuis le lancement.";
  return updateInfo.available ? `La version ${updateInfo.latest} est disponible.` : "Tu as la dernière version.";
}

function showUpdatePill() {
  const pill = $("#update-pill");
  if (updateInfo?.available) {
    pill.innerHTML = `${icon("download")}<span>Version ${esc(updateInfo.latest)} disponible</span>`;
    pill.hidden = false;
  } else {
    pill.hidden = true;
  }
}

async function checkForUpdates(manual) {
  updateChecking = true;
  updateError = "";
  if (settingsEl.open) renderSettings();
  if (infoEl.open) renderInfo();
  try {
    updateInfo = await api.checkUpdates();
    if (manual && !updateInfo.available && !settingsEl.open && !infoEl.open) toast(`Reprise est à jour (version ${updateInfo.current}).`);
    if (manual && updateInfo.available) openUpdate();
  } catch (e) {
    updateError = String(e);
    if (manual && !settingsEl.open && !infoEl.open) toast(updateError);
  } finally {
    updateChecking = false;
    showUpdatePill();
    if (settingsEl.open) renderSettings();
    if (infoEl.open) renderInfo();
  }
}

const updateEl = $("#update");

function openUpdate() {
  if (!updateInfo) return;
  const notes = (updateInfo.notes || "").split("\n").map((l) => l.trim()).filter((l) => /^[-*•]\s+/.test(l)).map((l) => l.replace(/^[-*•]\s+/, "").replace(/\*\*/g, ""));
  updateEl.innerHTML = `
    <div class="sheet-head">
      <h2 id="update-title">Reprise ${esc(updateInfo.latest)} est disponible</h2>
      <button class="btn ghost icon-only" data-u="close" aria-label="Fermer">${icon("close")}</button>
    </div>
    <div class="sheet-body">
      <p class="muted">Tu as la version ${esc(updateInfo.current)}.${notes.length ? " Nouveautés :" : ""}</p>
      ${notes.length ? `<ul class="notes">${notes.slice(0, 12).map((n) => `<li>${esc(n)}</li>`).join("")}</ul>` : ""}
      <ol class="how">
        <li>Télécharge le fichier <strong>.dmg</strong> sur la page de la version.</li>
        <li>Quitte Reprise, puis glisse la nouvelle version dans Applications pour remplacer l'ancienne. Tes contextes et tes réglages sont gardés.</li>
        <li>Dans le Terminal, une fois : <code>xattr -cr /Applications/Reprise.app</code></li>
      </ol>
      <div class="welcome-actions">
        <button class="btn ghost" data-u="close">Plus tard</button>
        <button class="btn primary" data-u="get">${icon("download")}Ouvrir la page de téléchargement</button>
      </div>
    </div>`;
  if (!updateEl.open) updateEl.showModal();
}

updateEl.addEventListener("click", (e) => {
  const t = e.target.closest("[data-u]");
  if (!t) return;
  if (t.dataset.u === "get") api.openLink(updateInfo.url).catch((err) => toast(String(err)));
  updateEl.close();
});
$("#update-pill").addEventListener("click", openUpdate);
on("update-available", (u) => { updateInfo = u; showUpdatePill(); });

// ───────────────────────────── Info ────────────────────────────────

const infoEl = $("#info");

async function openInfo() {
  if (!info) { try { info = await api.info(); } catch { info = null; } }
  renderInfo();
  if (!infoEl.open) infoEl.showModal();
}

function renderInfo() {
  const save = prettyShortcut(state.settings?.shortcut_save || "Alt+Command+S");
  const open = prettyShortcut(state.settings?.shortcut_open || "Alt+Command+R");
  infoEl.innerHTML = `
    <div class="sheet-head">
      <h2 id="info-title">Comment marche Reprise</h2>
      <button class="btn ghost icon-only" data-i="close" aria-label="Fermer">${icon("close")}</button>
    </div>
    <div class="sheet-body prose">
      <p class="lead">Reprise sert à <span class="marker">ne plus perdre le fil quand tu es interrompu</span>. Elle prend une photo de ce sur quoi tu travailles, avec une phrase pour te souvenir où tu en étais, et remet tout en place quand tu reviens.</p>

      <h3>En trois gestes</h3>
      <ol class="steps-list">
        <li><strong>Appuie sur ${esc(save)}</strong>, depuis n'importe quelle app. Une petite fenêtre s'ouvre.</li>
        <li><strong>Écris où tu en es</strong>, en une phrase. Tu peux cliquer sur les pastilles (« 4 apps », « 12 onglets »…) pour décocher ce que tu ne veux pas garder. Entrée, c'est sauvegardé.</li>
        <li><strong>Plus tard, clique sur Reprendre.</strong> Tout se rouvre, et ta phrase s'affiche sur un post-it en haut de l'écran.</li>
      </ol>

      <h3>Ce qui est enregistré</h3>
      <ul>
        <li>Les <strong>apps</strong> ouvertes (celles qui ont une icône dans le Dock).</li>
        <li>Les <strong>onglets</strong> de Safari, Chrome, Brave, Edge, Arc et Vivaldi, fenêtre par fenêtre.</li>
        <li>Les <strong>dossiers</strong> ouverts dans le Finder.</li>
        <li>Les <strong>fichiers</strong> ouverts dans tes apps (PDF, Pages, Keynote, VS Code…), si l'accès Accessibilité est autorisé.</li>
      </ul>
      <p class="muted">Ce qui ne l'est pas : ce qu'il y a <em>dans</em> les apps (la chanson en cours, un texte pas encore enregistré, la position dans une vidéo), les onglets de Firefox et souvent ceux de navigation privée. C'est pour ça que ta phrase compte.</p>

      <h3>Comment ça marche, en coulisses</h3>
      <p>Pour savoir ce qui est ouvert, Reprise pose la question à macOS en <strong>AppleScript</strong>, le langage intégré au Mac pour piloter les apps. macOS répond avec le nom de chaque app et son identifiant unique (par exemple <code>com.apple.Safari</code>), puis les navigateurs et le Finder donnent leurs onglets et leurs dossiers.</p>
      <p>Pour reprendre, Reprise fait l'inverse : elle relance chaque app avec la commande <code>open</code>, comme un double-clic, recrée les fenêtres de navigateur avec leurs onglets dans l'ordre, et rouvre les dossiers et les fichiers.</p>
      <p>C'est pour ça que macOS demande deux autorisations : <strong>Automatisation</strong> (piloter tes apps) et <strong>Accessibilité</strong> (voir les fichiers ouverts, facultatif).</p>

      <h3>Tes données</h3>
      <p>Tout est rangé dans un simple fichier sur ton Mac. Aucun compte, aucune pub, aucun traceur. La seule connexion à Internet sert à vérifier s'il existe une nouvelle version sur GitHub, et tu peux la couper dans les réglages.</p>

      <h3>Raccourcis</h3>
      <table class="keys-table">
        <tr><td><kbd>${esc(save)}</kbd></td><td>Sauvegarder ce qui est ouvert, depuis n'importe où</td></tr>
        <tr><td><kbd>${esc(open)}</kbd></td><td>Ouvrir Reprise</td></tr>
        <tr><td><kbd>↑</kbd> <kbd>↓</kbd></td><td>Passer d'un contexte à l'autre</td></tr>
        <tr><td><kbd>↵</kbd></td><td>Reprendre le contexte choisi</td></tr>
        <tr><td><kbd>⌘F</kbd></td><td>Chercher</td></tr>
        <tr><td><kbd>⌘N</kbd></td><td>Sauvegarder maintenant</td></tr>
        <tr><td><kbd>⌘⌫</kbd></td><td>Supprimer (avec « Annuler »)</td></tr>
      </table>

      <h3>Développement</h3>
      <div class="dev">
        <img src="assets/icon-180.png" alt="">
        <div>
          <p><strong>Reprise ${esc(info?.version || "")}</strong>. Application vibe codée par <strong>RenzVASA</strong>, avec l'aide de Claude, l'IA d'Anthropic.</p>
          <p class="muted">Faite avec Tauri 2 et Rust pour le moteur, HTML, CSS et JavaScript pour l'interface. Code ouvert sous licence MIT : tu peux le lire, le modifier et proposer des améliorations.</p>
          <div class="dev-actions">
            <button class="btn" data-i="github">Code source sur GitHub</button>
            <button class="btn" data-i="update">${updateChecking ? "Vérification…" : "Vérifier les mises à jour"}</button>
          </div>
          <p class="small muted" style="margin-top:.5rem">${esc(updateLine())}</p>
        </div>
      </div>
    </div>`;
}

infoEl.addEventListener("click", (e) => {
  const t = e.target.closest("[data-i]");
  if (!t) return;
  if (t.dataset.i === "close") infoEl.close();
  if (t.dataset.i === "github") api.openLink("https://github.com/RenzVASA/reprise").catch(() => {});
  if (t.dataset.i === "update") checkForUpdates(true);
});

// ───────────────────────────── Accueil ─────────────────────────────

const welcomeEl = $("#welcome");
let step = 0;

function renderWelcome() {
  const sc = prettyShortcut(state.settings?.shortcut_save || "Alt+Command+S");
  const steps = [
    `<h2 id="welcome-title">Reprends là où tu t'étais arrêté</h2>
     <p>Quand tu dois t'interrompre, appuie sur ce raccourci. Reprise photographie tes apps, tes onglets, tes dossiers et tes fichiers ouverts, et te demande en une phrase où tu en étais.</p>
     <div class="keys">${[...sc].map((k) => `<span class="key">${esc(k)}</span>`).join("")}</div>
     <p>Plus tard, un clic sur « Reprendre » remet tout en place, avec ta phrase affichée en haut de l'écran.</p>`,
    `<h2 id="welcome-title">Deux autorisations</h2>
     <p>macOS va te demander si Reprise peut piloter tes apps. C'est ce qui lui permet de lire puis de rouvrir ton travail. Tout reste sur ton Mac.</p>
     <div class="perm-card"><div class="perm"><div><div class="what">Automatisation</div><div class="why">Onglets, dossiers du Finder, apps ouvertes.</div></div>
       <span class="perm"><span class="perm-dot ${perms?.automation || ""}"></span><button class="btn ghost" data-w="pane" data-kind="automation">Réglages</button></span></div></div>
     <div class="perm-card"><div class="perm"><div><div class="what">Accessibilité</div><div class="why">Les fichiers ouverts dans tes apps. Facultatif.</div></div>
       <span class="perm"><span class="perm-dot ${perms?.accessibility || ""}"></span><button class="btn ghost" data-w="pane" data-kind="accessibility">Réglages</button></span></div></div>
     <button class="btn" data-w="check">${perms === "loading" ? "Vérification…" : "Demander les autorisations"}</button>`,
    `<h2 id="welcome-title">À toi de jouer</h2>
     <p>Va dans une autre app, ouvre ce sur quoi tu travailles, puis appuie sur <strong>${esc(sc)}</strong>. Écris ta phrase, valide avec Entrée, et c'est sauvegardé.</p>
     <p>Reprise reste disponible dans la barre des menus, en haut à droite de l'écran.</p>
     <div class="set-row"><div class="label"><span>Lancer Reprise à l'ouverture de session</span></div>
       <label class="switch"><input type="checkbox" data-w="login" ${state.settings?.launch_at_login ? "checked" : ""}><span class="track"></span></label></div>`,
  ];
  welcomeEl.innerHTML = `
    <div class="sheet-body">
      <div class="step-count">Étape ${step + 1} sur ${steps.length}</div>
      ${steps[step]}
      <div class="welcome-actions">
        ${step > 0 ? `<button class="btn ghost" data-w="back">Retour</button>` : "<span></span>"}
        <button class="btn primary" data-w="${step === steps.length - 1 ? "done" : "next"}">${step === steps.length - 1 ? "C'est parti" : "Continuer"}</button>
      </div>
    </div>`;
}

welcomeEl.addEventListener("click", async (e) => {
  const t = e.target.closest("[data-w]");
  if (!t) return;
  switch (t.dataset.w) {
    case "next": step++; renderWelcome(); break;
    case "back": step--; renderWelcome(); break;
    case "pane": api.openPrivacy(t.dataset.kind).catch(() => {}); break;
    case "check":
      perms = "loading"; renderWelcome();
      try { perms = await api.permissions(); } catch { perms = null; }
      renderWelcome();
      break;
    case "done":
      welcomeEl.close();
      await saveSettings({ onboarded: true });
      break;
  }
});
welcomeEl.addEventListener("change", (e) => {
  if (e.target.dataset.w === "login") saveSettings({ launch_at_login: e.target.checked });
});
welcomeEl.addEventListener("cancel", () => saveSettings({ onboarded: true }));

// ───────────────────────────── Démarrage ───────────────────────────

(async function start() {
  $("#brand-mark").innerHTML = repeatSign();
  $(".search-icon").innerHTML = icon("search");
  $("#btn-settings").innerHTML = icon("gear");
  $("#btn-info").innerHTML = icon("info");
  state.settings = await initPrefs();
  updateShortcutHints();
  await load();
  try { updateInfo = await api.getUpdate(); } catch { updateInfo = null; }
  showUpdatePill();
  if (state.settings && !state.settings.onboarded) {
    renderWelcome();
    welcomeEl.showModal();
  }
})();
