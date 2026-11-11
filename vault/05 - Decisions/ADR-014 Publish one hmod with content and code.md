---
tags: [adr, publication]
status: accepted
date: 2026-11-11
---
# ADR-014: Publish one .hmod with content and code

## Context
The mod has a native half (the code, `mixins/horizon_quests/`) and a content half (`mod/content`: the Quest Book's model and texture, the node frames `textures/gui/nodes.png`, `lang/en_us.json`). `tools/build-mod.ps1` / `.sh` already pack both into `build/horizon_quests.hmod`, and for the launcher of our pin (`fdc87ae`) cut it into `horizon-native-mods/horizon_quests.hnmod` + `horizon-mods/horizon_quests.hmod`.

At our pin, `docs/native-mods.md` said the registry "keeps publishing native mods as `.hnmod` until launchers and the game read native `.hmod`s": an `.hnmod` holds code only, so the content would not reach players.

Read on 2026-11-11 in Horizon `main` (`docs/api/mod-files.md`, `docs/native-mods.md`, horizon-mods' README):
- "Native registry entries may name `.hnmod` (code only) or `.hmod` (optionally with content). A combined file needs a launcher and game with native `.hmod` support; keep publishing code-only `.hnmod`s for older launchers."
- "The launcher can install native `.hmod` files from the registry, alongside legacy `.hnmod`s. A build validates the content and snapshots it under its own `native-content/<id>/`, then the game layers it through the normal content loader."
- A combined `.hmod` holds only `horizon-mod.toml`, `assets/`, `data/`, `horizon-native.json` and `mixins/<id>/` at its top level, the two manifests naming the same id; any other top-level entry refuses it.
- The registry's checks compile a native mod **on the newest Nightly** with the most downloaded native mods (`native-compat.yml`), and mark a version `broken_on` a Horizon commit where horizon-mixins refuses it.

## Options considered
1. **One combined `.hmod`** (ours as it is): + players get the textures; one file; the launcher snapshots the content per build / − needs a recent launcher (after our pin).
2. **A code-only `.hnmod`**: + any native launcher / − no textures: flat nodes, a vanilla-looking book (the mod falls back, but plainer).
3. **An `.hnmod` and a separate content entry**: − two registry entries cannot share the id `horizon_quests`, and the docs give no way to tie them.

## Decision
Option 1: the registry file is **`build/horizon_quests.hmod`**, content and code. It already follows the rules (checked: 59 entries, top level `assets/`, `horizon-mod.toml`, `horizon-native.json`, `mixins/`; same id in both manifests).

Before sending it:
- **Re-pin on a recent Nightly**: native `.hmod` support and the registry's compile check are on Horizon `main`, newer than `fdc87ae`. Bastien's launcher updates to the newest Nightly (it fetches that revision's source); then the glue is checked against it (`tools/check-glue.ps1`), and `horizon_revision` / the horizon-mixins revision / `hooks` are updated in `horizon-mixin.toml`, `compatibility.json` and the build scripts.
- **Run the registry's own check** on the file: `cargo horizon-mod check build/horizon_quests.hmod` (`cargo install --git https://github.com/horizon-rust/horizon cargo-horizon-mod`).
- If the submit form or the check still refuses a combined file for a `native-mod`, fall back to option 2 for that version and ask the maintainers.

## Update 2026-11-11
`horizon-launcher versions` after Bastien's launcher update: the newest Nightly is `nightly-2026-11-08-fdc87ae`, our pin. So no re-pin is needed for now, and the registry compiles on the same Horizon as we test. But the native `.hmod` support read on `main` is not in a Nightly yet: until one ships it, a combined `.hmod` cannot be installed by players' launchers. Choice left to Bastien: wait for that Nightly to publish 1.0.0 with its textures, or publish 1.0.0 as the code-only `.hnmod` now (the mod works, plainer) and the combined `.hmod` as the next version. **Bastien chose to wait** for the Nightly that ships native `.hmod` support: 1.0.0 goes out with its textures.

Tried the same day (`build-mod.ps1 -Combined`, launcher 0.2.58, Nightly `fdc87ae`): the launcher **builds the code from the combined `.hmod`** but copies no content (no `native-content/`). So the combined file degrades to code only on today's launchers, exactly like an `.hnmod`: publishing it now loses nothing, and the textures arrive by themselves once players' launcher and game support native content. Whether the submit form takes a `.hmod` for a native mod is the remaining unknown.

## Consequences
- Players need a launcher recent enough to read native `.hmod` files; the registry entry says so.
- The local build scripts keep cutting the file in two for launchers of our pin; once the pin moves to a launcher that reads native `.hmod`s, installing becomes copying the one file (to check then).
- Content stays client-side assets only (`side = "client"`): nothing changes for servers.

## Links
- [[Publishing and mod policy]]
- [[Decision index]]
- `release/registry-entry.md`, `release/v1-checklist.md`
