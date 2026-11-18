---
tags: [adr, platform]
status: accepted
date: 2026-11-16
---
# ADR-018: Port the mod to Horizon's mod API

## Context
On 2026-11-16 Horizon `main` merged the mod API (horizon-rust/horizon#17145, the arismoko fork's API, [[Fork - arismoko mod API]]). Mods are now one flat package (`horizon-mod.toml` with `[native]`, `Cargo.toml`, `src/`, `assets/`, `data/`), compiled against a prebuilt SDK; hooks are methods of `mod_api::Hooks` with a typed `ServerHost` / `ClientHost`. Our format (`horizon-native.json`, `horizon-mixin.toml`, `mixins/horizon_quests/`, `native_*` hook points) is **refused** on `main` → [[Native mods - structure and manifest]] § "The new format".

Not released yet on 2026-11-16: launcher 0.2.69 and Nightly `9f42064` predate the merge and still take our format. The first Nightly built from the new `main` will not, and the registry compiles native mods on the newest Nightly.

The combined-content fix our 1.0.0 waited for (horizon-rust/horizon#16007) was **closed unmerged** as superseded: the new format carries content and code in one `.hmod` by design, so our textures ship only once we are on it.

## What the port costs (read 2026-11-16 at `90e97a1`, hook by hook)
Nearly everything we use exists, mostly under the same name without `native_`:
- ✅ Direct: items, HUD (`render_gui_post`), client ticks, the seven screen hooks over our own `mod_screen_open` screen, `DrawContext` (`fill`, `quad`, `sprite`, `text`, `text_width`, `item_stack`, tooltips, clips), commands registration, configs (same four hooks), `server_tick`, login/logout, `item_crafted`, `furnace_extract`, `player_level_change`, `player_advancement_done`, inventory reads, `give_stack` / XP / levels, `tell`, `run_command`, tags, `world_dir`, `claim_items` / `poll`, clipboard (new).
- 🟠 Different shape: payload channels registered in `Hooks::register` and negotiated at join (`play_bidirectional`), key mapping registered up front, typed command invocation (feed `hq_commands` from literals and arguments), reload listeners as trait objects, `WorldOpenReady` → `server_started`, `ItemStack` / `ItemKey` construction, pickup poke → `player_inventory_changed`.
- ❌ Gone: `native_data!` (the built-in example pack moves to `data/horizon_quests/` and is read with `host.resource` or our reload listener), `native_take_items` / `native_carried_items` (hand-in rewritten on `inventory_slots` + `set_inventory_slot`), `native_recipes` (recipe view rebuilt from `item_keys`, `hover_name`, `tags()`, `recipes_of_type`), free-text client settings (the tracked quest moves to the client's `data_files()`), `native_corner_shown`.
- Likely gain: the single-player host is an ordinary `PlayerRef`, so our host/guest workarounds (taking items in place, `/clear` and level probes) may collapse into one path. To confirm in game.

In code: the pure modules (`hq_*.rs`, ~15,600 lines, the tests) move as they are into `src/`; the two glue files (~1,700 lines) and ~30 hook files are rewritten into one `impl Hooks`; `tools/build-mod.ps1` / `.sh`, `tools/check-glue.*`, `compatibility.json` and `horizon-mixin.toml` are replaced by `cargo horizon-mod` and the launcher's `native-build`.

## Options
1. **Port as soon as a Nightly with the mod API and its SDK is out**, then release 1.0.0 on it. + textures in the one `.hmod`, faster builds, crates.io allowed, a build that can fit later Nightlies (`uses.json`) / − a few days of port, a full in-game retest, and nothing can be compiled before that Nightly exists.
2. **Start the port now against `main`'s source**, compile it when the Nightly ships. + ready sooner / − written blind against an API that is still moving ("final bugs being fixed").
3. **Release 1.0.0 now in the old format** for launchers ≤ 0.2.69 + Nightly `9f42064` (code only, no textures), then port. − dead on the next Nightly; the registry's check would refuse it soon after.

## Recommendation (Claude)
Option 1, with the inventory of option 2 done first on paper (this note). The freeze for 1.0.0 stays; 1.0.0 becomes "the first version on the mod API".

## Decision
✅ **Accepted** (Bastien, 2026-11-16): **option 1**. Port the mod to `mod_api` as soon as the first Nightly built with the mod API and its SDK is out, then release **1.0.0 as the first version on the mod API**. The feature freeze stays: the port changes the plumbing, not what the mod does.

**Amended the same evening** (Bastien: "can't you take care of porting it in the meantime?"): the port is done **now, on the branch `mod-api-port`**, and `main` keeps the old format until the Nightly ships. It is not blind: the mod depends on the API crate by path (`.horizon/sdk/api/crates/horizon-mod-api`), and `tools/link-sdk` makes `.horizon/sdk/api` a junction to a Horizon checkout, so `cargo check` and `cargo test` compile the whole mod, glue included, against Horizon `main` (`c07bfa7`) with `-D warnings`. When the Nightly is out: move the pins (`cargo horizon-mod update`), write the registry lock (`horizon-launcher native-lock mod`), fix what moved, merge the branch into `main`, and Bastien tests everything in game.

## Consequences
Effective when the port lands (the old rules keep governing the current code until then):
- [[ADR-003 Hook points over raw injections]] and [[ADR-004 Mixin bundle style like smp]] are superseded: hooks are `mod_api::Hooks`, no `[[native.*]]` tables (they would tie the mod to one build).
- The CLAUDE.md rule "only crates in Horizon's `Cargo.lock`" becomes "crates.io crates pinned in our `Cargo.lock`, SDK re-exports preferred" (`mod_api::serde_json`).
- [[ADR-014 Publish one hmod with content and code]] is met by the format itself; [[ADR-015 Embed the content pack in the native mod]] is no longer needed.

## Outcome (2026-11-18)
- Nightly 2026-11-18 (`0b14f8a`) is the first with the mod API, published with its signed SDK (prerelease `sdk-0b14f8a…`, `sdk.json` on the Nightly). The branch was re-pinned on it (`[native] horizon = 0b14f8a`, `mixins = e59da90` from `sdk.json`), its `Cargo.lock` made registry-only as `native-lock` writes it, checked against that exact commit (244 tests, `-D warnings`), numbered **0.22.0** and merged into `main` ([[2026-11-18]]). The old format is gone from `main`.
- Still to see: the launcher's own build of it, and the in-game retest. 1.0.0 follows once it passes.

## Links
- [[Native mods - structure and manifest]] · [[Horizon hooks and API]] · [[Fork - arismoko mod API]]
- [[2026-11-16]] · `release/v1-checklist.md`
