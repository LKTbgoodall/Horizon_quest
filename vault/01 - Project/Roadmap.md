---
tags: [project, roadmap]
updated: 2026-11-10
---
# Roadmap

> Check items off as you go. The **current phase** is shown in [[00 - Home]].
> Feature priorities: [[FTB task types]], [[FTB reward types]].

## Phase 0: setup ⬅️ IN PROGRESS
- [x] Horizon and FTB Quests research (2026-11-10)
- [x] Project vault (2026-11-10)
- [ ] Validate ADRs 002 to 006 → [[Decision index]]
- [ ] Choose the license and the mod id (`horizon_quest`?) → [[Open questions]]
- [ ] Install the tools → [[Build, test and tools]] § Prerequisites
- [x] `git init`, `.gitignore`, first commit and push to https://github.com/LKTbgoodall/Horizon_quest (2026-11-10, sole author: LKTbgoodall)
- [ ] Re-clone the references into `ext/` (horizon: docs + `native-hooks.json` + `crates/horizon-client/src/game/native_hooks.rs`; horizon-mods)
- [ ] **Hello world**: copy `native/fps-overlay` or `coords-hud`, rename it, build it with the launcher and see it in game
- [ ] **Server hello world**: a `/quests` command that replies in chat (smp's `register.rs` + `command.rs` pattern)
- [ ] Check the real hook signatures in `native_hooks.rs` → update [[Horizon hooks and API]]

## Phase 1: pure logic core (without the game, `cargo test`)
- [ ] `hq_model`: types + serde + loading of `quests.json` and `chapters/*.json`
- [ ] Validation: duplicate ids, unknown dependencies, cycles
- [ ] `hq_logic`: dependencies (4 modes + `min_required`), visibility, startable, linear/flexible
- [ ] `hq_progress`: `QuestEvent` → progress → `Vec<Effect>`
- [ ] `hq_rewards`: item, xp, xp_levels, team/player claim, auto-claim
- [ ] `hq_store`: atomic JSON, migrations
- [ ] Minimal `hq_teams`: implicit personal team
- [ ] Example chapter "Getting started" in `data/`

## Phase 2: playable server MVP 🎯
- [ ] Glue: global state, `server_tick`, `world_lifecycle`, `player_logged_out`
- [ ] MVP commands → [[Commands and permissions]]
- [ ] Tasks: item (polling), checkmark, kill (`mob_killed`)
- [ ] Rewards: item (`give_stack`), xp, levels
- [ ] Chest book: chapters → quests → detail, claim, submit
- [ ] Tracked quest in the sidebar and notifications in chat
- [ ] Saving in `<world>/horizon-quest/`
- [ ] Test in solo, then on a dedicated server (engine export)
- [ ] **Release 0.1.0** (private, among friends)

## Phase 3: V1
- [ ] Tasks: craft, xp, dimension, location, advancement
- [ ] Rewards: command, choice, random, toast, reward tables
- [ ] Parties (teams): create, invite, join, leave, with FTB's merge rules
- [ ] Repeatable and optional quests, sequential tasks
- [ ] TOML config (`native_register_configs`)
- [ ] Datapack quests (`NativeReloadListener`)
- [ ] Translatable text (lang)
- [ ] **0.x publication on playhorizon.dev** → [[Publishing and mod policy]]

## Phase 4: V2, graphical book (Horizon client)
- [ ] `hq_net`: handshake, chunked sending of definitions, progress deltas
- [ ] Mod screen: pan/zoom canvas, shapes, lines, icons
- [ ] Chapter sidebar, quest panel, buttons, `show_recipes`
- [ ] Client toasts, opening by key or by item
- [ ] Assets (combined `.hmod`): shape textures, icon

## Phase 5: V3, editor and import
- [ ] In-game editor (edit mode)
- [ ] FTB Quests import (JSON5/SNBT)
- [ ] API for other mods (custom tasks and rewards)
- [ ] Stat, biome, structure tasks (depending on available hooks)
