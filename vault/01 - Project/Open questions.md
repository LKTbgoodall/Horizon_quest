---
tags: [project, questions]
updated: 2026-11-10
---
# Open questions

> Format: `- [ ] **Question**: context → lead`. Once resolved, check it off and note the answer with the date. If it leads to a decision, create an ADR.

## Decisions to make (Bastien)
- [ ] **License**: MIT (like Horizon and smp)? GPL? → a free license is needed to publish
- [ ] **Mod id and name**: `horizon_quest` / "Horizon Quest"? (id: `a-z0-9_`)
- [ ] **Language of in-game text**: FR, EN, or both via `lang/` from the MVP?
- [ ] **Quest text**: direct text or translation keys (like FTB)? → [[Horizon Quest data model]]
- [ ] **Teams**: minimal built-in system (proposed) or dependency on another mod?
- [ ] **Item not given** (inventory full): drop it on the ground, or a reward "mailbox"? → [[Rewards - implementation]]
- [ ] **Priority target**: solo/LAN or dedicated server?

## Technical unknowns (to check in the code or with Horizon)
- [ ] **Dedicated server**: the public doc says "Not there yet", but `native-mods.md` documents `--server-output`. Is it usable today? → test
- [ ] **Exact signatures** of `native_hud`, `native_fill`, `native_text`, `native_sprite`, `native_key`, `native_mod_key`… → read `crates/horizon-client/src/game/native_hooks.rs`
- [ ] **`native_destroy_block`**: how to get the id of the broken block?
- [ ] **`native_player_changed_dimension`**: the doc contradicts itself on solo (NM:2278 vs NM:1291) → test
- [ ] Does **`native_mob_killed`** cover ranged kills (bow, trident)?
- [ ] **Hooks ⏳ pending**: when will `player_inventory_changed`, `native_add_toast`, `native_title`, `native_boss_bar`, `native_has_permission` get a number? → watch the Nightlies, ask on Discord
- [ ] **Mod messages in solo**: do they work between the windowed game and the integrated server?
- [ ] Is **`/loot give`** implemented by Horizon?
- [ ] **A single client + server bundle**: how to cleanly separate the two states in the same mod instance?
- [ ] **2D clip/scissor** for the canvas: is there an undocumented helper (`gui_paint`?)
- [ ] **Command aliases** (`/q`): possible?
- [ ] **JSON5**: is a parser available in Horizon's `Cargo.lock`? (for the FTB import)

## Resolved
- [x] *Can Horizon load FTB Quests?* → **No**, no JVM or Java Mixins. It has to be rewritten (2026-11-10, [[ADR-001 Native Rust mod for Horizon]])
- [x] *Is there already a quest mod on Horizon?* → **No** (158 projects reviewed on 2026-11-10)
- [x] *Can a server mod show a GUI to vanilla clients?* → **Yes**, via chest menus (`native_open_menu`), like smp (2026-11-10)
