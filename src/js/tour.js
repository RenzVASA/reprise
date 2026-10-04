// Visite guidée : on éclaire un morceau de l'écran à la fois, avec une bulle d'explication.
// Échap pour arrêter, Entrée ou → pour continuer, ← pour revenir.



/**
 * @param {{target?: string, title: string, text: string}[]} steps
 * @param {{onEnd?: (finished: boolean) => void, doneLabel?: string}} opts
 */
export function startTour(steps, { onEnd, doneLabel = "Terminer" } = {}) {
  let i = 0;
  const root = document.createElement("div");
  root.className = "tour";
  root.innerHTML = `
    <div class="tour-hole" aria-hidden="true"></div>
    <div class="tour-bubble" role="dialog" aria-modal="true" aria-labelledby="tour-title">
      <div class="tour-count"></div>
      <h3 id="tour-title"></h3>
      <p class="tour-text"></p>
      <div class="tour-actions">
        <button class="btn ghost tour-skip">Passer la visite</button>
        <span class="tour-nav">
          <button class="btn tour-back">Retour</button>
          <button class="btn primary tour-next">Suivant</button>
        </span>
      </div>
    </div>`;
  document.body.appendChild(root);
  const hole = root.querySelector(".tour-hole");
  const bubble = root.querySelector(".tour-bubble");
  const next = root.querySelector(".tour-next");
  const back = root.querySelector(".tour-back");

  function place() {
    const step = steps[i];
    const el = step.target ? document.querySelector(step.target) : null;
    const pad = 8;
    const vw = window.innerWidth, vh = window.innerHeight;
    const bw = Math.min(360, vw - 32);
    bubble.style.width = `${bw}px`;

    if (!el) {
      // Étape sans cible : bulle au centre, tout l'écran assombri.
      root.classList.add("centered");
      hole.style.cssText = `left:${vw / 2}px;top:${vh / 2}px;width:0;height:0`;
      const bh = bubble.offsetHeight;
      bubble.style.left = `${(vw - bw) / 2}px`;
      bubble.style.top = `${Math.max(16, (vh - bh) / 2)}px`;
      return;
    }
    root.classList.remove("centered");
    el.scrollIntoView({ block: "nearest", inline: "nearest" });
    const r = el.getBoundingClientRect();
    const top = Math.max(4, r.top - pad), left = Math.max(4, r.left - pad);
    const w = Math.min(vw - left - 4, r.width + pad * 2), h = Math.min(vh - top - 4, r.height + pad * 2);
    hole.style.cssText = `left:${left}px;top:${top}px;width:${w}px;height:${h}px`;

    const bh = bubble.offsetHeight;
    const gap = 14;
    let bx, by;
    if (top + h + gap + bh < vh - 12) { by = top + h + gap; bx = left; }          // en dessous
    else if (top - gap - bh > 12) { by = top - gap - bh; bx = left; }             // au-dessus
    else if (left + w + gap + bw < vw - 12) { bx = left + w + gap; by = top; }    // à droite
    else { bx = left - gap - bw; by = top; }                                      // à gauche
    bubble.style.left = `${Math.min(Math.max(16, bx), vw - bw - 16)}px`;
    bubble.style.top = `${Math.min(Math.max(16, by), vh - bh - 16)}px`;
  }

  function show() {
    const step = steps[i];
    root.querySelector(".tour-count").textContent = `${i + 1} sur ${steps.length}`;
    root.querySelector("#tour-title").textContent = step.title;
    root.querySelector(".tour-text").innerHTML = step.text;
    back.hidden = i === 0;
    next.textContent = i === steps.length - 1 ? doneLabel : "Suivant";
    root.querySelector(".tour-skip").hidden = i === steps.length - 1;
    place();
    next.focus();
  }

  function end(finished) {
    window.removeEventListener("resize", place);
    document.removeEventListener("keydown", onKey, true);
    root.remove();
    onEnd?.(finished);
  }

  function onKey(e) {
    if (e.key === "Escape") { e.preventDefault(); e.stopPropagation(); end(false); }
    else if (e.key === "ArrowRight" || e.key === "Enter") { e.preventDefault(); e.stopPropagation(); go(1); }
    else if (e.key === "ArrowLeft") { e.preventDefault(); e.stopPropagation(); go(-1); }
    else if (e.key !== "Tab") { e.stopPropagation(); }
  }

  function go(d) {
    const n = i + d;
    if (n >= steps.length) return end(true);
    if (n < 0) return;
    i = n;
    show();
  }

  next.addEventListener("click", () => go(1));
  back.addEventListener("click", () => go(-1));
  root.querySelector(".tour-skip").addEventListener("click", () => end(false));
  window.addEventListener("resize", place);
  document.addEventListener("keydown", onKey, true);
  show();
}


