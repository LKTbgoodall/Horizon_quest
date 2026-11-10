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
horizon-launcher native-build C:\path\horizon_quest\mixins\horizon_quest --trust-native-code
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
python3 native/tools/native.py check   /path/horizon_quest
python3 native/tools/native.py check   /path/horizon_quest --horizon latest
python3 native/tools/native.py build   /path/horizon_quest
python3 native/tools/native.py package /path/horizon_quest   # → native/target/horizon_quest-<v>.hnmod
python3 native/tools/native.py proof   /path/horizon_quest --resources <game files>
```
- **Python ≥ 3.11 and rsync** are required. On Windows: **WSL** (« *on Windows, use WSL for the checks below* »).
- `native.py` downloads the Horizon archive at the pinned commit, installs `horizon-mixins-cli`, applies the bundle, then runs `cargo check|build --locked` with `RUSTFLAGS=-D warnings`.
- `NATIVE_NO_DEFAULT_FEATURES=1` gives a server build.

## Unit tests (without the game)
```bash
cargo test   # in the mod folder; Cargo.toml [lib] → pure rules
```
This is the smp pattern: **all the quest logic in pure modules** (dependencies, progress, rewards, command parsing, menu layout), tested without the game. See [[Architecture]].

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

## Prerequisites to install
- [ ] Horizon launcher (https://playhorizon.dev)
- [ ] A copy of Minecraft Java (Microsoft account) for the game files
- [ ] Git
- [ ] Stable Rust via https://rustup.rs (for local `cargo test`)
- [ ] WSL + Python 3.11 + rsync (for `native.py`)
- [ ] (optional) Obsidian for this vault
