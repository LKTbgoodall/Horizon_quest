---
tags: [design, tasks, hooks]
status: draft
date: 2026-11-10
---
# Tasks: mapping to Horizon hooks

Sources: [[FTB task types]] × [[Horizon hooks and API]]

| Task | Detection strategy | Hook / helper | API status | Phase |
|---|---|---|---|---|
| **Checkmark** | click in the book (menu or screen) or `/quests check <id>` | `native_menu_clicked`, `native_perform_command`, net message | ✅ | MVP |
| **Item** (have n items) | inventory **polling** every `detection_delay` ticks, then an event once the hook arrives | `native_inventory(uuid, Player)` (132); later `native_player_inventory_changed` (⏳); fast trigger via `native_item_picked_up` (40) | ✅ / ⏳ | MVP |
| Item **consume** | taken on "Submit" click in the book | `native_run_command("clear …")` (from a tick or a menu), measuring before and after (smp `MenuBag` pattern) | ✅ | MVP |
| **Kill** | event | `native_mob_killed` (40), otherwise `native_living_death(..., attacker, ...)` (392) | ✅ | MVP |
| **Craft** | event | `native_item_crafted` (161); furnace: `native_furnace_extract` (228) | ✅ | V1 |
| **XP Levels** | event + read | `native_player_level_change` (347) | ✅ | V1 |
| **Dimension** | event + fallback polling | `native_player_changed_dimension` (154) ⚠ to test in singleplayer; `native_server_players().dimension` | ✅ | V1 |
| **Location** | position polling | `native_server_players()` or `native_player_tick` (31) | ✅ | V1 |
| **Advancement** | event | `native_player_advancement_done` (249) | ✅ | V1 |
| Break / place a block | event | `native_destroy_block` / `native_place_block` (14). ⚠ We need to retrieve the block id (not read by smp): **to verify** | ✅ ? | V2 |
| **Stat** | event | `native_player_stat_award` (321) | ✅ | V2 |
| **Biome** | polling | `native_chunk_snapshot().noise_biome` + `native_biome_name` | ⏳ | V2 |
| **Structure** | — | ❌ not documented | ❌ | later |
| **Observation** | gaze polling (yaw/pitch + raycast?) | no documented raycast | ❌ ? | later |
| Use, eat an item | event | `native_living_entity_use_item_finish` (90), `native_use_item` (13) | ✅ | V2 |
| Fish, tame, breed, trade | event | `native_player_fish` (160), `native_entity_tame` (220), `native_entity_breed` (213), `native_trade_with_villager` (310) | ✅ | V2 |
| **Custom** | public API for other mods (`[dependencies]`) or the `/quests progress <player> <task> <n>` command | — | ✅ | V2 |

## Engine rules
Coded in `hq_progress.rs` (2026-11-10) for item, hand-in, kill and checkmark tasks.
- A task only progresses if the quest is **open** (dependencies met) in Linear mode, and **always** in Flexible mode (the quest then completes once its dependencies are met too). A completed quest no longer progresses. See [[FTB data model]] and [[Open questions]].
- Counters are **per team**. An event from one member advances the whole team.
- `Item` tasks without consume: progress = `min(count, number found)`; it follows the inventory up **and down** until the task reaches its goal, then it **does not regress**. (With teams: the max found on an online member.)
- `Item` tasks with consume: only a "Submit" in the book counts; partial hand-ins add up.
- A kill counts for **every** open quest that has a matching kill task. A quest opened by that very kill does not count it.
- Sequential tasks (V1) only progress one at a time.
