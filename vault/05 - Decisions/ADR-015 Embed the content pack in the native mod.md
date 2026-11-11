---
tags: [adr, publication]
status: proposed
date: 2026-11-11
---
# ADR-015: Embed the content pack in the native mod

> 🟡 **On hold** (Bastien, 2026-11-11): first wait for Friday's Nightly (native content support, [[ADR-014 Publish one hmod with content and code]]). This is the fallback if it does not come. Nothing is coded.

## Context
- [[ADR-014 Publish one hmod with content and code]] chose one `.hmod` with code and content, but today's launchers and our pinned Horizon (`fdc87ae`) take a native mod's **code only**: published that way, players get no textures (no node frames, no line chevrons, a plain Quest Book). 1.0.0 was waiting for a Nightly that reads native content, with none in sight.
- Bastien found that **Applied Energistics 2** (`ae2.hnmod`, on the registry) ships its whole content in one `.hnmod`: the content mod is a data file of the bundle (`native_data!("ae2", "ae2-content.hmod")`); a cancellable HEAD injection on `horizon_client::game::mods::content_root` writes it once to `.horizon-cache/` and calls `content_root` again with that file added to the dev mods, so Horizon loads it as a content mod installed by hand. An installed copy of the content mod wins (it is read first; a second mod of the same id is skipped).
- `content_root(root, enabled, dev) -> PathBuf` exists with the same signature in our pin (`crates/horizon-client/src/game/mods.rs`).
- [[ADR-003 Hook points over raw injections]] allows a raw injection only as a last resort, with its own ADR. No hook point loads a native mod's content.

## Decision (if adopted)
Do as AE2: an **exception to ADR-003** for this one injection.
- `tools/make-content-pack.py` packs `mod/content` into `mod/mixins/horizon_quests/data/content.hmod` (stored, not compressed, fixed dates: the same sources give the same file). The file is committed, since `native_data!` reads it when the game is built; a test checks it holds exactly the files of `mod/content`.
- The injection `embedded-content` (helper `embedded_content.rs`) on `horizon_client::game::mods::content_root`, HEAD, cancellable, as AE2's: writes `.horizon-cache/horizon_quests-embedded-content.hmod` when missing or different, adds it to `dev`, and returns `content_root` of that list.
- The build scripts install the `.hnmod` only (no content half in `horizon-mods/`, which would shadow the embedded copy). The registry gets **`build/install/horizon_quests.hnmod`** (Kind "Native mod (.hnmod)").

## Consequences
- + Players get every texture with today's launchers; 1.0.0 no longer waits for a Nightly.
- + The method is already accepted on the registry (AE2).
- − A raw target can change in any Horizon commit: the mod would stop compiling on that Nightly. `tools/check-glue.ps1` against a new pin shows it at once; the registry compiles on the newest Nightly.
- − The content is in the code bundle twice over (the `.hmod` in `data/` and the files in `mod/content`): `make-content-pack.py` must be run after changing `mod/content` (the test fails otherwise).
- When the game reads native content (the Nightly ADR-014 waited for), drop the injection and go back to ADR-014's one `.hmod`.

[[Decision index]]
