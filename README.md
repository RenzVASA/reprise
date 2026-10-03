<p align="center">
  <img src="docs/icon.png" width="128" height="128" alt="Icône de Reprise">
</p>

<h1 align="center">Reprise</h1>

<p align="center">
  <strong>Sauvegarde ton contexte de travail en un raccourci. Reprends-le en un clic.</strong><br>
  Pour Mac Apple Silicon (M1 et suivants) · gratuit · open source · tout reste sur ton Mac
</p>

<p align="center">
  <a href="https://github.com/RenzVASA/reprise/releases/latest"><b>Télécharger la dernière version</b></a>
</p>

<p align="center">
  <img src="docs/screenshots/main_light.png" width="820" alt="La fenêtre principale de Reprise">
</p>

---

## Le problème

Tu travailles sur un truc. Trois fenêtres de navigateur, douze onglets, un dossier ouvert dans le Finder, un fichier dans ton éditeur, une note à moitié écrite. Puis on t'interrompt : une pause, un cours, un autre projet, le lendemain.

Quand tu reviens, tout est mélangé, et tu perds vingt minutes à te souvenir de ce que tu faisais.

## Ce que fait Reprise

1. **Tu appuies sur ⌥⌘S.** Reprise photographie tes apps ouvertes, les onglets de tes navigateurs, tes dossiers du Finder et les fichiers ouverts dans tes apps.
2. **Tu écris une phrase : « où j'en étais ».** C'est elle qui fait toute la différence le lendemain. Des débuts de phrase sont proposés pour ne jamais rester bloqué devant une page blanche.
3. **Plus tard, tu cliques sur « Reprendre ».** Tout se rouvre à sa place, fenêtres de navigateur comprises, et ta phrase s'affiche sur un post-it en haut de l'écran.

<p align="center">
  <img src="docs/screenshots/capture.png" width="380" alt="La fenêtre de sauvegarde">
  &nbsp;&nbsp;
  <img src="docs/screenshots/note.png" width="300" alt="Le post-it « Tu en étais là »">
</p>

## Pensé pour les gens qui sautent d'une tâche à l'autre

Reprise est utile à tout le monde, mais il a été pensé d'abord pour les personnes neuroatypiques (TDAH, troubles dys, autisme…) pour qui « revenir dedans » après une interruption coûte cher.

- **Un seul geste à retenir.** Le raccourci marche depuis n'importe quelle app. Entrée pour valider, Échap pour annuler.
- **Polices de lecture au choix** : Atkinson Hyperlegible (par défaut), Lexend, OpenDyslexic ou la police du système. Taille du texte réglable.
- **Faire place nette** : en reprenant un contexte, Reprise peut masquer toutes les autres apps pour que tu ne voies que ce qui compte.
- **Choisir ce qu'on garde** : au moment de sauvegarder, clique sur « 4 apps » ou « 12 onglets » et décoche ce qui n'a rien à voir.
- **Apps jamais enregistrées** : Spotify, Messages, Discord… ne viennent plus jamais polluer tes contextes.
- **Rouvrir seulement une partie** : décoche ce dont tu n'as plus besoin avant de reprendre.
- **Mode clair, mode sombre**, et respect du réglage « Réduire les animations ».

## Toutes les fonctions

| | |
|---|---|
| Capture | Apps ouvertes, onglets de Safari, Chrome, Brave, Edge, Arc, Vivaldi (fenêtre par fenêtre), dossiers du Finder, fichiers ouverts (Aperçu, Pages, Keynote, Numbers, TextEdit, Xcode, VS Code et beaucoup d'autres) |
| Reprise | Rouvre tout, ou seulement ce que tu coches. Recrée les fenêtres de navigateur avec leurs onglets dans l'ordre |
| Choix | Décocher des éléments au moment de sauvegarder, liste d'apps à ne jamais enregistrer |
| Barre des menus | Sauvegarde et reprise des contextes récents sans ouvrir la fenêtre |
| Mises à jour | Vérification automatique (une fois par jour au plus) ou à la demande, avec lien vers la nouvelle version |
| Aide | Le bouton « i » explique ce que fait Reprise, ce qui est enregistré, comment ça marche et les raccourcis |
| Organisation | Recherche dans les noms, les notes et les onglets, épinglage, renommage, mise à jour d'un contexte avec ce qui est ouvert maintenant, suppression avec « Annuler » |
| Raccourcis | ⌥⌘S pour sauvegarder, ⌥⌘R pour ouvrir Reprise, personnalisables. Dans la fenêtre : ↑ ↓ pour naviguer, Entrée pour reprendre, ⌘F pour chercher, ⌘N pour sauvegarder |
| Réglages | Lancement à l'ouverture de session, icône dans le Dock ou seulement dans la barre des menus, post-it activable |

## Installation

1. Télécharge le fichier `.dmg` depuis la page [Releases](https://github.com/RenzVASA/reprise/releases/latest).
2. Ouvre-le et glisse **Reprise** dans **Applications**.
3. Reprise n'est pas signée par Apple (un compte développeur coûte 99 $ par an). macOS va donc dire que l'app est « endommagée » : ce n'est pas vrai, c'est juste qu'il ne la connaît pas. Ouvre le **Terminal** et colle :

   ```bash
   xattr -cr /Applications/Reprise.app
   ```

4. Lance Reprise. Un petit accueil en trois étapes t'explique tout.

### Les autorisations demandées

macOS protège tes apps, donc il va te demander deux choses :

- **Automatisation** : pour que Reprise puisse lire tes onglets et tes dossiers, puis les rouvrir. macOS pose la question une fois par app (Safari, Chrome, Finder…).
- **Accessibilité** (facultatif) : pour savoir quels fichiers sont ouverts dans tes apps. Sans elle, tout le reste marche.

Tu peux revoir ces autorisations à tout moment dans **Réglages Système › Confidentialité et sécurité**, ou depuis les réglages de Reprise.

### Tes données

Tout est stocké dans un simple fichier JSON sur ton Mac :
`~/Library/Application Support/io.github.renzvasa.reprise/contexts.json`.
Rien n'est envoyé sur Internet. Aucun compte, aucune pub, aucun traceur. La seule connexion sert à demander à GitHub s'il existe une nouvelle version, et elle se coupe dans les réglages.

## Limites connues

- **Firefox** ne laisse pas les autres apps lire ses onglets : l'app est sauvegardée, mais pas les onglets.
- Les onglets de **navigation privée** ne sont pas toujours lisibles.
- Une app qui n'expose pas ses fichiers ouverts à l'accessibilité sera rouverte, mais sans ses fichiers.
- Reprise rouvre les choses, elle ne peut pas restaurer l'état exact d'une app (position dans une vidéo, texte non enregistré…). C'est le rôle de ta phrase « où j'en étais ».

## Compiler soi-même

Il te faut [Node.js](https://nodejs.org) et [Rust](https://rustup.rs).

```bash
npm install          # installe l'outil Tauri
npm run dev          # lance Reprise en mode développement
npm run build        # fabrique Reprise.app et le .dmg dans src-tauri/target/release/bundle/
```

Pour travailler le design sans lancer l'app : `npm run apercu`, puis ouvre <http://localhost:8765> dans un navigateur. Les pages utilisent alors des données de démonstration.

### Comment c'est construit

- **Tauri 2 + Rust** pour le cœur : capture et reprise via AppleScript (`osascript`) et la commande `open`, raccourcis globaux, barre des menus, lancement au démarrage.
- **HTML, CSS et JavaScript** sans framework ni étape de compilation pour l'interface (dossier `src/`).
- **Un fichier JSON** pour les données, écrit de façon atomique pour ne jamais rien perdre.

```
src/                 interface : fenêtre principale, fenêtre de sauvegarde, post-it
src-tauri/src/
  lib.rs             démarrage, fenêtres, barre des menus, raccourcis, commandes
  capture.rs         la photo du contexte
  restore.rs         la remise en place
  script.rs          les scripts AppleScript et la lecture de leurs réponses
  store.rs           le stockage des contextes
  settings.rs        les réglages
  update.rs          la recherche de nouvelle version sur GitHub
```

Les tests du cœur se lancent avec `cargo test --manifest-path src-tauri/Cargo.toml`.

### Publier une nouvelle version

1. Change le numéro de version dans `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml` et `package.json`.
2. Pousse un tag : `git tag v1.1.0 && git push origin v1.1.0`.
3. GitHub Actions fabrique le `.dmg` et crée la Release tout seul.

## Idées pour la suite

- Sauvegarde automatique quand l'écran se verrouille (« filet de sécurité »).
- Rappel doux si un contexte épinglé n'a pas été repris depuis longtemps.
- Export et import des contextes.

## Crédits

Reprise est vibe codée par **RenzVASA**, avec l'aide de Claude, l'IA d'Anthropic.

Polices sous licence SIL Open Font License : Bricolage Grotesque, Atkinson Hyperlegible, Lexend, OpenDyslexic (licences dans `src/fonts/licences/`).

Licence du code : [MIT](LICENSE).
