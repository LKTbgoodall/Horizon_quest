---
tags: [horizon, build, outils]
source: playhorizon.dev/docs/make-a-mod, /docs/test-a-mod, horizon/docs/native-mods.md, horizon-mods/native/tools/native.py
verifie_le: 2026-11-10
---
# Build, test et outils

Retour : [[Horizon - Vue d'ensemble]]

> [!danger] Règle importante
> « *It is not an MCP tool: trusting native code is the player's decision, never an agent's.* »
> → Les commandes contenant `--trust-native-code` doivent être **lancées par toi (Bastien)**, pas par Claude. Claude peut préparer le code et les commandes, mais c'est toi qui les exécutes.

## Deux chaînes d'outils
### A. Le launcher (natif Windows) : pour jouer et tester
```powershell
horizon-launcher native-build C:\chemin\horizon_quest\mixins\horizon_quest --trust-native-code
horizon-launcher native-build <dossier> --instance NOM --trust-native-code
horizon-launcher settings native-profile dev          # rebuild ~6x plus rapide, jeu ~2x plus lent
horizon-launcher settings native-profile performance  # pour revenir
```
- Le launcher télécharge **son propre Rust 1.99.1** (`x86_64-pc-windows-gnu`, WinLibs GCC 16.2 / MinGW-w64, ≈1,7 Go). Notre Rust à nous n'est pas utilisé.
- Durées : premier build ≈7-8 min sous Windows (la doc web annonce 20-30 min), `native/target` ≈2 Go. Ensuite, une modif de helper prend ≈45 s et une modif de la crate du mod ≈11 s.
- Log de build : `native/build.log` dans le dossier de données du launcher (bouton « Build log »).
- Cache plafonné à 30 GiB : `native-cache-gib`, `native-cache --clear`.

### B. `native.py` (registre `horizon-mods`) : check, package, proof
```bash
git clone https://github.com/horizon-rust/horizon-mods
cd horizon-mods
python3 native/tools/native.py check   /chemin/horizon_quest
python3 native/tools/native.py check   /chemin/horizon_quest --horizon latest
python3 native/tools/native.py build   /chemin/horizon_quest
python3 native/tools/native.py package /chemin/horizon_quest   # → native/target/horizon_quest-<v>.hnmod
python3 native/tools/native.py proof   /chemin/horizon_quest --resources <fichiers de jeu>
```
- Il faut **Python ≥ 3.11 et rsync**. Sous Windows : **WSL** (« *on Windows, use WSL for the checks below* »).
- `native.py` télécharge l'archive d'Horizon au commit épinglé, installe `horizon-mixins-cli`, applique le bundle, puis lance `cargo check|build --locked` avec `RUSTFLAGS=-D warnings`.
- `NATIVE_NO_DEFAULT_FEATURES=1` donne un build serveur.

## Tests unitaires (sans le jeu)
```bash
cargo test   # dans le dossier du mod ; Cargo.toml [lib] → règles pures
```
C'est le pattern de smp : **toute la logique des quêtes dans des modules purs** (dépendances, progression, récompenses, parsing de commandes, layout des menus), testés sans le jeu. Voir [[Architecture]].

## Serveur dédié
```powershell
horizon-launcher native-build <dossier> --server-output <DIR> --version <TAG> --trust-native-code [--profile ...]
horizon-server --dir <SERVER> config set engine.path=<DIR>\horizon-engine.exe
horizon-server --dir <SERVER> run
```
- L'export est compilé en `--no-default-features` (sans audio). Il tourne uniquement sur le même OS et la même architecture.
- `GET /api/v1/mods` liste les mods natifs inclus dans l'engine.

## Tests E2E
- `proof.py` (comme dans smp) lance `horizon-client --dedicated --nogui` et des clients `horizon-ctl --offscreen` qui lisent le chat, les menus (slots numérotés depuis 300) et l'inventaire.
- Variables utiles : `HORIZON_HOME`, `HORIZON_LAUNCHER_HOME`, `HORIZON_NO_AUDIO=1`, `HORIZON_LAUNCHER_NO_SELF_UPDATE=1`.

## Pièges Windows
- **Smart App Control** bloque les programmes fraîchement compilés (os error 4551). Il faut le désactiver dans Sécurité Windows.
- **Chemin de profil non-ASCII** : le build est déplacé vers `ProgramData\horizon-launcher-<id>`.
- `RUSTFLAGS`, `CARGO_*` et `~/.cargo/config.toml` sont **ignorés** volontairement.
- Les bibliothèques statiques MSVC précompilées sont incompatibles (on est en toolchain GNU).
- Si deux mods injectent au même endroit, on obtient des warnings.

## Prérequis à installer
- [ ] Launcher Horizon (https://playhorizon.dev)
- [ ] Une copie de Minecraft Java (compte Microsoft) pour les fichiers de jeu
- [ ] Git
- [ ] Rust stable via https://rustup.rs (pour `cargo test` en local)
- [ ] WSL + Python 3.11 + rsync (pour `native.py`)
- [ ] (optionnel) Obsidian pour ce vault
