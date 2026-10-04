// Données de démonstration, utilisées seulement quand les pages sont ouvertes
// dans un navigateur (pour travailler le design sans lancer l'app).

const H = 3600e3;
const now = Date.now();
let n = 0;
const id = () => "d" + (++n);

const app = (name, bundle) => ({ id: id(), kind: "app", app_name: name, bundle_id: bundle, label: name, value: bundle, group: null });
const tab = (browser, bundle, title, url, group = 1) => ({ id: id(), kind: "tab", app_name: browser, bundle_id: bundle, label: title, value: url, group });
const folder = (path) => ({ id: id(), kind: "folder", app_name: "Finder", bundle_id: "com.apple.finder", label: path.split("/").pop(), value: path, group: null });
const doc = (appName, bundle, path) => ({ id: id(), kind: "document", app_name: appName, bundle_id: bundle, label: path.split("/").pop(), value: path, group: null });

const S = "com.apple.Safari", C = "com.google.Chrome";

let contexts = [
  {
    id: "c1", name: "Exposé d'histoire : la guerre froide", pinned: true,
    note: "Je finissais la partie sur la crise de Cuba, il manque la conclusion et les sources de la diapo 7.",
    created_at: now - 1.6 * H, updated_at: now - 1.6 * H, last_restored_at: null, restore_count: 2, front_app: "Keynote",
    items: [
      app("Keynote", "com.apple.iWork.Keynote"), app("Safari", S), app("Notes", "com.apple.Notes"), app("Aperçu", "com.apple.Preview"),
      tab("Safari", S, "Crise des missiles de Cuba — Wikipédia", "https://fr.wikipedia.org/wiki/Crise_des_missiles_de_Cuba"),
      tab("Safari", S, "La guerre froide en 10 dates – Lumni", "https://www.lumni.fr/dossier/la-guerre-froide"),
      tab("Safari", S, "Le mur de Berlin – Archives INA", "https://www.ina.fr/mur-de-berlin"),
      tab("Safari", S, "Chronologie 1947-1991", "https://www.histoire.fr/chronologie-guerre-froide", 2),
      folder("/Users/moi/Documents/Lycée/Histoire"),
      doc("Keynote", "com.apple.iWork.Keynote", "/Users/moi/Documents/Lycée/Histoire/Exposé guerre froide.key"),
      doc("Aperçu", "com.apple.Preview", "/Users/moi/Documents/Lycée/Histoire/Manuel chapitre 4.pdf"),
    ],
  },
  {
    id: "c2", name: "Bug d'export PDF", pinned: false,
    note: "Le PDF sort vide quand le nom contient un accent. Piste : l'encodage du chemin dans export.rs, ligne 88.",
    created_at: now - 5 * H, updated_at: now - 5 * H, last_restored_at: now - 3 * H, restore_count: 1, front_app: "Code",
    items: [
      app("Code", "com.microsoft.VSCode"), app("Terminal", "com.apple.Terminal"), app("Google Chrome", C),
      tab("Google Chrome", C, "Issue #42 · export vide avec accents", "https://github.com/exemple/app/issues/42"),
      tab("Google Chrome", C, "std::path::Path - Rust", "https://doc.rust-lang.org/std/path/struct.Path.html"),
      tab("Google Chrome", C, "Percent-encoding — MDN", "https://developer.mozilla.org/fr/docs/Glossary/Percent-encoding"),
      folder("/Users/moi/Projets/app"),
      doc("Code", "com.microsoft.VSCode", "/Users/moi/Projets/app/src/export.rs"),
    ],
  },
  {
    id: "c3", name: "Révisions maths : suites", pinned: false,
    note: "Exercice 12 fait. Revoir la démonstration par récurrence avant jeudi.",
    created_at: now - 26 * H, updated_at: now - 26 * H, last_restored_at: null, restore_count: 0, front_app: "Aperçu",
    items: [
      app("Aperçu", "com.apple.Preview"), app("Safari", S),
      tab("Safari", S, "Suites arithmétiques et géométriques – cours", "https://www.maths-et-tiques.fr/suites"),
      tab("Safari", S, "Raisonnement par récurrence – vidéo", "https://www.youtube.com/watch?v=exemple"),
      doc("Aperçu", "com.apple.Preview", "/Users/moi/Documents/Lycée/Maths/Fiche suites.pdf"),
    ],
  },
  {
    id: "a1", name: "Sauvegarde auto", pinned: false, auto: true, note: "",
    created_at: now - 0.4 * H, updated_at: now - 0.4 * H, last_restored_at: null, restore_count: 0, front_app: "Code",
    items: [app("Code", "com.microsoft.VSCode"), app("Safari", S), tab("Safari", S, "MDN – Array.prototype.map()", "https://developer.mozilla.org/fr/docs/Web/JavaScript/Reference/Global_Objects/Array/map")],
  },
  {
    id: "a2", name: "Sauvegarde auto", pinned: false, auto: true, note: "",
    created_at: now - 20 * H, updated_at: now - 20 * H, last_restored_at: null, restore_count: 0, front_app: "Aperçu",
    items: [app("Aperçu", "com.apple.Preview"), doc("Aperçu", "com.apple.Preview", "/Users/moi/Documents/Lycée/Maths/Fiche suites.pdf")],
  },
  {
    id: "c4", name: "Organiser le voyage à Lisbonne", pinned: false,
    note: "",
    created_at: now - 4 * 24 * H, updated_at: now - 4 * 24 * H, last_restored_at: null, restore_count: 0, front_app: "Safari",
    items: [
      app("Safari", S), app("Numbers", "com.apple.iWork.Numbers"),
      tab("Safari", S, "Auberges de jeunesse à Lisbonne", "https://www.hostelworld.com/lisbonne"),
      tab("Safari", S, "Tram 28 : horaires et itinéraire", "https://www.carris.pt/tram-28"),
      doc("Numbers", "com.apple.iWork.Numbers", "/Users/moi/Documents/Budget Lisbonne.numbers"),
    ],
  },
];

let settings = {
  shortcut_save: "Alt+Command+S", shortcut_open: "Alt+Command+R", font: "atkinson", text_scale: 100,
  tidy_by_default: false, show_note: true, show_in_dock: true, launch_at_login: false, theme: "auto",
  ignored_apps: [{ bundle_id: "com.apple.MobileSMS", name: "Messages" }], check_updates: true, last_update_check: 0,
  shortcut_resume: "Alt+Shift+Command+R", auto_save: true, welcome_back: true, welcome_minutes: 20, seen: [],
  last_version: new URLSearchParams(location.search).get("whatsnew") === "1" ? "1.0.0" : "1.1.0",
  onboarded: new URLSearchParams(location.search).get("onboarding") !== "1",
};

const listeners = {};
const emit = (evt, payload) => (listeners[evt] || []).forEach((f) => f(payload));
const clone = (x) => JSON.parse(JSON.stringify(x));
const wait = (ms) => new Promise((r) => setTimeout(r, ms));

const pendingSnapshot = {
  front_app: "Code",
  warnings: new URLSearchParams(location.search).get("warn") === "1"
    ? ["Reprise n'a pas le droit de piloter Google Chrome. Autorise-le dans Réglages Système › Confidentialité et sécurité › Automatisation."]
    : [],
  taken_at: now,
  items: [
    app("Code", "com.microsoft.VSCode"), app("Safari", S), app("Terminal", "com.apple.Terminal"), app("Spotify", "com.spotify.client"),
    tab("Safari", S, "Issue #42 · export vide avec accents", "https://github.com/exemple/app/issues/42"),
    tab("Safari", S, "std::path::Path - Rust", "https://doc.rust-lang.org/std/path/struct.Path.html"),
    tab("Safari", S, "Percent-encoding — MDN", "https://developer.mozilla.org/fr/docs/Glossary/Percent-encoding"),
    tab("Safari", S, "YouTube – lofi beats to code to", "https://www.youtube.com/watch?v=lofi"),
    tab("Safari", S, "Reddit – r/rust", "https://www.reddit.com/r/rust/"),
    folder("/Users/moi/Projets/app"), folder("/Users/moi/Téléchargements"),
    doc("Code", "com.microsoft.VSCode", "/Users/moi/Projets/app/src/main.rs"),
  ],
};

const demoUpdate = {
  current: "1.1.0", latest: "1.2.0", available: new URLSearchParams(location.search).get("update") === "1", can_install: true,
  url: "https://github.com/RenzVASA/reprise/releases/latest", dmg_url: null,
  notes: "- Les onglets de Firefox sont enregistrés\n- Le Terminal se rouvre dans le bon dossier\n- Corbeille : un contexte supprimé reste 30 jours",
};

export const demo = {
  async invoke(cmd, args) {
    await wait(60);
    switch (cmd) {
      case "list_contexts": return clone(contexts).sort((a, b) => b.created_at - a.created_at);
      case "update_context": {
        const c = contexts.find((x) => x.id === args.id);
        if (args.patch.name != null) c.name = args.patch.name;
        if (args.patch.note != null) c.note = args.patch.note;
        if (args.patch.pinned != null) c.pinned = args.patch.pinned;
        if (args.patch.auto != null) { c.auto = args.patch.auto; if (!c.auto) c.name = "Contexte gardé"; }
        emit("contexts-changed"); return clone(c);
      }
      case "delete_context": {
        const c = contexts.find((x) => x.id === args.id);
        contexts = contexts.filter((x) => x.id !== args.id);
        emit("contexts-changed"); return clone(c);
      }
      case "put_back_context": contexts.push(args.context); emit("contexts-changed"); return null;
      case "restore_context": await wait(700); return { opened: 9, failures: [] };
      case "open_item": return { opened: 1, failures: [] };
      case "start_capture": return null;
      case "get_pending_capture": {
        const ready = new URLSearchParams(location.search).get("loading") !== "1";
        return { ready, front_app: "Code", snapshot: ready ? pendingSnapshot : null, replace: null };
      }
      case "save_capture": return null;
      case "cancel_capture": return null;
      case "get_note": return new URLSearchParams(location.search).get("welcome") === "1"
        ? { id: "c1", name: contexts[0].name, note: contexts[0].note, mode: "welcome", away_ms: 2.3 * H }
        : { id: "c1", name: contexts[0].name, note: contexts[0].note, mode: "note", away_ms: 0 };
      case "resume_last": return null;
      case "mark_seen":
        if (args.key && !settings.seen.includes(args.key)) settings.seen.push(args.key);
        if (args.version) settings.last_version = args.version;
        return clone(settings);
      case "install_update": {
        const total = 3.1 * 1048576;
        for (let d = 0; d <= total; d += total / 12) { await wait(120); emit("update-progress", { downloaded: d, total }); }
        emit("update-installed");
        await wait(400);
        location.reload();
        return null;
      }
      case "close_note": return null;
      case "get_settings": return clone(settings);
      case "set_settings": settings = clone(args.settings); emit("settings-changed", clone(settings)); return clone(settings);
      case "pause_shortcuts": return null;
      case "ignore_app":
        if (!settings.ignored_apps.some((a) => a.bundle_id === args.bundleId)) settings.ignored_apps.push({ bundle_id: args.bundleId, name: args.name });
        emit("settings-changed", clone(settings)); return clone(settings);
      case "check_updates": await wait(600); return demoUpdate;
      case "get_update": return new URLSearchParams(location.search).get("update") === "1" ? demoUpdate : null;
      case "check_permissions": await wait(500); return { automation: "ok", accessibility: "denied" };
      case "app_info": return { version: "1.1.0", data_path: "~/Library/Application Support/io.github.renzvasa.reprise/contexts.json" };
      default: return null;
    }
  },
  listen(evt, fn) {
    (listeners[evt] ||= []).push(fn);
    return Promise.resolve(() => {});
  },
};
