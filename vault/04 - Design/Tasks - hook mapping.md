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
| Item **consume** | taken on "Submit" in the book or `/quests submit` | The player whose state is in place (their own command; the host's book): `native_carried_items` + `native_take_items` (9). A hub guest's book, outside any turn: `native_inventory` (132) → `hq_session::hand_in_plan` → `native_run_command("clear <name> <item> <n>")` (21). Plain stacks only | ✅ coded v0.0.6 | MVP |
| **Kill** | event | `native_mob_killed` (40), otherwise `native_living_death(..., attacker, ...)` (392) | ✅ | MVP |
| **Craft** | event, matched to craft tasks on the next tick (where item tags can be read) | `native_item_crafted(player, recipe, stack)` (161); furnace: `native_furnace_extract(player, name, pos, item, count)` (228) | ✅ coded v0.0.6 | V1 |
| **XP Levels** | event; the level is remembered and reported again when a task opens. **Not known before its first change in the session** (`NativeServerPlayer.xp_level` is ⏳) | `native_player_level_change(player, name, from, to)` (347) | ✅ coded v0.0.6 | V1 |
| **Dimension** | **polling only**, with the inventory look, while a task waits for it: the event (154) is not heard outside a dedicated server | `native_server_players()` (8) for hub guests, `native_player()` for the host | ✅ coded v0.0.6 | V1 |
| **Location** | position polling, same look as dimension; a box of blocks, both corners included | `native_server_players()` (8), `native_player()` for the host | ✅ coded v0.0.6 | V1 |
| **Advancement** | event, remembered in `Progress.advancements` when a task of the pack names it, so a quest opening later finds it. **One earned before the mod was installed is not seen** (`native_player_advancements` is ⏳) | `native_player_advancement_done(player, name, advancement)` (249) | ✅ coded v0.0.6 | V1 |
| Break / place a block | event | `native_destroy_block` / `native_place_block` (14). ⚠ We need to retrieve the block id (not read by smp): **to verify** | ✅ ? | V2 |
| **Stat** | event | `native_player_stat_award` (321) | ✅ | V2 |
| **Biome** | polling | `native_chunk_snapshot().noise_biome` + `native_biome_name` | ⏳ | V2 |
| **Structure** | — | ❌ not documented | ❌ | later |
| **Observation** | gaze polling (yaw/pitch + raycast?) | no documented raycast | ❌ ? | later |
| Use, eat an item | event | `native_living_entity_use_item_finish` (90), `native_use_item` (13) | ✅ | V2 |
| Fish, tame, breed, trade | event | `native_player_fish` (160), `native_entity_tame` (220), `native_entity_breed` (213), `native_trade_with_villager` (310) | ✅ | V2 |
| **Custom** | public API for other mods (`[dependencies]`) or the `/quests progress <player> <task> <n>` command | — | ✅ | V2 |

## Engine rules
Coded in `hq_progress.rs` (2026-11-10) for item, hand-in, kill and checkmark tasks; craft, experience, dimension, location and advancement tasks, tasks in order and repeatable quests since v0.0.6.
- A task only progresses if the quest is **open** (dependencies met) in Linear mode, and **always** in Flexible mode (the quest then completes once its dependencies are met too). A completed quest no longer progresses. See [[FTB data model]] and [[Open questions]].
- Counters are **per team**. An event from one member advances the whole team.
- `Item` tasks without consume: progress = `min(count, number found)`; it follows the inventory up **and down** until the task reaches its goal, then it **does not regress**. (With teams: the max found on an online member.)
- `Item` tasks with consume: only a "Submit" in the book counts; partial hand-ins add up.
- A kill counts for **every** open quest that has a matching kill task. A quest opened by that very kill does not count it.
- **Ranged kills caveat**: currently `native_mob_killed` (40) only attributes direct melee blows in `fdc87ae`. Projectiles (bow, crossbow, trident) and indirect sources will need damage source resolution via `native_living_death` (392) once verified.
- **Inventory polling scaling**: item tasks poll 41 slots every `detection_delay` ticks (default 20 = 1s) plus pokes ([[ADR-010 Item detection by polling plus triggers]]). For multiplayer scaling in V1, checks should be staggered across ticks to prevent multi-player tick spikes.
- **Sequential tasks** (`sequential_tasks`, v0.0.6): a task only counts once every task before it is done (`Book::task_counts`); a book request for a later one is refused (`NotYet`).
- **Craft** tasks add up what is taken from a crafting grid or a furnace while the task counts; holding the item is not crafting it.
- **Experience** tasks follow the level up and down until done. **Dimension / location** tasks are done the moment the player is seen there.
- **Repeatable quests**: see [[ADR-011 Repeatable quests start over]].
