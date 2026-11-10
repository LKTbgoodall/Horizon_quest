---
tags: [project, questions]
updated: 2026-11-10
---
# Open questions

> Format: `- [ ] **Question**: context → lead`. Once resolved, check it off and note the answer with the date. If it leads to a decision, create an ADR.

## Decisions to make (Bastien)
- [ ] **Quest text**: plain text in the quest files, or translation keys like FTB? In-game text is English-only for now, so plain text is enough to start → [[Horizon Quests data model]]

## Technical unknowns (to check in the code or with Horizon)
### 🔴 Critical for the graphical book ([[ADR-008 Graphical client book first]]): phase 0 spike
- [ ] **Mod messages in solo**: do `native_net_send_to_server` / `native_net_from_client` work between the window game and the integrated server?
- [ ] **Mod screen**: does `native_mod_screen_open` + `native_screen_render_post` + the `native_screen_mouse_*_pre` hooks work as documented? Real signatures of `native_fill`, `native_text`, `native_sprite`, `native_item_stack`, `native_quad`?
- [ ] **2D clip/scissor** for the canvas: is there an undocumented helper (`gui_paint`?)
- [ ] **A single client + server bundle**: how to cleanly separate the two states in the same mod instance?
- [ ] Which screen-input hooks are missing from the typed trait (`screen_key_pressed_pre`, `screen_char_typed_pre`, `screen_mouse_scrolled_pre`…) and must be bound as mixin helpers?

### Other
- [ ] **Dedicated server**: the public doc says "Not there yet", but `native-mods.md` documents `--server-output`. Is it usable today? → test later (solo/LAN comes first)
- [ ] **Exact signatures** of `native_hud`, `native_key`, `native_mod_key`… → read `crates/horizon-client/src/game/native_hooks.rs`
- [ ] **`native_destroy_block`**: how to get the id of the broken block?
- [ ] **`native_player_changed_dimension`**: the doc contradicts itself on solo (NM:2278 vs NM:1291) → test
- [ ] Does **`native_mob_killed`** cover ranged kills (bow, trident)?
- [ ] **Hooks ⏳ pending**: when will `player_inventory_changed`, `native_add_toast`, `native_title`, `native_boss_bar`, `native_has_permission` get a number? → watch the Nightlies, ask on Discord
- [ ] Is **`/loot give`** implemented by Horizon?
- [ ] **Command aliases** (`/q`): possible?
- [ ] **JSON5**: is a parser available in Horizon's `Cargo.lock`? (for the FTB import)

## Resolved
- [x] *Can Horizon load FTB Quests?* → **No**, no JVM or Java Mixins. It has to be rewritten (2026-11-10, [[ADR-001 Native Rust mod for Horizon]])
- [x] *Is there already a quest mod on Horizon?* → **No** (158 projects reviewed on 2026-11-10)
- [x] *Can a server mod show a GUI to vanilla clients?* → **Yes**, via chest menus (`native_open_menu`), like smp (2026-11-10)
- [x] *License?* → **MIT** (2026-11-10, [[ADR-009 MIT license and mod id horizon_quests]])
- [x] *Mod id and name?* → **`horizon_quests`**, "Horizon Quests" (2026-11-10, ADR-009)
- [x] *Language of in-game text?* → **English only** for now; translations later (2026-11-10)
- [x] *Priority target?* → **Solo/LAN first**, code kept ready for dedicated servers (2026-11-10)
- [x] *Teams?* → **Built-in system in V1**; the MVP has per-player progress (2026-11-10)
- [x] *Inventory full on claim?* → **Drop the rest at the player's feet** (2026-11-10, [[Rewards - implementation]])
- [x] *UI order?* → **Graphical book directly**, no chest-menu book (2026-11-10, [[ADR-008 Graphical client book first]])
- [x] *ADRs 002 to 006?* → **All accepted** (2026-11-10, ADR-002's UI part superseded by ADR-008)
- [x] *FTB Quests import?* → **Yes, later** (phase 5); keep our format convertible (2026-11-10)
- [x] *Example chapter?* → **Yes**, a "Getting started" chapter that a config option can turn off (2026-11-10)
- [x] *Audience?* → **Published for everyone** on playhorizon.dev (2026-11-10)
