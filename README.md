<p align="center">
  <img src="docs/icon.png" width="128" height="128" alt="Icône de Reprise">
</p>

<h1 align="center">Reprise</h1>

<p align="center">
  <strong>Sauvegarde ton contexte de travail en un raccourci. Reprends-le en un clic.</strong><br>
  Pour Mac, Windows et Linux · gratuit · open source · tout reste sur ton ordinateur
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

1. **Tu appuies sur ⌥⌘S** (Ctrl+Alt+S sur Windows et Linux). Reprise photographie tes apps ouvertes, les onglets de tes navigateurs, tes dossiers et les fichiers ouverts dans tes apps.
2. **Tu écris une phrase : « où j'en étais ».** C'est elle qui fait toute la différence le lendemain. Des débuts de phrase sont proposés pour ne jamais rester bloqué devant une page blanche.
3. **Plus tard, tu cliques sur « Reprendre ».** Tout se rouvre, chaque fenêtre à sa place et à sa taille, et ta phrase s'affiche sur un post-it en haut de l'écran.

Et si tu oublies d'appuyer sur le raccourci ? Le **filet de sécurité** s'en occupe : Reprise sauvegarde toute seule quand tu verrouilles ton ordinateur, qu'il se met en veille ou que tu t'absentes. En revenant, un post-it te demande : « Tu étais sur… on reprend ? »

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
- **Mode clair, mode sombre** et **habillages** (Classique, Forêt, Océan, plus Halloween et Noël qui arrivent tout seuls à leur saison, icône du Dock comprise), avec respect du réglage « Réduire les animations ».

## Toutes les fonctions

| | |
|---|---|
| Capture | Apps ouvertes, onglets de Safari, Chrome, Brave, Edge, Arc, Vivaldi, Opera, Opera GX (fenêtre par fenêtre), dossiers du Finder, fichiers ouverts (Aperçu, Pages, Keynote, Numbers, TextEdit, Xcode, VS Code et beaucoup d'autres) |
| Reprise | Rouvre tout, ou seulement ce que tu coches. Recrée les fenêtres de navigateur avec leurs onglets dans l'ordre, et remet chaque fenêtre à sa place et à sa taille, y compris sur un deuxième écran |
| Filet de sécurité | Sauvegarde automatique au verrouillage, à la mise en veille ou après 5 minutes d'inactivité, sauf pendant une vidéo ou un stream. Les 3 dernières sont rangées dans un dossier qui s'active ou se désactive d'un clic. Post-it « Tu étais sur… » au retour |
| Choix | Décocher des éléments au moment de sauvegarder, liste d'apps à ne jamais enregistrer |
| Barre des menus | Sauvegarde et reprise des contextes récents sans ouvrir la fenêtre |
| Habillages | Classique, Forêt, Océan, en clair ou en sombre. Halloween (15 octobre → 2 novembre) et Noël (1er décembre → 6 janvier) arrivent et repartent tout seuls. L'icône du Dock et le post-it suivent l'habillage |
| Mises à jour | Reprise prévient quand une nouvelle version sort, la télécharge, vérifie sa signature, l'installe et redémarre, en un clic |
| Aide | Visite guidée au premier lancement, astuce à la première sauvegarde, et le bouton « i » qui explique tout |
| Organisation | Recherche dans les noms, les notes et les onglets, épinglage, renommage, mise à jour d'un contexte avec ce qui est ouvert maintenant, suppression avec « Annuler » |
| Raccourcis | ⌥⌘S pour sauvegarder, ⌥⌘R pour ouvrir Reprise, ⌥⇧⌘R pour reprendre le dernier contexte, personnalisables. Dans la fenêtre : ↑ ↓ pour naviguer, Entrée pour reprendre, ⌘F pour chercher, ⌘N pour sauvegarder |
| Réglages | Lancement à l'ouverture de session, icône dans le Dock ou seulement dans la barre des menus, post-it activable |

## Installation

Tout se télécharge sur la page [Releases](https://github.com/RenzVASA/reprise/releases/latest).

### Mac (Apple Silicon, macOS 11 ou plus récent)

1. Télécharge le fichier `.dmg`, ouvre-le et glisse **Reprise** dans **Applications**.
2. Reprise n'est pas signée par Apple (un compte développeur coûte 99 $ par an). macOS va donc dire que l'app est « endommagée » : ce n'est pas vrai, c'est juste qu'il ne la connaît pas. Ouvre le **Terminal** et colle :

   ```bash
   xattr -cr /Applications/Reprise.app
   ```

3. Lance Reprise. Un petit accueil t'explique tout.

### Windows 10 et 11

1. Télécharge le fichier `Reprise_…_x64-setup.exe` et lance-le. Pas besoin d'être administrateur.
2. Windows peut afficher « Windows a protégé votre ordinateur » : Reprise n'a pas de certificat Microsoft payant. Clique sur **Informations complémentaires**, puis **Exécuter quand même**.
3. Reprise s'ouvre, et vit ensuite près de l'horloge, dans la zone de notification.

### Linux

- **N'importe quelle distribution** : télécharge l'`.AppImage`, rends-la exécutable (`chmod +x Reprise_*.AppImage`) et lance-la. C'est cette version qui se met à jour toute seule.
- **Debian, Ubuntu, Mint…** : `sudo apt install ./Reprise_*_amd64.deb`.

Le mieux est un bureau GNOME ou KDE. Sous Wayland, les raccourcis globaux peuvent ne pas marcher dans toutes les apps : l'icône dans la zone de notification fait la même chose.

### Vérifier que le fichier est authentique

Chaque version publie l'**empreinte SHA-256** de chaque installateur (dans le texte de la Release et dans le fichier `SHA256SUMS.txt`). C'est une sorte d'empreinte digitale : si le fichier a été modifié, même d'un seul octet, l'empreinte change complètement.

```bash
shasum -a 256 ~/Downloads/Reprise_*_aarch64.dmg        # Mac
sha256sum ~/Downloads/Reprise_*.AppImage                # Linux
```

Sous Windows, dans PowerShell : `Get-FileHash $HOME\Downloads\Reprise_*_x64-setup.exe`.

Compare le résultat avec l'empreinte affichée sur la page de la Release : les deux doivent être identiques.

Chaque installateur est fabriqué directement par GitHub Actions à partir du code public de ce dépôt, et GitHub signe sa provenance. Pour le vérifier (avec l'outil [GitHub CLI](https://cli.github.com)) :

```bash
gh attestation verify <le fichier téléchargé> -R RenzVASA/reprise
```

### Signée RenzVASA (Mac)

Depuis la version 1.1.2, l'app Mac est signée avec le certificat RenzVASA. Ce n'est pas un certificat payant d'Apple (d'où la commande `xattr` ci-dessus), mais il permet à macOS de reconnaître Reprise d'une version à l'autre : tes autorisations (Automatisation, Accessibilité) restent en place après chaque mise à jour.

### Les autorisations demandées (Mac)

macOS protège tes apps, donc il va te demander deux choses :

- **Automatisation** : pour que Reprise puisse lire tes onglets et tes dossiers, puis les rouvrir. macOS pose la question une fois par app (Safari, Chrome, Finder…).
- **Accessibilité** (facultatif) : pour savoir quels fichiers sont ouverts dans tes apps. Sans elle, tout le reste marche.

Tu peux revoir ces autorisations à tout moment dans **Réglages Système › Confidentialité et sécurité**, ou depuis les réglages de Reprise.

Sous Windows et Linux, aucune autorisation n'est demandée.

### Tes données

Tout est stocké dans un simple fichier JSON sur ton ordinateur :

- Mac : `~/Library/Application Support/io.github.renzvasa.reprise/contexts.json`
- Windows : `%APPDATA%\io.github.renzvasa.reprise\contexts.json`
- Linux : `~/.local/share/io.github.renzvasa.reprise/contexts.json`

Rien n'est envoyé sur Internet. Aucun compte, aucune pub, aucun traceur. La seule connexion sert à demander à GitHub s'il existe une nouvelle version, et elle se coupe dans les réglages.

## Limites connues

- **Sur Mac, Firefox** ne laisse pas les autres apps lire ses onglets : l'app est sauvegardée, mais pas les onglets.
- Les onglets de **navigation privée** ne sont pas toujours lisibles.
- Une app qui n'expose pas ses fichiers ouverts à l'accessibilité sera rouverte, mais sans ses fichiers.
- **Sur Windows et Linux**, les onglets sont lus dans le fichier où chaque navigateur note sa session : il le met à jour toutes les quelques secondes, donc un onglet ouvert à l'instant peut manquer. Les fichiers retrouvés sont ceux que tu as ouverts en double-cliquant dessus, pas ceux ouverts depuis l'app. Sur Windows, la place des fenêtres de navigateur n'est pas remise ; sur Linux, la place des fenêtres et « Faire place nette » n'existent pas encore.
- **Sur Linux**, les dossiers ouverts dans le gestionnaire de fichiers ne sont pas encore retrouvés.
- Reprise rouvre les choses, elle ne peut pas restaurer l'état exact d'une app (position dans une vidéo, texte non enregistré…). C'est le rôle de ta phrase « où j'en étais ».

## Compiler soi-même

Il te faut [Node.js](https://nodejs.org) et [Rust](https://rustup.rs). Sous Windows, il faut aussi les **Visual Studio Build Tools** (« Développement Desktop en C++ ») ; sous Linux, les bibliothèques de WebKitGTK (voir [les prérequis de Tauri](https://v2.tauri.app/start/prerequisites/)).

```bash
npm install          # installe l'outil Tauri
npm run dev          # lance Reprise en mode développement
npm run build        # fabrique l'app et son installateur dans src-tauri/target/release/bundle/
```

`npm run build` signe aussi l'archive de mise à jour : il lui faut la clé privée. Si tu l'as créée avec le script de publication, elle est dans `~/.tauri/reprise.key` :

```bash
export TAURI_SIGNING_PRIVATE_KEY="$(cat ~/.tauri/reprise.key)"
export TAURI_SIGNING_PRIVATE_KEY_PASSWORD="$(cat ~/.tauri/reprise.password)"
npm run build
```

Pour travailler le design sans lancer l'app : `npm run apercu`, puis ouvre <http://localhost:8765> dans un navigateur. Les pages utilisent alors des données de démonstration.

### Comment c'est construit

- **Tauri 2 + Rust** pour le cœur : raccourcis globaux, barre des menus, lancement au démarrage, et la capture propre à chaque système :
  - **Mac** : AppleScript (`osascript`) et la commande `open` ;
  - **Windows** : PowerShell pour la liste des apps et des dossiers de l'Explorateur, et des appels directs à Windows pour la place des fenêtres, « Faire place nette » et l'absence ;
  - **Linux** : `/proc` et les raccourcis d'apps (`.desktop`), systemd-logind et GNOME pour l'absence ;
  - **Windows et Linux** : les onglets sont lus dans les fichiers de session des navigateurs (Firefox, et le format « SNSS » de Chrome, Edge, Brave, Vivaldi, Opera).
- **HTML, CSS et JavaScript** sans framework ni étape de compilation pour l'interface (dossier `src/`).
- **Un fichier JSON** pour les données, écrit de façon atomique pour ne jamais rien perdre.

```
src/                 interface : fenêtre principale, fenêtre de sauvegarde, post-it
  css/themes.css     les habillages (Forêt, Océan, Halloween, Noël)
design/styles/       les icônes du Dock de chaque habillage (SVG)
src-tauri/src/
  lib.rs             démarrage, fenêtres, barre des menus, raccourcis, commandes
  platform.rs        le point d'entrée de tout ce qui dépend du système
  capture.rs         la photo du contexte (Mac)
  restore.rs         le plan de remise en place, et son exécution sur Mac
  script.rs          les scripts AppleScript et la lecture de leurs réponses (Mac)
  platform_windows.rs, platform_linux.rs   la capture et l'absence sur Windows et Linux
  desktop.rs         la photo et la reprise communes à Windows et Linux
  sessions.rs        la lecture des sessions des navigateurs (Windows et Linux)
  sys.rs             petits outils (lancer un programme sans fenêtre noire…)
  store.rs           le stockage des contextes
  settings.rs        les réglages
  update.rs          la recherche de nouvelle version sur GitHub
  watch.rs           le filet de sécurité (verrouillage, veille, inactivité)
```

Les tests du cœur se lancent avec `cargo test --manifest-path src-tauri/Cargo.toml`.

### Publier une nouvelle version

1. Change le numéro de version dans `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml` et `package.json`.
2. Ajoute une section pour cette version en haut de `CHANGELOG.md` : c'est ce texte qui s'affiche dans la fenêtre de mise à jour.
3. Pousse un tag : `git tag v1.5.0 && git push origin v1.5.0`.
4. GitHub Actions fabrique, sur un Mac, un PC Windows et un PC Linux, les installateurs, les fichiers de mise à jour signés et `latest.json`, puis crée la Release. Les utilisateurs sont prévenus tout seuls.

### Comment marchent les mises à jour

Chaque Release contient un fichier `latest.json` qui annonce la dernière version. Reprise le lit, et si elle est plus récente, propose de l'installer. L'archive téléchargée est **signée** avec une clé privée que seul le propriétaire du dépôt possède (dans les secrets GitHub) ; Reprise vérifie cette signature avec la clé publique écrite dans `tauri.conf.json` avant de remplacer quoi que ce soit. Une archive modifiée par quelqu'un d'autre est refusée.

**Ne perds pas `~/.tauri/reprise.key`** : sans elle, impossible de signer les prochaines versions, et il faudrait que tout le monde réinstalle à la main.

## Idées pour la suite

- Les onglets de Firefox sur Mac, lus dans son fichier de session (comme sur Windows et Linux).
- La place des fenêtres et « Faire place nette » sur Linux (sous X11).
- Le Terminal et VS Code rouverts dans le bon dossier.
- Une mini-liste de prochaines étapes à cocher sur le post-it.
- Une corbeille : un contexte supprimé reste 30 jours.
- Export et import des contextes.
- D'autres habillages de saison (printemps, rentrée…).

## Crédits

Reprise est vibe codée par **RenzVASA**, avec l'aide de Claude, l'IA d'Anthropic.

Polices sous licence SIL Open Font License : Bricolage Grotesque, Atkinson Hyperlegible, Lexend, OpenDyslexic (licences dans `src/fonts/licences/`).

Licence du code : [MIT](LICENSE).
