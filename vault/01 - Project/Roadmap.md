---
tags: [project, roadmap]
updated: 2026-11-10
---
# Roadmap

> Check items off as you go. The **current phase** is shown in [[00 - Home]].
> Feature priorities: [[FTB task types]], [[FTB reward types]].
> Targets: **solo/LAN first**, graphical book from the MVP ([[ADR-008 Graphical client book first]]), English-only in-game text.

## Phase 0: setup ✅ (LAN test still optional)
- [x] Horizon and FTB Quests research (2026-11-10)
- [x] Project vault (2026-11-10)
- [x] `git init`, `.gitignore`, first commit and push to https://github.com/LKTbgoodall/Horizon_quest (2026-11-10, sole author: LKTbgoodall)
- [x] Validate ADRs 002 to 006 (2026-11-10)
- [x] License (MIT) and mod id (`horizon_quests`) (2026-11-10, [[ADR-009 MIT license and mod id horizon_quests]])
- [x] Rust on the dev PC: rustup, stable 1.99.1 `x86_64-pc-windows-gnu` (same as Horizon's launcher), for `cargo test` (2026-11-10)
- [x] References cloned into `ext/` (gitignored): `horizon` sparse at `1453e0e` (2026-11-10) + `horizon-mods` (2026-11-10)
- [x] Real signatures checked in `native_hooks.rs` (`NATIVE_HOOKS_API` 415) → [[Horizon hooks and API]] § 0 (2026-11-10)
- [x] Mod skeleton in `mod/` (v0.0.1, hooks 313, MIT), 16 unit tests pass with `-D warnings` (2026-11-10)
- [x] **Spike 1, hello world**: HUD hint shown in solo (2026-11-10, dev PC). Options > Mods row "Horizon Quests 0.0.1: ON" + Settings button seen
- [x] **Spike 2, test book**: K opens the mod screen; 6 nodes, item icons, lines; pan and zoom work (footer showed zoom 1.77, pan 5,4) (2026-11-10). Click-select works: gold frame + name top right
- [x] **Spike 3, mod messages in solo**: `NET OK: pong #14 received`, one pong per opening, 14 in a row with no loss (2026-11-10). **ADR-008 holds**
- [x] **Spike 4, server command**: `/quests` answers `Hello __LKT__! Book pings answered so far: 14.` (2026-11-10)

## Phase 1: pure logic core (without the game, `cargo test`)
> Started on 2026-11-10 while the in-game test of phase 0 is still pending: this phase needs no game.
> The phase 1 modules are in `src/lib.rs` only, **not in `horizon-mixin.toml` yet** (dead code would break the game build under `-D warnings`). Each joins the bundle with the glue that uses it.

- [x] `hq_model`: types + serde for `quests.json`, chapters and progress (2026-11-10)
- [x] `hq_validate`: loading of `quests.json` and `chapters/*.json` + validation: duplicate ids, unknown dependencies, cycles, bad items; a bad chapter is left out alone (2026-11-10)
- [x] `hq_logic`: `Book` index, dependencies (4 modes + `min_required`), visibility, quest states, linear/flexible (2026-11-10)
- [x] `hq_progress`: `QuestEvent` → progress → `Vec<Effect>`; item, hand-in, kill and checkmark tasks; completion in dependency order (2026-11-10)
- [x] Rewards, pure part (in `hq_progress`, no separate `hq_rewards` needed yet): item, xp, xp_levels, claim, claim all, auto-claim, once per player (2026-11-10). Dropping leftovers at the player's feet is glue (phase 2)
- [x] `hq_store`: atomic JSON, progress files per team, version check (2026-11-10). Still to do: background writer thread, `teams.json`, `audit.log`
- [x] `hq_net`: `ToServer` / `ToClient` messages (JSON), protocol version, frames under 30,000 bytes, `Sender` paced at 16 frames a tick, `Receiver` that trusts nothing (2026-11-10)
- [x] `hq_book`: the whole book as a draw list + click zones: regions (header, chapter list, canvas, quest panel, footer), nodes with shapes / sizes / state colours, dependency lines, fit-to-canvas, text wrapping, buttons (Check, Submit, Claim, Claim all), tooltip, click / drag / wheel handling (2026-11-10)
- [x] `hq_commands`: `/quests` forms, op levels, help, validation report, chat lines for effects; `force_complete` and `reset_quest` in `hq_progress` (2026-11-10)
- [x] Example chapter "Getting started" in `data/quests/` (11 quests), with an end-to-end test that finishes it (2026-11-10)
- [ ] Config option to turn the example chapter off: `example_chapter` in `config/horizon_quests-server.toml` (`hq_config`, API 397): **coded (v0.0.5), to test**

> **Phase 1 is done as far as it can go without the game** (98 tests). Everything left needs the glue, and the glue needs the first in-game test (phase 0).

## Phase 2: playable MVP (solo/LAN) 🎯 ⬅️ IN PROGRESS
> Steps 1, 2 & 3 coded on 2026-11-10 as v0.0.3: server glue + net sync (`hq_net`) + graphical book (`hq_book` via `hq_client_glue`). v0.0.4 (same day) fixes the review of v0.0.3 (build, claims in the book, book hand-ins for LAN guests, sync and loading robustness). v0.0.5 adds the server config file (`example_chapter`): everything of phase 2 that needs no game is now coded. "Coded" items wait for the in-game test before they are ticked.

- [ ] Server glue: global state, `server_tick`, `world_lifecycle`, `player_logged_in/out`: **coded (v0.0.2/v0.0.3), to test**
- [ ] Tasks: item (polling + pickup/craft/furnace triggers, [[ADR-010 Item detection by polling plus triggers]]), checkmark, kill (`mob_killed`, melee only), hand-in (`/quests submit` or book button): **coded (v0.0.2/v0.0.3), to test**
- [ ] Rewards: item (`give_stack`, the rest through `/give`, which drops it), xp, levels: **coded (v0.0.2/v0.0.3), to test**
- [ ] Saving in `<world>/horizon-quests/progress/<uuid>.json` (on every change, on leave, on close): **coded (v0.0.2/v0.0.3), to test**. A pack in `<world>/horizon-quests/quests/` replaces the built-in example
- [ ] Net sync: handshake (`Hello`/`Welcome`), definitions (`Book`), progress deltas (`Progress`), claim/submit/check requests re-validated by the server (`hq_net`): **coded (v0.0.3), to test**
- [ ] **Graphical book**: chapter sidebar, pan/zoom canvas with shapes, icons and dependency lines, quest detail panel, claim and submit buttons (`hq_book`): **coded (v0.0.3), to test**
- [ ] Opening: key (`native_mod_key`, default K) and HUD hint hiding while open: **coded (v0.0.3), to test**
- [ ] Notifications: chat message on completion: **coded (v0.0.2/v0.0.3), to test**
- [ ] Commands: `/quests` help, progress, claim, check, submit, reload, validate, complete, reset, reset_all → [[Commands and permissions]]: **coded (v0.0.2/v0.0.3), to test**
- [ ] Test in solo and LAN on the home PC: checklist in `mod/README.md` § Test in game (solo steps 3, LAN step 4)
- [ ] **Release 0.1.0** (private testing)

## Phase 3: V1
> Started on 2026-11-10 while phase 2 still waits for its in-game test. v0.0.6 (the three items marked below) is **half done on branch `wip/v0.0.6`**: pure modules written, glue and tests left → [[2026-11-10]] § Session 12.

- [ ] Tasks: craft, xp, dimension, location, advancement: **engine coded on `wip/v0.0.6`, glue to do**
- [ ] Rewards: command, choice, random, toast, reward tables: **engine coded on `wip/v0.0.6`, glue to do**
- [ ] Teams (parties): create, invite, join, leave, with FTB's merge rules
- [ ] Repeatable and optional quests, sequential tasks: **engine coded on `wip/v0.0.6`, glue to do**
- [ ] TOML config (`native_register_configs`)
- [ ] Datapack quests (`NativeReloadListener`)
- [ ] Book polish: search, tracked quest in the HUD, `show_recipes`, client toasts
- [ ] Assets (combined `.hmod`): shape textures, icon
- [ ] **First public release on playhorizon.dev** → [[Publishing and mod policy]]

## Phase 4: V2
- [ ] Dedicated server support (engine export) and its tests
- [ ] Translations (`lang/`)
- [ ] Stat, biome tasks (depending on available hooks)
- [ ] Optional chest-menu fallback for vanilla clients (only if asked for)

## Phase 5: V3, editor and import
- [ ] In-game editor (edit mode)
- [ ] FTB Quests import (JSON5/SNBT)
- [ ] API for other mods (custom tasks and rewards)
- [ ] Structure task (if a hook appears)
