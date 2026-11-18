---
tags: [inbox, issue-draft, horizon]
date: 2026-11-18
status: draft (Bastien posts it)
---
# Issue draft: native build hangs after "Checking the pinned registry index"

To post on https://github.com/horizon-rust/horizon/issues (title and body below). Bastien runs `horizon-launcher bug-report` first and attaches the file it writes (nothing is sent by that command).

---

**Title:** bug(native mods): a mod lock that moves a game crate hangs the build forever instead of failing (Windows)

**Body:**

**What happens**
Building a flat native mod on Nightly 2026-11-18 (`0b14f8a`) never finishes. The build stops right after `cargo update --workspace` succeeds, and the next step, "Downloading the mods' pinned crates" (`cargo fetch --locked`), never starts. No error, no exit: the launcher waits forever. It happens from the CLI (`native-build <folder> --instance ... --update --trust-native-code`) and from the Mods page's Build button, every time (4 tries).

**Versions**
- Launcher 0.2.78, Windows 11 (x86_64, Windows GNU toolchain 1.99.1 from the launcher)
- Nightly 2026-11-18 · `0b14f8a` (prepared; its SDK `sdk-0b14f8a…` downloaded, 287 MB)
- A fresh instance on that Nightly, with only this mod installed

**End of `native/build.log`**
```
Planning the build: 50 ms
Checking the prebuilt game: 10 ms
Checking the Rust toolchain: 0 ms
Native mods route: prebuilt.
$ "...\rust\1.99.1-x86_64-pc-windows-gnu\bin\cargo.exe" "update" "--manifest-path" "...\native\prebuilt-build\0b14f8acfd97-2f43e84777a82131\stage-2e3c9281475b6c91\Cargo.toml" "--workspace"
warning: patch `libmimalloc-sys v0.1.49 (...\prebuilt-package\patches\libmimalloc-sys)` was not used in the crate graph
warning: patch `naga v30.0.1 (...\prebuilt-package\patches\naga)` was not used in the crate graph
warning: patch `wgpu-hal v30.0.1 (...\prebuilt-package\patches\wgpu-hal)` was not used in the crate graph
     Locking 2 packages to highest compatible versions
      Adding horizon_patch v0.0.0 (...\stage-2e3c9281475b6c91)
      Adding mod-horizon-quests v0.22.0 (...\stage-2e3c9281475b6c91\native-api\horizon_quests)
note: pass `--verbose` to see 1 unchanged dependencies behind latest
Checking the pinned registry index: 81 ms
```
Nothing after that line, ever (left 10+ minutes).

**What the process does meanwhile**
- No `cargo.exe` or `rustc.exe` running, no child process at all.
- No disk reads or writes, no network connection; nothing new in `native/` or `prebuilt-build/`.
- One thread wakes about every 100 ms (low CPU), the rest wait.

Reading `launcher/src/native_mods/registry_plan.rs` (`resolve_and_fetch`) at `0b14f8a`: after the `cargo update` run returns (its "81 ms" line is written), the code does `read_lock`, `RegistryLock::parse`, `check_mods`, `check_partial_resolution`, then logs `$ cargo fetch ...` before spawning it. That `$` line never appears, so the build stops somewhere between those calls. A panic in the build thread with its message lost (the launcher is a windowed app) would look exactly like this, but I could not confirm it.

**Cause on our side (found later)**
The mod's `Cargo.lock` pinned newer patch versions than the prebuilt game's lock for crates the game also uses (`syn` 3.0.10 vs 3.0.6, `unicode-ident` 1.0.27 vs 1.0.26, `equivalent`, `toml_parser`...). Replaying `RegistryLock::check_partial_resolution` on the staged lock shows it fails for `indexmap`, `proc-macro2`, `serde_derive` and `toml` ("moved or changed its dependency graph"). With the lock re-resolved on the game's versions, it passes. So the build was right to refuse the mod, but **it hangs instead of reporting the error**, from both the CLI and the Mods page. A minimal mod without crates.io dependencies builds in 3 s on the same setup.

Also: `horizon-launcher native-lock` writes the newest versions, not the SDK game's, so a lock it makes can hit the same refusal.

**To reproduce**
The mod is public: https://github.com/LKTbgoodall/Horizon_quest, folder `mod/` (commit `02f8b5c`, before the lock fix), pinned to `horizon = 0b14f8a…`, `mixins = e59da90…`. Its `Cargo.lock` holds only crates.io pins (log, serde, serde_json, toml and their dependencies).

**Possibly related**
On the same machine, before the Nightly was prepared, `native-build` said once "nightly-2026-11-18-0b14f8a is not prepared yet", then later runs hung the same way with no output, until `horizon-launcher prepare` was run.
