// Fenêtre principale de Reprise.
import { api, on } from "./api.js";
import { applyPrefs, initPrefs } from "./prefs.js";
import { esc, icon, repeatSign, ago, when, bucket, domain, hue, initial, prettyShortcut, plural } from "./util.js";
import { startTour } from "./tour.js";

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
  if (state.touring) return;
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
    .sort((a, b) => (a.auto - b.auto) || (b.pinned - a.pinned) || (a.auto ? b.updated_at - a.updated_at : b.created_at - a.created_at));
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
  if (c.auto) {
    return `
    <button class="row auto ${c.id === state.selected ? "selected" : ""}" data-id="${esc(c.id)}" aria-current="${c.id === state.selected}">
      <div class="row-name">${icon("shield")}<span>${esc(when(c.updated_at))}</span></div>
      <div class="row-meta">${meta}<span class="time">${esc(c.front_app || "")}</span></div>
    </button>`;
  }
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
    const pinned = vis.filter((c) => c.pinned && !c.auto);
    if (pinned.length) html += `<h2 class="group-title">Épinglés</h2>` + pinned.map(rowHtml).join("");
    let last = null;
    for (const c of vis.filter((c) => !c.pinned && !c.auto)) {
      const b = bucket(c.created_at);
      if (b !== last) { html += `<h2 class="group-title">${b}</h2>`; last = b; }
      html += rowHtml(c);
    }
    const autos = vis.filter((c) => c.auto);
    if (autos.length) {
      html += `<h2 class="group-title safety" title="Reprise sauvegarde toute seule quand tu t'absentes. Elle garde les 3 dernières.">${icon("shield")}Filet de sécurité</h2>` + autos.map(rowHtml).join("");
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

  if (c.auto) {
    metaBits.length = 0;
    metaBits.push(`Sauvegardé automatiquement ${when(c.updated_at)}`);
    if (c.front_app) metaBits.push(`${c.front_app} au premier plan`);
  }
  const autoBanner = c.auto ? `
      <div class="auto-banner">
        ${icon("shield")}
        <p>Reprise a fait cette sauvegarde toute seule pendant ton absence. Elle sera remplacée par les suivantes : garde-la si tu veux la conserver.</p>
        <button class="btn" data-action="keep">Garder ce contexte</button>
      </div>` : "";

  detailEl.innerHTML = `
    <div class="detail-inner">
      ${autoBanner}
      <h1 class="title" id="title" contenteditable="${c.auto ? "false" : "plaintext-only"}" spellcheck="false" title="${c.auto ? "" : "Clique pour renommer"}">${esc(c.auto ? "Sauvegarde auto" : c.name)}</h1>
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
        ${c.auto ? "" : `<button class="btn ghost" data-action="pin">${icon("pin")}${c.pinned ? "Désépingler" : "Épingler"}</button>`}
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
  if (!c || c.auto) return;
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

async function keepAuto() {
  const c = current();
  if (!c) return;
  try {
    const u = await api.update(c.id, { auto: false });
    Object.assign(c, u);
    render();
    const t = $("#title");
    if (t) {
      t.focus();
      document.getSelection()?.selectAllChildren(t);
    }
    toast("Gardé. Donne-lui un nom, puis ajoute une phrase « où tu en étais ».");
  } catch (e) { toast(String(e)); }
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
    case "keep": keepAuto(); break;
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
        ${rec("shortcut_resume", "Reprendre le dernier contexte")}
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
        <h3>Filet de sécurité</h3>
        ${toggleRow("auto_save", "Sauvegarder tout seul quand je m'absente", "Quand tu verrouilles ton Mac, qu'il se met en veille, ou après 5 minutes sans toucher au clavier. Reprise garde les 3 dernières.")}
        ${toggleRow("welcome_back", "Me proposer de reprendre en revenant", "Un post-it « Tu étais sur… » après une absence.")}
        <div class="set-row ${s.welcome_back ? "" : "dim"}">
          <div class="label"><span>Après une absence de</span></div>
          <div class="segmented" role="radiogroup" aria-label="Durée d'absence">
            ${[10, 20, 30, 60].map((m) =>
              `<button role="radio" aria-checked="${s.welcome_minutes === m}" data-set="welcome-min" data-value="${m}" ${s.welcome_back ? "" : "disabled"}>${m < 60 ? m + " min" : "1 h"}</button>`).join("")}
          </div>
        </div>
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
            <button class="btn" data-set="tour">Revoir la visite guidée</button>
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
    case "tour": settingsEl.close(); runTour(); break;
    case "welcome-min": saveSettings({ welcome_minutes: Number(t.dataset.value) }); break;
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
  if (!updateInfo.available) return "Tu as la dernière version.";
  return updateInfo.can_install ? `La version ${updateInfo.latest} est prête à s'installer.` : `La version ${updateInfo.latest} est disponible.`;
}

function showUpdatePill() {
  const pill = $("#update-pill");
  if (updateInfo?.available) {
    pill.innerHTML = `${icon("download")}<span>${updateInfo.can_install ? "Mettre à jour" : "Version " + esc(updateInfo.latest) + " disponible"}</span>`;
    pill.title = `Reprise ${updateInfo.latest} est disponible`;
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
let installing = false;

function updateNotes() {
  return (updateInfo?.notes || "").split("\n").map((l) => l.trim()).filter((l) => /^[-*•]\s+/.test(l))
    .map((l) => l.replace(/^[-*•]\s+/, "").replace(/\*\*/g, ""));
}

function openUpdate() {
  if (!updateInfo) return;
  const notes = updateNotes();
  const auto = updateInfo.can_install;
  updateEl.innerHTML = `
    <div class="sheet-head">
      <h2 id="update-title">Reprise ${esc(updateInfo.latest)} est disponible</h2>
      <button class="btn ghost icon-only" data-u="close" aria-label="Fermer">${icon("close")}</button>
    </div>
    <div class="sheet-body">
      <p class="muted">Tu as la version ${esc(updateInfo.current)}.${notes.length ? " Nouveautés :" : ""}</p>
      ${notes.length ? `<ul class="notes">${notes.slice(0, 12).map((n) => `<li>${esc(n)}</li>`).join("")}</ul>` : ""}
      ${auto ? `
        <p class="how-auto">Reprise télécharge la nouvelle version, vérifie qu'elle vient bien de GitHub, l'installe et redémarre. Tes contextes et tes réglages sont gardés.</p>
        <div class="progress" id="up-progress" hidden><div class="bar"><span id="up-bar"></span></div><span class="small muted" id="up-text">Téléchargement…</span></div>
        <div class="welcome-actions">
          <button class="btn ghost" data-u="close" id="up-later">Plus tard</button>
          <button class="btn primary" data-u="install" id="up-install">${icon("download")}Mettre à jour et redémarrer</button>
        </div>` : `
        <ol class="how">
          <li>Télécharge le fichier <strong>.dmg</strong> sur la page de la version.</li>
          <li>Quitte Reprise, puis glisse la nouvelle version dans Applications pour remplacer l'ancienne. Tes contextes et tes réglages sont gardés.</li>
          <li>Dans le Terminal, une fois : <code>xattr -cr /Applications/Reprise.app</code></li>
        </ol>
        <div class="welcome-actions">
          <button class="btn ghost" data-u="close">Plus tard</button>
          <button class="btn primary" data-u="get">${icon("download")}Ouvrir la page de téléchargement</button>
        </div>`}
    </div>`;
  if (!updateEl.open) updateEl.showModal();
}

async function installUpdate() {
  if (installing) return;
  installing = true;
  $("#up-progress").hidden = false;
  $("#up-install").disabled = true;
  $("#up-later").disabled = true;
  $("#up-install").textContent = "Mise à jour en cours…";
  try {
    await api.installUpdate();
  } catch (e) {
    installing = false;
    $("#up-text").textContent = String(e);
    $("#up-install").disabled = false;
    $("#up-later").disabled = false;
    $("#up-install").textContent = "Réessayer";
  }
}

on("update-progress", ({ downloaded, total }) => {
  const bar = $("#up-bar"), txt = $("#up-text");
  if (!bar) return;
  const mb = (n) => (n / 1048576).toFixed(1).replace(".", ",");
  if (total) {
    bar.style.width = `${Math.min(100, (downloaded / total) * 100)}%`;
    txt.textContent = `Téléchargement : ${mb(downloaded)} sur ${mb(total)} Mo`;
  } else {
    txt.textContent = `Téléchargement : ${mb(downloaded)} Mo`;
  }
});
on("update-installed", () => {
  const bar = $("#up-bar"), txt = $("#up-text");
  if (bar) bar.style.width = "100%";
  if (txt) txt.textContent = "Installé. Reprise redémarre…";
});

updateEl.addEventListener("click", (e) => {
  const t = e.target.closest("[data-u]");
  if (!t || t.disabled) return;
  if (t.dataset.u === "install") { installUpdate(); return; }
  if (t.dataset.u === "get") api.openLink(updateInfo.url).catch((err) => toast(String(err)));
  updateEl.close();
});
updateEl.addEventListener("cancel", (e) => { if (installing) e.preventDefault(); });
$("#update-pill").addEventListener("click", openUpdate);
on("update-available", (u) => { updateInfo = u; showUpdatePill(); });
on("update-open", (u) => { updateInfo = u; showUpdatePill(); openUpdate(); });

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
  const resume = prettyShortcut(state.settings?.shortcut_resume || "Alt+Shift+Command+R");
  infoEl.innerHTML = `
    <div class="sheet-head">
      <h2 id="info-title">Comment marche Reprise</h2>
      <button class="btn ghost icon-only" data-i="close" aria-label="Fermer">${icon("close")}</button>
    </div>
    <div class="sheet-body prose">
      <p class="lead">Reprise sert à <span class="marker">ne plus perdre le fil quand tu es interrompu</span>. Elle prend une photo de ce sur quoi tu travailles, avec une phrase pour te souvenir où tu en étais, et remet tout en place quand tu reviens.</p>
      <button class="btn" data-i="tour">Revoir la visite guidée</button>

      <h3>En trois gestes</h3>
      <ol class="steps-list">
        <li><strong>Appuie sur ${esc(save)}</strong>, depuis n'importe quelle app. Une petite fenêtre s'ouvre.</li>
        <li><strong>Écris où tu en es</strong>, en une phrase. Tu peux cliquer sur les pastilles (« 4 apps », « 12 onglets »…) pour décocher ce que tu ne veux pas garder. Entrée, c'est sauvegardé.</li>
        <li><strong>Plus tard, clique sur Reprendre.</strong> Tout se rouvre, les fenêtres à leur place, et ta phrase s'affiche sur un post-it en haut de l'écran. Encore plus rapide : <strong>${esc(resume)}</strong> reprend le dernier contexte sans ouvrir Reprise.</li>
      </ol>

      <h3>Le filet de sécurité</h3>
      <p>Tu as oublié d'appuyer sur ${esc(save)} ? Pas grave. Quand tu verrouilles ton Mac, qu'il se met en veille ou que tu ne touches plus au clavier pendant 5 minutes, Reprise sauvegarde toute seule. Elle garde les 3 dernières sauvegardes, en bas de la liste. Si l'une d'elles est importante, clique sur « Garder ce contexte ».</p>
      <p>Et quand tu reviens après une vraie absence (Mac verrouillé ou en veille), un post-it te demande : « Tu étais sur… on reprend ? »</p>

      <h3>Ce qui est enregistré</h3>
      <ul>
        <li>Les <strong>apps</strong> ouvertes (celles qui ont une icône dans le Dock).</li>
        <li>Les <strong>onglets</strong> de Safari, Chrome, Brave, Edge, Arc, Vivaldi, Opera et Opera GX, fenêtre par fenêtre.</li>
        <li>Les <strong>dossiers</strong> ouverts dans le Finder.</li>
        <li>Les <strong>fichiers</strong> ouverts dans tes apps (PDF, Pages, Keynote, VS Code…), si l'accès Accessibilité est autorisé.</li>
        <li>La <strong>place et la taille</strong> de chaque fenêtre, y compris sur un deuxième écran. Si cet écran n'est plus branché, la fenêtre reste sur l'écran principal.</li>
      </ul>
      <p class="muted">Ce qui ne l'est pas : ce qu'il y a <em>dans</em> les apps (la chanson en cours, un texte pas encore enregistré, la position dans une vidéo), les onglets de Firefox et souvent ceux de navigation privée. C'est pour ça que ta phrase compte.</p>

      <h3>Comment ça marche, en coulisses</h3>
      <p>Pour savoir ce qui est ouvert, Reprise pose la question à macOS en <strong>AppleScript</strong>, le langage intégré au Mac pour piloter les apps. macOS répond avec le nom de chaque app et son identifiant unique (par exemple <code>com.apple.Safari</code>), puis les navigateurs et le Finder donnent leurs onglets et leurs dossiers.</p>
      <p>Pour reprendre, Reprise fait l'inverse : elle relance chaque app avec la commande <code>open</code>, comme un double-clic, recrée les fenêtres de navigateur avec leurs onglets dans l'ordre, et rouvre les dossiers et les fichiers.</p>
      <p>C'est pour ça que macOS demande deux autorisations : <strong>Automatisation</strong> (piloter tes apps) et <strong>Accessibilité</strong> (voir les fichiers ouverts, facultatif).</p>

      <h3>Tes données</h3>
      <p>Tout est rangé dans un simple fichier sur ton Mac. Aucun compte, aucune pub, aucun traceur. La seule connexion à Internet sert à vérifier s'il existe une nouvelle version sur GitHub, et tu peux la couper dans les réglages.</p>

      <h3>Les mises à jour</h3>
      <p>Quand une nouvelle version sort, un bouton « Mettre à jour » apparaît en haut de la fenêtre. Un clic, et Reprise la télécharge, vérifie sa signature (pour être sûre qu'elle vient bien de GitHub et que personne ne l'a modifiée), l'installe et redémarre. Tes contextes et tes réglages restent intacts.</p>

      <h3>Raccourcis</h3>
      <table class="keys-table">
        <tr><td><kbd>${esc(save)}</kbd></td><td>Sauvegarder ce qui est ouvert, depuis n'importe où</td></tr>
        <tr><td><kbd>${esc(open)}</kbd></td><td>Ouvrir Reprise</td></tr>
        <tr><td><kbd>${esc(resume)}</kbd></td><td>Reprendre le dernier contexte, depuis n'importe où</td></tr>
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
  if (t.dataset.i === "tour") { infoEl.close(); runTour(); }
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
    `<h2 id="welcome-title">Une visite d'une minute ?</h2>
     <p>Je te montre où est chaque chose, sur un exemple. Tu pourras la revoir quand tu veux avec le bouton <strong>i</strong>, en haut de la fenêtre.</p>
     <div class="set-row"><div class="label"><span>Lancer Reprise à l'ouverture de session</span><span class="hint">Pour que le filet de sécurité soit toujours là.</span></div>
       <label class="switch"><input type="checkbox" data-w="login" ${state.settings?.launch_at_login ? "checked" : ""}><span class="track"></span></label></div>`,
  ];
  welcomeEl.innerHTML = `
    <div class="sheet-body">
      <div class="step-count">Étape ${step + 1} sur ${steps.length}</div>
      ${steps[step]}
      <div class="welcome-actions">
        ${step === steps.length - 1 ? `<button class="btn ghost" data-w="skip">Passer</button>` : step > 0 ? `<button class="btn ghost" data-w="back">Retour</button>` : "<span></span>"}
        <button class="btn primary" data-w="${step === steps.length - 1 ? "done" : "next"}">${step === steps.length - 1 ? "Faire la visite" : "Continuer"}</button>
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
      await finishOnboarding();
      runTour();
      break;
    case "skip":
      welcomeEl.close();
      await finishOnboarding();
      break;
    case "whatsnew-tour":
      welcomeEl.close();
      runTour();
      break;
    case "whatsnew-ok":
      welcomeEl.close();
      break;
  }
});

async function finishOnboarding() {
  await saveSettings({ onboarded: true });
  try { state.settings = await api.markSeen("", info?.version || null); } catch { /* sans gravité */ }
}

// ───────────────────────────── Visite guidée ───────────────────────

const DEMO_ID = "__visite__";
const DEMO_AUTO_ID = "__visite_auto__";

function demoContexts() {
  const now = Date.now();
  const it = (id, kind, app, label, value, group = null) => ({ id, kind, app_name: app, bundle_id: null, label, value, group, frames: [] });
  return [
    {
      id: DEMO_ID, name: "Exemple : exposé d'histoire", pinned: false, auto: false,
      note: "Il reste la conclusion. Les sources de la diapo 7 sont dans l'onglet Lumni.",
      created_at: now - 3600e3, updated_at: now - 3600e3, last_restored_at: null, restore_count: 0, front_app: "Keynote",
      items: [
        it("v1", "app", "Keynote", "Keynote", "k"), it("v2", "app", "Safari", "Safari", "s"),
        it("v3", "tab", "Safari", "La guerre froide en 10 dates – Lumni", "https://www.lumni.fr/dossier/la-guerre-froide", 1),
        it("v4", "tab", "Safari", "Crise des missiles de Cuba — Wikipédia", "https://fr.wikipedia.org/wiki/Crise_des_missiles_de_Cuba", 1),
        it("v5", "folder", "Finder", "Histoire", "/Users/toi/Documents/Histoire"),
        it("v6", "document", "Keynote", "Exposé.key", "/Users/toi/Documents/Histoire/Exposé.key"),
      ],
    },
    {
      id: DEMO_AUTO_ID, name: "Sauvegarde auto", pinned: false, auto: true, note: "",
      created_at: now - 7200e3, updated_at: now - 7200e3, last_restored_at: null, restore_count: 0, front_app: "Safari",
      items: [it("w1", "app", "Safari", "Safari", "s")],
    },
  ];
}

function runTour() {
  for (const d of document.querySelectorAll("dialog[open]")) d.close();
  const before = { contexts: state.contexts, selected: state.selected, query: state.query };
  // Toujours le même exemple, pour que la visite soit identique pour tout le monde.
  state.touring = true;
  state.contexts = demoContexts();
  state.selected = DEMO_ID;
  state.query = "";
  searchEl.value = "";
  render();
  detailEl.scrollTop = 0;

  const s = state.settings || {};
  const save = prettyShortcut(s.shortcut_save || "Alt+Command+S");
  const resume = prettyShortcut(s.shortcut_resume || "Alt+Shift+Command+R");
  startTour([
    { title: "Voici un exemple", text: "Pour la visite, j'ai préparé un faux contexte : un exposé d'histoire en cours. Rien n'est enregistré, il disparaîtra à la fin." },
    { target: "#btn-capture", title: "Sauvegarder", text: `Le geste principal : <strong>${esc(save)}</strong>, depuis n'importe quelle app. Ce bouton fait la même chose.` },
    { target: "#list", title: "Tes contextes", text: "Ils sont rangés par jour, les épinglés en haut. Cherche dans les noms, les notes et les onglets avec <strong>⌘F</strong>." },
    { target: ".note-block", title: "Où tu en étais", text: "La phrase que tu écris en sauvegardant, surlignée pour la voir tout de suite. Clique dessus pour la modifier." },
    { target: ".resume-bar", title: "Reprendre", text: `Un clic rouvre tout, fenêtres à leur place. « Faire place nette » masque le reste. Depuis n'importe où : <strong>${esc(resume)}</strong>.` },
    { target: ".section", title: "Choisir ce qu'on rouvre", text: "Décoche ce dont tu n'as plus besoin. Survole une ligne pour rouvrir un seul élément." },
    { target: ".group-title.safety", title: "Le filet de sécurité", text: "Si tu oublies de sauvegarder, Reprise le fait toute seule quand tu verrouilles ton Mac ou que tu t'absentes. Elle garde les 3 dernières ici." },
    { target: ".top-actions", title: "Aide et réglages", text: "Le <strong>i</strong> explique tout, la roue règle les polices, les raccourcis et le filet. Reprise vit aussi dans la barre des menus, en haut à droite de l'écran." },
    { title: "À toi de jouer", text: `Va dans une autre app, ouvre ce sur quoi tu travailles, et appuie sur <strong>${esc(save)}</strong>. Écris ta phrase, Entrée, c'est sauvegardé.` },
  ], {
    doneLabel: "C'est parti",
    onEnd: () => {
      state.touring = false;
      state.contexts = before.contexts;
      state.selected = before.selected;
      state.query = before.query;
      searchEl.value = before.query;
      api.markSeen("tour").then((s2) => { state.settings = s2; }).catch(() => {});
      load();
    },
  });
}

// ───────────────────────────── Nouveautés ──────────────────────────

const WHATS_NEW = [
  ["Les fenêtres reviennent à leur place", "Position et taille, y compris sur un deuxième écran."],
  ["Le filet de sécurité", "Reprise sauvegarde toute seule quand tu verrouilles ton Mac ou que tu t'absentes."],
  ["« Tu étais sur… »", "En revenant, un post-it te propose de reprendre."],
  ["Reprendre le dernier contexte en un raccourci", "⌥⇧⌘R, depuis n'importe quelle app."],
  ["Les mises à jour s'installent toutes seules", "Un clic, Reprise redémarre à jour."],
];

function showWhatsNew(version) {
  welcomeEl.innerHTML = `
    <div class="sheet-body">
      <div class="step-count">Reprise ${esc(version)}</div>
      <h2 id="welcome-title">Quoi de neuf ?</h2>
      <ul class="whatsnew">
        ${WHATS_NEW.map(([t, d]) => `<li><strong>${esc(t)}</strong><span>${esc(d)}</span></li>`).join("")}
      </ul>
      <div class="welcome-actions">
        <button class="btn ghost" data-w="whatsnew-tour">Revoir la visite guidée</button>
        <button class="btn primary" data-w="whatsnew-ok">Super</button>
      </div>
    </div>`;
  welcomeEl.showModal();
}
welcomeEl.addEventListener("change", (e) => {
  if (e.target.dataset.w === "login") saveSettings({ launch_at_login: e.target.checked });
});
welcomeEl.addEventListener("cancel", () => { if (!state.settings?.onboarded) finishOnboarding(); });

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
  try { info = await api.info(); } catch { info = null; }
  showUpdatePill();
  if (state.settings && !state.settings.onboarded) {
    renderWelcome();
    welcomeEl.showModal();
  } else if (state.settings && info && state.settings.last_version !== info.version) {
    // Première ouverture après une mise à jour : on montre les nouveautés une fois.
    showWhatsNew(info.version);
    api.markSeen("", info.version).then((s2) => { state.settings = s2; }).catch(() => {});
  }
})();
