// Fenêtre de sauvegarde rapide.
import { api, on } from "./api.js";
import { initPrefs } from "./prefs.js";
import { esc, icon, repeatSign, plural, suggestName, domain } from "./util.js";

const $ = (s) => document.querySelector(s);
const noteEl = $("#note");
const nameEl = $("#name");
const saveBtn = $("#save");
const previewEl = $("#preview");
const warnEl = $("#warn");
const pickEl = $("#pick");

let ready = false;
let saveWhenReady = false;
let saving = false;
let nameTouched = false;

function reset() {
  ready = false;
  saveWhenReady = false;
  saving = false;
  nameTouched = false;
  excluded = new Set();
  openKind = null;
  ignoredMsg = "";
  noteEl.value = "";
  nameEl.value = "";
  warnEl.hidden = true;
  $("#cap-title").textContent = "Sauvegarder ce contexte";
  $("#cap-sub").textContent = "Photo de ce qui est ouvert…";
  renderPreview(null);
  setSaveLabel();
}

function setSaveLabel() {
  saveBtn.disabled = saving;
  saveBtn.innerHTML = saving ? "Sauvegarde…" : saveWhenReady ? "Dès que c'est prêt…" : "Sauvegarder <kbd>↵</kbd>";
}

const KINDS = [
  ["app", "app", "apps", "app"],
  ["tab", "onglet", "onglets", "tab"],
  ["folder", "dossier", "dossiers", "folder"],
  ["document", "fichier", "fichiers", "document"],
];

/** Éléments décochés dans cette sauvegarde. */
let excluded = new Set();
/** Catégorie dépliée dans le panneau de choix. */
let openKind = null;
let snapshot = null;
let ignoredMsg = "";

function renderPreview(snap) {
  snapshot = snap;
  if (!snap) {
    previewEl.innerHTML = `<span class="spinner"></span><span>Photo de tes apps, onglets et dossiers…</span>`;
    pickEl.hidden = true;
    return;
  }
  const chips = KINDS.map(([kind, one, many, ic]) => {
    const all = snap.items.filter((i) => i.kind === kind);
    if (!all.length) return "";
    const kept = all.filter((i) => !excluded.has(i.id)).length;
    const label = kept === all.length ? plural(all.length, one, many) : `${kept} sur ${plural(all.length, one, many)}`;
    return `<button type="button" class="chip pick-chip ${openKind === kind ? "open" : ""} ${kept < all.length ? "partial" : ""}"
      data-kind="${kind}" aria-expanded="${openKind === kind}" title="Choisir ce qu'on garde">${icon(ic)}${label}<span class="chev" aria-hidden="true"></span></button>`;
  }).join("");
  previewEl.innerHTML = chips
    ? chips + `<span class="pick-hint">${openKind ? "" : "Clique pour choisir"}</span>`
    : `<span>Rien d'ouvert à part Reprise. Tu peux quand même garder ta note.</span>`;

  renderPick();

  if (snap.warnings?.length) {
    const w = snap.warnings[0];
    const kind = /Accessibilit/.test(w) ? "accessibility" : "automation";
    warnEl.innerHTML = `${icon("warn")}<span title="${esc(snap.warnings.join("\n"))}">${esc(w)}${snap.warnings.length > 1 ? ` (+${snap.warnings.length - 1})` : ""}</span>
      <button class="btn" data-pane="${kind}">Ouvrir les réglages</button>`;
    warnEl.hidden = false;
  } else {
    warnEl.hidden = true;
  }
}

function renderPick() {
  document.body.classList.toggle("picking", Boolean(snapshot && openKind));
  if (!snapshot || !openKind) { pickEl.hidden = true; return; }
  const items = snapshot.items.filter((i) => i.kind === openKind);
  if (!items.length) { openKind = null; pickEl.hidden = true; document.body.classList.remove("picking"); return; }
  const allOff = items.every((i) => excluded.has(i.id));
  const sub = (it) => it.kind === "tab" ? domain(it.value)
    : it.kind === "app" ? "" : it.value.replace(/^\/Users\/[^/]+/, "~");
  pickEl.innerHTML = `
    <div class="pick-head">
      <span>${ignoredMsg ? esc(ignoredMsg) : "Décoche ce que tu ne veux pas garder."}</span>
      <button type="button" class="link" data-all="${allOff ? "on" : "off"}">${allOff ? "Tout cocher" : "Tout décocher"}</button>
    </div>
    <ul class="pick-list">
      ${items.map((it) => `
        <li class="${excluded.has(it.id) ? "off" : ""}">
          <label>
            <input type="checkbox" data-id="${esc(it.id)}" ${excluded.has(it.id) ? "" : "checked"}>
            <span class="pick-text"><span class="pick-title">${esc(it.label)}</span>${sub(it) ? `<span class="pick-sub">${esc(sub(it))}</span>` : ""}</span>
          </label>
          ${it.kind === "app" && it.bundle_id ? `<button type="button" class="link quiet" data-ignore="${esc(it.bundle_id)}" data-name="${esc(it.label)}">Ne jamais enregistrer</button>` : ""}
        </li>`).join("")}
    </ul>`;
  pickEl.hidden = false;
}

async function refresh() {
  let p = null;
  try { p = await api.pending(); } catch { /* fenêtre ouverte sans capture */ }
  if (!p) return;
  if (p.replace) {
    $("#cap-title").textContent = `Mettre à jour « ${p.replace.name} »`;
    if (!nameTouched && !nameEl.value) nameEl.value = p.replace.name;
    if (!noteEl.value) noteEl.value = p.replace.note || "";
  } else if (!nameTouched && !nameEl.value) {
    nameEl.value = suggestName(p.front_app);
  }
  const total = p.snapshot?.items.length;
  $("#cap-sub").textContent = p.ready
    ? `${plural(total, "élément photographié", "éléments photographiés")}${p.front_app ? `, ${p.front_app} au premier plan` : ""}.`
    : `Photo de ${p.front_app || "ce qui est ouvert"}…`;
  if (p.ready && !ready) {
    ready = true;
    renderPreview(p.snapshot);
    if (saveWhenReady) save();
  } else if (!p.ready) {
    renderPreview(null);
  }
  setSaveLabel();
}

async function save() {
  if (saving) return;
  if (!ready) { saveWhenReady = true; setSaveLabel(); return; }
  saving = true;
  setSaveLabel();
  try {
    await api.saveCapture(nameEl.value, noteEl.value, excluded.size ? [...excluded] : null);
    dismissTip();
    reset();
  } catch (e) {
    saving = false;
    setSaveLabel();
    previewEl.insertAdjacentHTML("beforeend", `<span class="chip" style="color:var(--danger)">${esc(String(e))}</span>`);
  }
}

function cancel() {
  reset();
  api.cancelCapture();
}

// ── Événements ──
saveBtn.addEventListener("click", save);
$("#cancel").addEventListener("click", cancel);
nameEl.addEventListener("input", () => (nameTouched = true));

document.addEventListener("keydown", (e) => {
  if (e.key === "Escape") { e.preventDefault(); cancel(); return; }
  if (e.key === "Enter" && !e.shiftKey && !e.isComposing) {
    if (e.target === noteEl || e.target === nameEl || e.metaKey) { e.preventDefault(); save(); }
  }
});

$("#starters").addEventListener("click", (e) => {
  const b = e.target.closest(".starter");
  if (!b) return;
  const text = b.textContent;
  const v = noteEl.value;
  noteEl.value = v && !/\s$/.test(v) ? `${v} ${text}` : v + text;
  noteEl.focus();
  noteEl.setSelectionRange(noteEl.value.length, noteEl.value.length);
});

previewEl.addEventListener("click", (e) => {
  const chip = e.target.closest(".pick-chip");
  if (!chip) return;
  openKind = openKind === chip.dataset.kind ? null : chip.dataset.kind;
  ignoredMsg = "";
  renderPreview(snapshot);
});

pickEl.addEventListener("change", (e) => {
  const id = e.target.dataset.id;
  if (!id) return;
  e.target.checked ? excluded.delete(id) : excluded.add(id);
  renderPreview(snapshot);
});

pickEl.addEventListener("click", async (e) => {
  const all = e.target.closest("[data-all]");
  if (all) {
    const ids = snapshot.items.filter((i) => i.kind === openKind).map((i) => i.id);
    ids.forEach((id) => (all.dataset.all === "off" ? excluded.add(id) : excluded.delete(id)));
    renderPreview(snapshot);
    return;
  }
  const ign = e.target.closest("[data-ignore]");
  if (ign) {
    const bundle = ign.dataset.ignore;
    try {
      await api.ignoreApp(bundle, ign.dataset.name);
      // L'app, ses onglets et ses fichiers sortent de cette sauvegarde aussi.
      snapshot.items.filter((i) => i.bundle_id === bundle).forEach((i) => excluded.add(i.id));
      snapshot = { ...snapshot, items: snapshot.items.filter((i) => i.bundle_id !== bundle) };
      ignoredMsg = `${ign.dataset.name} ne sera plus jamais enregistrée. Tu peux changer d'avis dans les réglages.`;
      renderPreview(snapshot);
    } catch (err) {
      ignoredMsg = String(err);
      renderPick();
    }
  }
});

warnEl.addEventListener("click", (e) => {
  const b = e.target.closest("[data-pane]");
  if (b) api.openPrivacy(b.dataset.pane).catch(() => {});
});

on("capture-start", () => { reset(); refresh(); setTimeout(() => noteEl.focus(), 30); });
on("capture-ready", () => refresh());
window.addEventListener("focus", () => noteEl.focus());

let tipSeen = true;

function showTip(on) {
  $("#tip").hidden = !on;
  document.body.classList.toggle("has-tip", on);
}

function dismissTip() {
  if (tipSeen) return;
  tipSeen = true;
  showTip(false);
  api.markSeen("capture").catch(() => {});
}

$("#tip-ok").addEventListener("click", () => { dismissTip(); noteEl.focus(); });

(async function start() {
  $("#cap-sign").innerHTML = repeatSign();
  const prefs = await initPrefs();
  tipSeen = !prefs || (prefs.seen || []).includes("capture");
  showTip(!tipSeen);
  reset();
  await refresh();
  noteEl.focus();
})();
