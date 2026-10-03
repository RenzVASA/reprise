// Post-it affiché après la reprise d'un contexte.
import { api, on } from "./api.js";
import { initPrefs } from "./prefs.js";
import { icon, repeatSign } from "./util.js";

const $ = (s) => document.querySelector(s);

async function show() {
  let n = null;
  try { n = await api.note(); } catch { /* rien à montrer */ }
  if (!n) return;
  $("#name").textContent = n.name;
  $("#name").title = n.name;
  $("#text").textContent = n.note;
}

const close = () => api.closeNote();
$("#go").addEventListener("click", close);
$("#close").addEventListener("click", close);
document.addEventListener("keydown", (e) => { if (e.key === "Escape" || e.key === "Enter") close(); });

on("note-show", show);

(async function start() {
  $("#sign").innerHTML = repeatSign();
  $("#close").innerHTML = icon("close");
  await initPrefs();
  await show();
})();
