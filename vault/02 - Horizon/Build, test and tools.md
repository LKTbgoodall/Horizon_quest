---
tags: [horizon, build, tools]
source: playhorizon.dev/docs/make-a-mod, /docs/test-a-mod, horizon/docs/native-mods.md, horizon-mods/native/tools/native.py
verified_on: 2026-11-10
---
# Build, test and tools

Back: [[Horizon - Overview]]

> [!danger] Important rule
> « *It is not an MCP tool: trusting native code is the player's decision, never an agent's.* »
> → Commands containing `--trust-native-code` must be **run by you (Bastien)**, not by Claude. Claude can prepare the code and the commands, but you are the one who runs them.

## Two toolchains
### A. The launcher (native Windows): for playing and testing
```powershell
horizon-launcher native-build C:\path\horizon_quests\mixins\horizon_quests --trust-native-code
horizon-launcher native-build <folder> --instance NAME --trust-native-code
horizon-launcher settings native-profile dev          # rebuild ~6x faster, game ~2x slower
horizon-launcher settings native-profile performance  # to switch back
```
- The launcher downloads **its own Rust 1.99.1** (`x86_64-pc-windows-gnu`, WinLibs GCC 16.2 / MinGW-w64, ≈1.7 GB). Our own Rust is not used.
- Durations: first build ≈7-8 min on Windows (the web docs say 20-30 min), `native/target` ≈2 GB. After that, a helper change takes ≈45 s and a change to the mod crate ≈11 s.
- Build log: `native/build.log` in the launcher's data folder ("Build log" button).
- Cache capped at 30 GiB: `native-cache-gib`, `native-cache --clear`.

### B. `native.py` (`horizon-mods` registry): check, package, proof
```bash
git clone https://github.com/horizon-rust/horizon-mods
cd horizon-mods
python3 native/tools/native.py check   /path/horizon_quests
python3 native/tools/native.py check   /path/horizon_quests --horizon latest
python3 native/tools/native.py build   /path/horizon_quests
python3 native/tools/native.py package /path/horizon_quests   # → native/target/horizon_quests-<v>.hnmod
python3 native/tools/native.py proof   /path/horizon_quests --resources <game files>
```
- **Python ≥ 3.11 and rsync** are required. On Windows: **WSL** (« *on Windows, use WSL for the checks below* »).
- `native.py` downloads the Horizon archive at the pinned commit, installs `horizon-mixins-cli`, applies the bundle, then runs `cargo check|build --locked` with `RUSTFLAGS=-D warnings`.
- `NATIVE_NO_DEFAULT_FEATURES=1` gives a server build.

## Unit tests (without the game)
```bash
cargo test   # in the mod folder; Cargo.toml [lib] → pure rules
```
This is the smp pattern: **all the quest logic in pure modules** (dependencies, progress, rewards, command parsing, menu layout), tested without the game. See [[Architecture]].

## Type-check the glue without the game (`tools/check-glue.ps1`, 2026-11-10)
`cargo test` never compiles `*_glue.rs` or the hook helpers. This does, on a PC whose launcher has built Horizon once:
```powershell
powershell -ExecutionPolicy Bypass -File tools\check-glue.ps1
```
- It runs `horizon-mixins.exe apply --workspace <launcher's source copy of the pin> --mods mod\mixins --out build\check\stage` (source transform only; the output folder must not exist), then `cargo check -p horizon-client --bin horizon-client --locked` in the stage with its own target folder (`build\check\target`). Nothing is built to run, the mod's code is never executed, and no `--trust-native-code` is involved, so Claude may run it.
- The launcher's pieces it uses: source `%APPDATA%\horizon-launcher\native\source\<horizon rev>\`, tool `native\tools\<mixins rev, 12 chars>\bin\horizon-mixins.exe`. The launcher keeps its own Rust (`horizon-launcher\rust`) and cargo home (`horizon-launcher\cargo`); the script uses the system's rustup toolchain (same 1.99.1) and cargo home, so the first run downloads Horizon's crates.
- First run about 3 minutes, then about 1 minute. The mod's modules are inlined in `crates\horizon-client\src\game.rs`, so that is the file errors and warnings name.
- It replaces the hand-written stub crate of Sessions 9 to 11, **which had missed a real build break** (`native_as_player` private): hand-copied signatures prove nothing about visibility.
- Horizon's own crates give two warnings there (`horizon-friends`, `horizon-protocol`: unused `mut`), so the check cannot run with `-D warnings`; read the warnings that name `game.rs`. v0.0.6 has none.
- A `pub fn` of a glue module that takes `Game` warns (`private_interfaces`: `Game` is more private than the function). Glue functions are `pub(super)` since v0.0.6.

## Dedicated server
```powershell
horizon-launcher native-build <folder> --server-output <DIR> --version <TAG> --trust-native-code [--profile ...]
horizon-server --dir <SERVER> config set engine.path=<DIR>\horizon-engine.exe
horizon-server --dir <SERVER> run
```
- The export is compiled with `--no-default-features` (no audio). It only runs on the same OS and the same architecture.
- `GET /api/v1/mods` lists the native mods included in the engine.

## E2E tests
- `proof.py` (as in smp) launches `horizon-client --dedicated --nogui` and `horizon-ctl --offscreen` clients that read the chat, the menus (slots numbered from 300) and the inventory.
- Useful variables: `HORIZON_HOME`, `HORIZON_LAUNCHER_HOME`, `HORIZON_NO_AUDIO=1`, `HORIZON_LAUNCHER_NO_SELF_UPDATE=1`.

## Windows pitfalls
- **Smart App Control** blocks freshly compiled programs (os error 4551). It must be turned off in Windows Security.
- **Non-ASCII profile path**: the build is moved to `ProgramData\horizon-launcher-<id>`.
- `RUSTFLAGS`, `CARGO_*` and `~/.cargo/config.toml` are deliberately **ignored**.
- Precompiled MSVC static libraries are incompatible (we are on the GNU toolchain).
- If two mods inject at the same place, you get warnings.

## Two-PC workflow
| | Dev PC (work, this repo) | Home PC (Windows) |
|---|---|---|
| Role | write code, `cargo test`, commit and push | `git pull`, build the mod with the launcher, test in game |
| Game | ❌ none, no in-game test possible | ✅ Horizon launcher + Minecraft Java |
| Rust | ✅ rustup, stable 1.99.1 `x86_64-pc-windows-gnu` (installed 2026-11-10) | the launcher brings its own Rust 1.99.1 |

Loop: Claude commits and pushes each step → Bastien runs `git pull` on the home PC → builds with `horizon-launcher native-build … --trust-native-code` (Bastien runs it, never Claude) → Bastien reports the result (screenshots, `native/build.log`) → the next session fixes it.

## Prerequisites to install
Dev PC:
- [x] Git
- [x] Stable Rust (GNU toolchain) via rustup, for `cargo test` (2026-11-10)
- [ ] (optional) WSL + Python 3.11 + rsync, for `native.py check` without the game
- [ ] (optional) Obsidian for this vault

Home PC:
- [x] Horizon launcher and Minecraft Java (according to Bastien, 2026-11-10)
- [ ] Git, and a clone of https://github.com/LKTbgoodall/Horizon_quest
- [ ] Smart App Control off if builds get blocked (os error 4551)

## Installing our mod in an instance (learned 2026-11-10)
- `horizon-launcher native-build <mod folder> --trust-native-code` builds the instance's installed mods **plus** that folder, but does **not** install it: the launcher's Mods tab does not list it, shows "To recompile", and the next build from the launcher (its "Compile" button) **drops the mod**.
- To install it for real: a `.hnmod` (zip, `/` separators) with `horizon-native.json` (`game`, `hooks`, `horizon`, `id`, `mixins`, `schema` 1, `version`) and the bundle under `mixins/<id>/`, copied into `%APPDATA%\horizon\instances\<instance>\horizon-native-mods\`, plus the line `mod.<id>:true` in `horizon-mods.txt` of the instance. Then `native-build --instance <instance>` with no folder.
- `tools\build-mod.ps1` does all of that (modelled on Bastien's script in his `organa` project). `build/` is gitignored.
- `tools/build-mod.sh` is its bash twin for Linux and macOS (added 2026-11-10 for a LAN guest on Arch Linux): same package (files only, no folder entries), instance found under `~/.config/horizon/instances/NAME` or `~/.local/share/horizon/instances/NAME` (guessed from the Windows path `%APPDATA%\horizon\instances`, **not verified on Linux**; `--instance-dir` overrides), launcher `horizon-launcher` on the PATH or `$HORIZON_LAUNCHER`. Packaging tested on Windows (Git Bash, python fallback); install and build untested.
- Paths on Bastien's PC: launcher `%LOCALAPPDATA%\Programs\Horizon Launcher\horizon-launcher.exe` (not on PATH; it is a windowed app, so PowerShell's `&` does not wait for it, `Start-Process -Wait` does), data `%APPDATA%\horizon-launcher` (`native/build.log`, `native/built.json`, `versions/`), instances `%APPDATA%\horizon\instances\` (default `mon-horizon`).
- First real build: 2026-11-10, Horizon `fdc87ae`, mixins `5a64af9`, profile performance, 4 min 12 s incremental, **no compile error** in our glue. The mod now pins `fdc87ae`.

## One .hmod (since v0.1.2)
- `build-mod.ps1` / `build-mod.sh` pack `build/horizon_quests.hmod` (content from `mod/content` + native bundle), the one file of the mod, reproducible (fixed dates, sorted). Install cuts it in two (`build/install/`): `.hnmod` to `horizon-native-mods/` (the only place the pinned launcher builds from), content `.hmod` to `horizon-mods/`. `-Uninstall` removes both.
- No `include_str!` / `#[path]` / external `mod` in a bundle module, even under `#[cfg(test)]`: `horizon-mixins apply` fails with `E_MODULE`. Tests that read repo files go in `mod/tests/`.
