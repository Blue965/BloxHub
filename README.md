# BloxHub 🔴

**Ta communauté Roblox en app de bureau — chat façon Discord, éditeur de FastFlags, lanceur intégré et profils Roblox. Tout en Rust.**

> ⚠️ Projet communautaire **non officiel**. Ni affilié à, ni approuvé par Roblox Corporation.
> Les FastFlags ne sont pas officiellement supportés par Roblox : utilise-les à tes propres risques.

---

## ✨ Fonctionnalités

| Module | Description |
|---|---|
| 💬 **Chat communautaire** | Serveurs/salons/DM style Discord : 5 salons, messages temps réel, indicateurs « en train d'écrire », présence en ligne, historique persistant, anti-flood. Serveur WebSocket autonome inclus. |
| ⚡ **Éditeur de FastFlags** | Catalogue de 23 flags connus documentés (FR/EN), recherche, filtres par catégorie, presets prêts à l'emploi (FPS Max, Équilibré, Qualité Max, Latence, Télémétrie OFF), éditeur avancé clé/valeur, import/export JSON, **sauvegarde automatique** de l'ancien fichier avant chaque application. |
| 🚀 **Lanceur** | Détection automatique de l'installation Roblox, lancement du jeu, connexion directe à une expérience par Place ID, indicateur « Roblox tourne ». |
| 👤 **Profils Roblox** | Recherche par pseudo via l'API publique : avatar, badge vérifié, date de création, description, liste d'amis avec photos. |
| 🌍 **FR + EN** | Sélecteur de langue dans les paramètres. |
| 🎨 **Thème Roblox** | Interface sombre noir/gris aux accents rouge Roblox (#E2231A). |

---

## 🧱 Technique

- **Tauri 2** (Rust) — fenêtre native légère, interface web sans bundler Node.
- Workspace à 3 crates :
  - `core/` → **bloxhub-core** : toute la logique (FastFlags, lanceur, API Roblox, client chat) — testable sans Tauri ;
  - `src-tauri/` → **bloxhub** : la coquille Tauri (commandes `invoke`, relais d'événements) ;
  - `server/` → **bloxhub-server** : serveur de chat WebSocket autonome (binaire séparé).
- Frontend dans `ui/` : HTML/CSS/JS pur, i18n maison, mode démo dans le navigateur (sans Tauri).

---

## 📋 Prérequis

1. **Rust 1.77+** → [rustup.rs](https://rustup.rs) (vérifie avec `cargo --version`).
2. Selon ton OS (pour le build Tauri) :
   - **Windows 10/11** : rien à faire, WebView2 est inclus dans Windows. Pour le bundle `.msi`/NSIS, installe **Visual Studio Build Tools** (charge de travail « Développement Desktop C++ »).
   - **macOS** : Xcode Command Line Tools (`xcode-select --install`).
   - **Linux (Debian/Ubuntu)** :
     ```bash
     sudo apt update
     sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget file \
       libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev
     ```
     (Fedora : `webkit2gtk4.1-devel openssl-devel curl gcc gcc-c++ …` — voir la doc Tauri.)

---

## 🚀 Lancer BloxHub en développement

```bash
cd bloxhub
cargo run -p bloxhub
```

C'est tout ! La fenêtre BloxHub s'ouvre. (Alternative équivalente : `cargo tauri dev` après `cargo install tauri-cli`.)

> 💡 **Aperçu navigateur** : tu peux ouvrir `ui/index.html` directement dans un navigateur — l'app se met en **mode démo** (données fictives) pour tester l'interface sans Rust.

---

## 📦 Compiler la version finale

### Binaire simple
```bash
cargo build --release -p bloxhub
# → bloxhub/target/release/bloxhub(.exe)
```

### Installeur complet (recommandé)
```bash
cargo install tauri-cli --version "^2"
cargo tauri build
```
Résultats dans `target/release/bundle/` :
- **Windows** : `.msi` + installateur NSIS `.exe`
- **Linux** : `.deb` + `.AppImage` + `.rpm`
- **macOS** : `.app` + `.dmg`

---

## 💬 Le serveur de chat

L'app se connecte par défaut à `ws://127.0.0.1:8787` (serveur local).

### Démarrer le serveur
```bash
cargo run -p bloxhub-server -- --port 8787 --data ./bloxhub-chat-data
```
- `--port <n>` : port d'écoute (défaut **8787**)
- `--data <dir>` : dossier de l'historique (défaut `./bloxhub-chat-data`, 200 derniers messages par salon, persistés dans `history.json`)

Salons par défaut : `general`, `fastflags`, `jeux`, `aide`, `off-topic`.

### Jouer avec tes amis (LAN)
1. Lance le serveur sur ton PC.
2. Trouve ton IP locale (`ipconfig` sur Windows, `ip a` sur Linux).
3. Ouvre le port **8787** en TCP dans ton pare-feu.
4. Tes amis mettent `ws://TON-IP:8787` dans **Paramètres → Chat → Adresse du serveur**, puis relancent la connexion (bouton Ouvrir le chat / redémarrage de l'app).

### Sur un VPS
Le serveur n'a **pas de TLS** intégré. Sur un VPS public, place-le derrière un reverse proxy TLS, par exemple avec Caddy :
```
chat.tondomaine.fr {
    reverse_proxy 127.0.0.1:8787
}
```
Puis dans BloxHub : `wss://chat.tondomaine.fr`. (En local/LAN, `ws://` direct est ok.)

> 🔐 Note : le serveur v1 est fait pour un cercle d'amis — pas d'authentification par mot de passe, pseudos uniques seulement.

---

## ⚡ Guide des FastFlags

1. Onglet **FastFlags** → choisis un **preset** (ou coche des flags dans le **Catalogue**, ou passe en **Avancé** pour des clés custom).
2. Clique **Appliquer** → BloxHub écrit `%LOCALAPPDATA%\Roblox\Versions\<version>\ClientSettings\ClientAppSettings.json` (Windows), avec **sauvegarde** `ClientAppSettings.json.bak-<horodatage>` à côté.
3. **Redémarre Roblox** — les flags se lisent uniquement au démarrage du client.
4. Pour revenir en arrière : bouton **Tout retirer**, ou restaure le fichier `.bak`.

- Chemins gérés : Windows (`%LOCALAPPDATA%\Roblox\Versions`), macOS (`/Applications/Roblox.app`), et **chemin personnalisé** dans Paramètres (Linux/Wine, installs non standard).
- Import/Export : JSON du format `{"FFlag…": valeur}` — compatible avec les listes de la communauté.
- ⚠️ Un flag peut être ignoré ou retiré par Roblox à tout moment ; certains presets modifient le rendu (éclairage Voxel, pas d'ombres) — c'est voulu.

---

## 🧪 Tests

```bash
# Compilation vérifiée
cargo check

# Tests d'intégration client↔serveur (démarre d'abord le serveur)
cargo run -p bloxhub-server -- --port 8799 --data /tmp/chat-test &
cargo test -p bloxhub-core --test chat_client

# Scénario protocolaire complet (2 clients, DMs, flood, persistance)
python3 scripts/test_chat_server.py   # 11 vérifications
```

---

## 🌐 Site & téléchargements

La page d'accueil **`index.html`** (à la racine) est le site officiel de téléchargement : installateur `.exe`, version portable et code source, en français et en anglais.

- Les binaires Windows précompilés sont dans **`downloads/`** :
  - `BloxHub_0.1.0_x64-setup.exe` — installateur NSIS (~3 Mo) ;
  - `bloxhub-windows-v0.1.0.zip` — version portable complète (~7,4 Mo).

### Activer le site GitHub Pages

1. GitHub → ton dépôt → **Settings → Pages** ;
2. *Source* : « Deploy from a branch », branche **`main`**, dossier **`/ (root)`** ;
3. Le site est alors servi sur `https://blue965.github.io/BloxHub/` et les boutons de téléchargement pointent directement vers les fichiers du dépôt.

> 💡 Astuce : pour des versions futures, préfère joindre les binaires à une **Release GitHub** (Releases → New release → attache les fichiers) et mets à jour les liens de `index.html`.

---

## 🗂️ Structure du projet

```
bloxhub/
├── index.html              # page d'accueil du site (téléchargements, FR/EN)
├── downloads/              # binaires Windows précompilés (exe + zip portable)
├── Cargo.toml              # workspace (3 crates)
├── core/                   # bloxhub-core : logique pure
│   ├── src/fflags.rs       #   détection Roblox, lecture/écriture flags, catalogue, presets
│   ├── src/launcher.rs     #   lancement, protocole roblox://, processus
│   ├── src/rbxapi.rs       #   API publique Roblox (profil, avatars, amis)
│   ├── src/chat.rs         #   client WebSocket (reconnexion auto)
│   ├── src/settings.rs     #   réglages persistants
│   └── tests/chat_client.rs#   tests d'intégration
├── src-tauri/              # coquille Tauri 2
│   ├── tauri.conf.json     #   config fenêtre + bundle
│   ├── capabilities/       #   permissions
│   ├── icons/              #   icônes générées (PNG/ICO/ICNS)
│   └── src/lib.rs          #   commandes invoke + relais chat-event
├── server/                 # bloxhub-server : serveur de chat autonome
│   └── src/main.rs         #   salons, DMs, présence, historique, anti-flood
├── ui/                     # frontend sans bundler
│   ├── index.html          #   structure (5 vues)
│   ├── styles.css          #   thème sombre Roblox
│   ├── i18n.js             #   traductions FR/EN
│   └── app.js              #   logique UI + mode démo navigateur
└── scripts/
    ├── gen_icons.py        # régénère les icônes
    └── test_chat_server.py # scénario protocolaire
```

## ➕ Ajouter un flag ou un preset

Tout se passe dans `core/src/fflags.rs` :
- **Catalogue** : ajoute une entrée dans `catalog()` (`clé, valeur, type, catégorie, desc_fr, desc_en`).
- **Preset** : ajoute un `Preset { …, flags: map!{ "DFInt…" => … } }` dans `presets()`.
L'UI et le bouton **Appliquer** reprennent automatiquement tes ajouts (les presets s'ajoutent à la pending-map, puis un clic Applique écrit le JSON).

---

## 🛠️ Dépannage

| Problème | Solution |
|---|---|
| « Roblox introuvable » | Vérifie que Roblox est lancé au moins une fois, ou renseigne le chemin de `RobloxPlayerBeta.exe` dans **Paramètres**. |
| Build Linux : erreur `webkit2gtk-4.1` | Installe les paquets de la section Prérequis (le nom varie selon la distro : `webkit2gtk-4.1-dev` Debian/Ubuntu, `webkit2gtk4.1-devel` Fedora). |
| « Adresse déjà utilisée » au lancement du serveur | Le port 8787 est pris : change avec `--port 8790` et mets la même adresse dans l'app. |
| « Pseudo déjà pris » dans le chat | Le pseudo est unique (casse ignorée) : change-le dans Paramètres puis reconnecte-toi. |
| Les flags ne s'appliquent pas | Redémarre Roblox ; vérifie que le fichier `ClientSettings/ClientAppSettings.json` affiché en haut de l'onglet correspond bien à ta version. |
| Rien ne s'affiche dans le chat | Le serveur tourne-t-il ? L'adresse dans Paramètres est-elle la bonne ? Le badge en haut du salon doit être vert. |

---

## 📜 Licence & mentions

- Code sous licence **MIT** (voir `LICENSE`).
- « Roblox » est une marque de Roblox Corporation — ce projet n'y est pas affilié.
- Les usages de FastFlags documentés ici proviennent de la communauté (Bloxstrap & co) et ne sont pas officiels.
