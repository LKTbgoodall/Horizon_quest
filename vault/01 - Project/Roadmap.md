---
tags: [project, roadmap]
updated: 2026-11-10
---
# Roadmap

> Check items off as you go. The **current phase** is shown in [[00 - Home]].
> Feature priorities: [[FTB task types]], [[FTB reward types]].
> Targets: **solo/LAN first**, graphical book from the MVP ([[ADR-008 Graphical client book first]]), English-only in-game text.

## Phase 0: setup ⬅️ IN PROGRESS
- [x] Horizon and FTB Quests research (2026-11-10)
- [x] Project vault (2026-11-10)
- [x] `git init`, `.gitignore`, first commit and push to https://github.com/LKTbgoodall/Horizon_quest (2026-11-10, sole author: LKTbgoodall)
- [x] Validate ADRs 002 to 006 (2026-11-10)
- [x] License (MIT) and mod id (`horizon_quests`) (2026-11-10, [[ADR-009 MIT license and mod id horizon_quests]])
- [x] Rust on the dev PC: rustup, stable 1.99.1 `x86_64-pc-windows-gnu` (same as Horizon's launcher), for `cargo test` (2026-11-10)
- [x] References cloned into `ext/` (gitignored): `horizon` sparse at `1453e0e` (2026-11-10) + `horizon-mods` (2026-11-10)
- [x] Real signatures checked in `native_hooks.rs` (`NATIVE_HOOKS_API` 415) → [[Horizon hooks and API]] § 0 (2026-11-10)
- [x] Mod skeleton in `mod/` (v0.0.1, hooks 313, MIT), 16 unit tests pass with `-D warnings` (2026-11-10)
- [ ] **Spike 1, hello world**: Options > Mods row + HUD hint. Code written; **to test on the home PC** (see `mod/README.md`)
- [ ] **Spike 2, test book**: K opens a mod screen with 6 demo nodes, icons, lines; drag pans, wheel zooms, click selects. Code written; **to test**
- [ ] **Spike 3, mod messages in solo**: opening the book pings the server half, which answers pong (footer + chat). Code written; **to test**. The result decides if ADR-008 holds as is
- [ ] **Spike 4, server command**: `/quests` answers in chat. Code written; **to test**

## Phase 1: pure logic core (without the game, `cargo test`)
- [ ] `hq_model`: types + serde + loading of `quests.json` and `chapters/*.json`
- [ ] Validation: duplicate ids, unknown dependencies, cycles
- [ ] `hq_logic`: dependencies (4 modes + `min_required`), visibility, startable, linear/flexible
- [ ] `hq_progress`: `QuestEvent` → progress → `Vec<Effect>`
- [ ] `hq_rewards`: item, xp, xp_levels, claim, auto-claim, drop leftovers at the player's feet
- [ ] `hq_store`: atomic JSON, migrations
- [ ] `hq_net`: protocol messages, versioned, chunked under 30,000 bytes
- [ ] Canvas math (pure): pan/zoom transform, hit testing, dependency line geometry, layout of the detail panel
- [ ] Example chapter "Getting started" in `data/`, with a config option to turn it off

## Phase 2: playable MVP (solo/LAN) 🎯
- [ ] Server glue: global state, `server_tick`, `world_lifecycle`, `player_logged_in/out`
- [ ] Tasks: item (polling), checkmark, kill (`mob_killed`)
- [ ] Rewards: item (`give_stack` + drop the rest), xp, levels
- [ ] Saving in `<world>/horizon-quests/`
- [ ] Net sync: handshake, definitions, progress deltas, claim/submit/check requests re-validated by the server
- [ ] **Graphical book**: chapter sidebar, pan/zoom canvas with shapes, icons and dependency lines, quest detail panel, claim and submit buttons
- [ ] Opening: key (`native_mod_key`) and client command
- [ ] Notifications: chat message and sound on completion
- [ ] Admin commands → [[Commands and permissions]]
- [ ] Test in solo and LAN on the home PC
- [ ] **Release 0.1.0** (private testing)

## Phase 3: V1
- [ ] Tasks: craft, xp, dimension, location, advancement
- [ ] Rewards: command, choice, random, toast, reward tables
- [ ] Teams (parties): create, invite, join, leave, with FTB's merge rules
- [ ] Repeatable and optional quests, sequential tasks
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
