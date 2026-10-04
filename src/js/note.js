// Post-it : « Tu en étais là » après une reprise, ou « Tu étais sur… » en revenant.
import { api, on } from "./api.js";
import { initPrefs } from "./prefs.js";
import { icon, repeatSign, duration } from "./util.js";

const $ = (s) => document.querySelector(s);
let current = null;

async function show() {
  let n = null;
  try { n = await api.note(); } catch { /* rien à montrer */ }
  if (!n) return;
  current = n;
  const welcome = n.mode === "welcome";
  $("#kicker").textContent = welcome
    ? (n.away_ms ? `De retour après ${duration(n.away_ms)}` : "Content de te revoir")
    : "Tu en étais là";
  $("#name").textContent = welcome ? `Tu étais sur : ${n.name}` : n.name;
  $("#name").title = n.name;
  const text = $("#text");
  text.textContent = n.note || (welcome ? "Pas de note pour ce contexte." : "");
  text.classList.toggle("empty", !n.note);
  $("#later").hidden = !welcome;
  const go = $("#go");
  go.disabled = false;
  go.classList.toggle("primary", welcome);
  go.textContent = welcome ? "Reprendre" : "C'est reparti";
}

async function primary() {
  if (current?.mode === "welcome") {
    const go = $("#go");
    go.disabled = true;
    go.textContent = "Reprise…";
    try {
      await api.restore(current.id);
      // Le post-it « Tu en étais là » prend le relais (ou se ferme s'il n'y a pas de note).
    } catch {
      api.closeNote();
    }
  } else {
    api.closeNote();
  }
}

$("#go").addEventListener("click", primary);
$("#later").addEventListener("click", () => api.closeNote());
$("#close").addEventListener("click", () => api.closeNote());
document.addEventListener("keydown", (e) => {
  if (e.key === "Escape") api.closeNote();
  if (e.key === "Enter") primary();
});

on("note-show", show);

(async function start() {
  $("#sign").innerHTML = repeatSign();
  $("#close").innerHTML = icon("close");
  await initPrefs();
  await show();
})();
