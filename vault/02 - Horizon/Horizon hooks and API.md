---
tags: [horizon, reference, api, hooks]
source: horizon/docs/native-mods.md (NM, 5523 lines), native-hooks.json (434 hooks, 177 helpers), mods.md, native-hooks-neoforge.md, api/stability.md, api/server.md
verified_on: 2026-11-10
---
# Horizon hooks and API ("quest mod" view)

Back: [[Horizon - Overview]] · Concrete examples: [[Example - smp mod (patterns to reuse)]]

> [!warning] Legend
> - **API N**: requires `"hooks": N` at minimum. The current level on main is **≥ 415** (NF says 409 on Nov 5, NM documents up to 415). `smp` targets 413.
> - **⏳ pending**: documented in the form `@API:name@`, with no number for now, and **absent from the `native-hooks.json` snapshot**. It **cannot** be targeted yet. Recheck on every Horizon update: [[Open questions]].
> - Many signatures (`native_hud`, `native_fill`, `native_text`, `native_sprite`, `native_key`…) are **not documented**: you have to read `crates/horizon-client/src/game/native_hooks.rs` in Horizon's code.
> - `NM:1234` references point to lines of `docs/native-mods.md` in the horizon repository.

## 1. Network and client ↔ server sync
| Need | API | Status |
|---|---|---|
| Mod messages | `native_net_send_to_player(player, channel, data)` / `native_net_send_to_server(channel, data)`; receiving `native_net_from_client(player, channel, data)` / `native_net_from_server(channel, data)` | ✅ API 40 |
| Channel | `horizon:mod_net`, subchannel `mod_id:name` (`[a-z0-9_]` : `[a-z0-9_./-]`, 1-64 characters each) | |
| Limits | **30,000 bytes per message, 32 messages per tick** in each direction and per player. All mods hear everything, so you must filter by channel. A vanilla client **never** receives these messages. | |
| Synced attachments | `AttachmentType { sync: true }`: sent to Horizon clients (the player and those tracking them, or the whole level), 1 MiB max | ✅ |
| Raw payload | `native_send_custom_payload_to_player` (1 MiB, also reaches vanilla clients), `native_custom_payload_from_client` | ⏳ |
| Tracking | `native_player_start_tracking` / `stop_tracking` | ✅ API 141 |
| Connection | `native_player_logged_in` / `native_player_logged_out` (paired, including on server shutdown) | ✅ API 57 |

⚠️ Data coming from the client is **untrusted**: treat it as player input and revalidate everything on the server side.

## 2. Task triggers (server side)
| Task | Hook | Status |
|---|---|---|
| **Obtain an item** | `native_player_inventory_changed(player, stacks)`: 41 slots, fired by the vanilla `InventoryChangeTrigger` check. « *what quest … mods need instead of polling* » | ⏳ (ideal) |
| | `native_item_picked_up` (UUID, `canceled`) | ✅ API 40 |
| | `native_item_picked_up_v2(player, stack, thrower, position, canceled)` | ⏳ |
| | **Fallback**: polling `native_inventory(uuid, kind)` (41 slots) every N ticks | ✅ API 132 |
| | `native_count_items(item)` (active player, simple stacks) | ✅ API 9 |
| **Craft** | `native_item_crafted` (uuid, recipe id, result with components). Grid and table only | ✅ API 161 |
| **Smelt** | `native_furnace_extract(player, name, pos, item, count)` | ✅ API 228 |
| Others | `native_anvil_craft_post` (327), `native_enchant_item` (209), `native_trade_with_villager` (310), `native_brewing_complete` (198) | ✅ |
| **Kill** | `native_mob_killed` (melee, sweep, spear; names the player) | ✅ API 40 |
| | `native_living_death(entity, kind, damage_type, attacker, cancel)` (mobs and players) | ✅ API 392 |
| | `native_loot_drops(context, drops)`: the context includes the killing or breaking player | ✅ API 75 |
| Player death | `native_player_died` | ✅ API 40 |
| **Break a block** | `native_destroy_block` (player state injected; ⚠ smp does not read the block id) | ✅ API 14 |
| Place a block | `native_place_block` | ✅ API 14 |
| **Advancement** | `native_player_advancement_done(player, name, advancement)` | ✅ API 249 |
| | `native_player_advancement_progress` (320), `native_player_advancement_criterion_grant` (339) | ✅ |
| **Dimension** | `native_player_changed_dimension(player, name, from, to)`. ⚠ The docs contradict themselves on singleplayer (NM:2278 vs NM:1291): **to be tested** | ✅ API 154 |
| | `native_player_portal` (271), `native_player_teleported` (170) | ✅ |
| **Position / zone** | `native_player_tick` (every tick for every player) or polling `native_server_players()` | ✅ API 31 |
| | `native_player_move(...)` | ⏳ |
| **Biome** | server: `native_chunk_snapshot(...).noise_biome` + `native_biome_name` | ⏳ |
| **Structure** | ❌ **not documented** | ❌ |
| **Stat** | `native_player_stat_award(player, name, stat_type, stat, value, cancel)` | ✅ API 321 |
| | reading `native_player_stats(uuid)`, `native_player_stat(...)` | ⏳ |
| **XP** | `native_player_xp_change` (394), `native_player_level_change(player, name, from, to)` (347) | ✅ |
| Use an item | `native_use_item` (13), `native_use_item_on` (13), `native_living_entity_use_item_finish` (90, eat/drink) | ✅ |
| Interact with an entity | `native_interact_entity` (29) | ✅ |
| Fish, tame, breed | `native_player_fish` (160), `native_entity_tame` (220), `native_entity_breed` (213) | ✅ |
| Chat | `native_player_chat(player, name, message, replies, cancel)` | ✅ API 399 |
| **Custom trigger** | `native_trigger_criterion(uuid, trigger, test)`: fires a datapack advancement | ✅ API 69 |
| Tick | `native_server_tick` (4), `native_level_tick_pre/post` (23) | ✅ |

## 3. Giving rewards
| Reward | API | Status |
|---|---|---|
| **Item with components** | `native_give_stack(uuid, stack)` from any server hook. **Returns what did not fit, nothing is dropped**: it is up to us to drop or store it | ✅ API 98 |
| Simple item | `native_give_items(item, count)` (active player only) | ✅ API 9 |
| Build a stack | `ItemStack::from_nbt`, `mod_api::parse_snbt`, `native_item_from_snbt` | partial ⏳ |
| **XP** | `native_give_experience(uuid, points)`, `native_give_levels(uuid, levels)`. Returns `false` if the player list is busy | ✅ API 101 |
| **Command** | `native_run_command(line)`: console, level 4, no output, **only where no player is injected** (tick, menu). Rewards are therefore **deferred to the tick** | ✅ API 21 |
| | `native_run_command_captured`, `native_run_command_as(player, line)` | ⏳ |
| Message | `native_tell(uuid, text)`, `native_broadcast`, `native_feedback` (inside a command), `native_action_bar(uuid, text)` | ✅ |
| Title / boss bar | `native_title(...)`, `native_boss_bar(...)`. Meanwhile: `title` / `bossbar` commands via `native_run_command` (like smp) | ⏳ |
| **Toast** | client: `native_add_toast(id, icon, title, description, challenge)`. « *A quest mod's server half tells the player's client half (mod messages) and the client half calls it* ». On the vanilla side: go through an advancement + `native_trigger_criterion` | ⏳ / ✅ |
| Sound | server: `native_play_sound_to` (⏳), `playsound` command; client: `native_play_local_sound` (83) | |
| Effects, teleport | `native_add_effect`, `native_teleport_player` | ⏳ |
| Loot table | no "roll and give" helper: we go through `native_run_command("loot give …")` (unverified) or **our own tables** | ❓ |

## 4. Persistence
| Method | Details | For us |
|---|---|---|
| **Files in the world** | `native_world_dir()`: « *a server mod keeps its data in a folder of its own so the world's backups carry it* » (the smp pattern: `world/horizon-smp/*.json`) | ✅ **main choice**: [[ADR-005 Progress as JSON in the world folder]] |
| AttachmentType | `Holder::Level/Chunk/BlockEntity/Entity/Player(uuid)`, NBT `Compound` codec. ⚠ **Online** players only. On respawn, only `copy_on_death: true`. Without a codec, the value is lost on restart | useful for **syncing** per-player data to the client |
| Player files | `native_player_load_from_file` / `save_to_file` (323/324): `players/data/<uuid>.<suffix>` | option |
| Config | `native_config_dir()` (26); `native_register_configs` + `NativeConfig::new(mod_id, kind, default)` → `config/<id>-<kind>.toml` (397), hooks `native_config_loading/reloading/unloading` | ✅ mod config |
| Datapacks | `native_add_server_reload_listeners` + `NativeReloadListener::json(id, folder)` reads `data/<ns>/<folder>/*.json` from all packs, applied on every `/reload` (400) | 💡 **quests shippable as a datapack!** |
| Embedded data | `native_data!("<mod id>", "path")` (41) | default quests |
| Client settings | `native_setting` / `native_set_setting`, `native_data_dir(mod_id)` (⏳) | UI preferences |

## 5. Interface
### Server side, compatible with vanilla clients
- **Chest menus** (API 10): `native_open_menu(uuid, rows, title, items)`, `native_set_menu_items`, `native_close_menu`. Clicks in `native_menu_clicked(uuid, window, slot, button, mode)`: « *No click moves anything* ». `native_menu_closed`. Displayed as `generic_9x1..9x6`.
- **Sidebar**: `native_sidebar(uuid, sidebar)`, 15 lines, for a single player (⚠ not on `Host`, call it from a mixin helper). `native_tab_list` (49).
- Chat and action bar (see above). 128×128 maps (chapter images?). Client holograms (⏳).

### Client side (Horizon only): a real custom screen
- **Mod screen** (API 111): `native_mod_screen_open(mod_id)`, `native_mod_screen()`, `native_mod_screen_close(mod_id)`. Esc closes it. It does not pause singleplayer and only opens "over the world".
- Rendering in `native_screen_render_post` (78): screen id, mouse in GUI pixels, partial tick.
- **Inputs** (all cancellable):
  - `native_screen_mouse_button_pressed_pre` / `_released_pre` (80);
  - `native_screen_mouse_dragged_pre(screen, button, delta, modifiers, canceled)` (123) → **canvas pan**;
  - `native_screen_mouse_scrolled_pre` (93) → **zoom**;
  - `native_screen_key_pressed_pre`, `native_screen_char_typed_pre` (104) → **text input (editor)**;
  - `native_screen_key_released_pre` (118);
  - `native_screen_opening/init_post/closing` (365-367).
- **Drawing**:
  - `native_fill`, `native_text`, `native_text_width`, `native_sprite` (GUI textures `assets/<ns>/textures/gui`);
  - `native_quad(batch, points, argb)` → **dependency lines**;
  - `native_item_stack(batch, stack, at, size)` (313) → **quest icons**;
  - `native_item_model` (111);
  - `native_mod_image(w, h, rgba)` (355);
  - nine-slice, tooltips and `gui_paint` (411, details not documented);
  - `native_draw_scene` (3D).
- ❌ What **does not exist**: a general 2D clip/scissor, a widget library, a text area, a scrollable list. **Everything has to be built by ourselves** (see [[UI - Quest book]]).
- **Recipe viewer**: `native_show_recipes(stack, lookup)`. « *A mod that draws items on screens of its own (a quest book, a guide) can open whatever recipe viewer is installed* ».
- HUD: `native_hud`, `native_gui_layer_pre/post` (139), `native_gui_render_post` (410), `native_world_primitives` (lines, boxes and text in the world, `.through_walls()`) → **quest waypoint**.
- Opening the book:
  - key: `native_key`, `native_input_key` (121), `native_mod_key` / `native_set_mod_key` (411);
  - client command: `native_commands` / `native_command` (35);
  - claimed item: `native_claim_items(mod_id, ids)` + `native_poll` (116), a right click with that item never reaches the server;
  - button drawn on the inventory.
- Reading the client's inventory: `host.own_inventory()` (⏳), `host.menu_stacks()`, `native_player()`.

## 6. Commands
- `native_register_commands(commands)` (API 8), called once when the world opens.
- Creation: `NativeCommand::new("quests open [quest:word]")` or `NativeCommand::op("quests reload", 2)`.
- Argument types: `word`, `text` (rest of the line, last), `integer`, `number`, `boolean`, `player`, `item`, `block_pos`. Optional arguments go at the end in `[]`. ⏳ `word?`, `text?`, `player_name`.
- Execution: `native_perform_command(player, command, ran)`. **This is not a Brigadier executor**: you parse the line yourself, the tree is only used for autocompletion on the client side.
- Console: `native_perform_console_command` (99).
- Permissions: op levels 0-4 via `NativeCommand::op`. Permission nodes (`native_has_permission(player, node, default_level)`, `NativeCommand::permission(node)`) are ⏳.

## 7. Typed API: dispatch rules
- « *Mixin injections run first … The first `Cancel` stops dispatch. For replacement hooks, the first `Some` wins. Collection hooks call every mod.* »
- Hooks take `&self` and must be `Send + Sync` → state in `Mutex` or atomics. One instance serves **both** the windowed game and the server game.
- Tests: `horizon_client::mod_api::dispatch_<hook>(&mut host, &mods, args…)` with a `Host` double.
