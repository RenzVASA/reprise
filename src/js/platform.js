// Le système sur lequel tourne Reprise, et les mots qui vont avec.
// On le devine tout de suite (sans attendre le cœur Rust) pour que rien ne clignote.

const ua = navigator.userAgent || "";
export const OS = /Windows/i.test(ua) ? "windows" : /Mac OS X|Macintosh/i.test(ua) ? "macos" : /Linux|X11/i.test(ua) ? "linux" : "macos";
export const isMac = OS === "macos";

document.documentElement.classList.add(`os-${OS}`);

/** Les mots qui changent d'un système à l'autre. */
export const T = {
  macos: {
    computer: "ton Mac",
    computerCap: "Ton Mac",
    lockVerb: "verrouilles ton Mac",
    dock: "le Dock",
    dockIcon: "Icône dans le Dock",
    dockHint: "Sinon, Reprise vit seulement dans la barre des menus.",
    tray: "la barre des menus",
    fileManager: "le Finder",
    reveal: "Montrer dans le Finder",
    appsCaptured: "Les <strong>apps</strong> ouvertes (celles qui ont une icône dans le Dock).",
    foldersCaptured: "Les <strong>dossiers</strong> ouverts dans le Finder.",
    filesCaptured: "Les <strong>fichiers</strong> ouverts dans tes apps (Aperçu, Pages, VS Code…).",
    modName: "⌘",
  },
  windows: {
    computer: "ton PC",
    computerCap: "Ton PC",
    lockVerb: "verrouilles ton PC",
    dock: "la barre des tâches",
    dockIcon: "Icône dans la barre des tâches",
    dockHint: "Sinon, Reprise vit seulement près de l'horloge, dans la zone de notification.",
    tray: "la zone de notification, près de l'horloge",
    fileManager: "l'Explorateur",
    reveal: "Montrer dans l'Explorateur",
    appsCaptured: "Les <strong>apps</strong> ouvertes (celles qui ont une fenêtre).",
    foldersCaptured: "Les <strong>dossiers</strong> ouverts dans l'Explorateur.",
    filesCaptured: "Les <strong>fichiers</strong> que tu as ouverts en double-cliquant dessus (Word, Bloc-notes, PDF…).",
    modName: "Ctrl+",
  },
  linux: {
    computer: "ton ordinateur",
    computerCap: "Ton ordinateur",
    lockVerb: "verrouilles ta session",
    dock: "la barre des tâches",
    dockIcon: "Icône dans la barre des tâches",
    dockHint: "Sinon, Reprise vit seulement dans la zone de notification.",
    tray: "la zone de notification",
    fileManager: "le gestionnaire de fichiers",
    reveal: "Montrer dans les fichiers",
    appsCaptured: "Les <strong>apps</strong> ouvertes (celles qui ont un raccourci dans le menu des applications).",
    foldersCaptured: "",
    filesCaptured: "Les <strong>fichiers</strong> que tu as ouverts en double-cliquant dessus (PDF, textes, documents…).",
    modName: "Ctrl+",
  },
}[OS];

/** « Faire place nette » et la place des fenêtres : Mac et Windows (pas encore Linux). */
export const canTidy = OS !== "linux";

/** La touche des raccourcis de l'app : ⌘ sur Mac, Ctrl ailleurs. */
export const mod = (e) => (isMac ? e.metaKey : e.ctrlKey);

/** Raccourcis par défaut, comme dans le cœur Rust. */
export const DEFAULTS = isMac
  ? { save: "Alt+Command+S", open: "Alt+Command+R", resume: "Alt+Shift+Command+R" }
  : { save: "Ctrl+Alt+S", open: "Ctrl+Alt+R", resume: "Ctrl+Alt+Shift+R" };

/** Un raccourci de l'app à afficher : « ⌘F » sur Mac, « Ctrl+F » ailleurs. */
export const appKey = (k) => (isMac ? `⌘${k}` : `Ctrl+${k}`);
