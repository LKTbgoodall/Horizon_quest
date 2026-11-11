---
tags: [horizon, reference, api, hooks]
source: horizon/docs/native-mods.md (NM, 5523 lines), native-hooks.json (434 hooks, 177 helpers), mods.md, native-hooks-neoforge.md, api/stability.md, api/server.md
verified_on: 2026-11-11
---
# Horizon hooks and API ("quest mod" view)

Back: [[Horizon - Overview]] · Concrete examples: [[Example - smp mod (patterns to reuse)]]

> [!warning] Legend
> - **API N**: requires `"hooks": N` at minimum. `NATIVE_HOOKS_API` is **415** at Horizon `1453e0e` (2026-11-10). `smp` targets 413; our mod targets 313.
> - **⏳ pending**: documented in the form `@API:name@`, with no number for now, and **absent from the `native-hooks.json` snapshot**. It **cannot** be targeted yet. Recheck on every Horizon update: [[Open questions]].
> - Many signatures (`native_hud`, `native_fill`, `native_text`, `native_sprite`, `native_key`…) are **not documented**: you have to read `crates/horizon-client/src/game/native_hooks.rs` in Horizon's code.
> - `NM:1234` references point to lines of `docs/native-mods.md` in the horizon repository.

## 0. Verified signatures (read in `ext/horizon`, commit `1453e0e`, 2026-11-10)
All of these are methods of `Game<'_>` in `crates/horizon-client/src/game/native_hooks.rs` (helpers are `pub(super)`, so modules under `crate::game` can call them). Injection target: `horizon_client::game::native_hooks::Game<'_>::<name>`. Working bindings are in `mod/mixins/horizon_quests/horizon-mixin.toml`.

**Hooks** (`&self` ones can't mutate the game: keep state in a `thread_local!` on the client, atomics or a `Mutex` on the server)
```rust
fn native_tick(&mut self)                                   // every Game; check native_window()
fn native_server_tick(&mut self, context: &NativeLifecycleContext, phase: ServerTickPhase)
fn native_hud(&mut self, batch: &mut HudBatch, corners: &mut Corners)
fn native_key(&mut self, key: &str, pressed: bool)          // "key.keyboard.k"; fires on any screen (chat too!)
fn native_mods(&mut self, mods: &mut Vec<NativeMod>)        // NativeMod { id, title, description, keys: Vec<[String; 3]> }
fn native_command(&mut self, command: &str, ran: &mut bool) // client-side command
fn native_screen_render_post(&self, batch: &mut HudBatch, screen: &NativeScreenView<'_>)
fn native_screen_mouse_button_pressed_pre(&self, screen: &NativeScreenView<'_>, button: i32, modifiers: u32, canceled: &mut bool)
fn native_screen_mouse_button_released_pre(&self, screen, button: i32, modifiers: u32, canceled: &mut bool)
fn native_screen_mouse_dragged_pre(&self, screen, button: i32, delta: [f32; 2], modifiers: u32, canceled: &mut bool) // API 123
fn native_screen_mouse_scrolled_pre(&self, screen, scroll: [f64; 2], modifiers: u32, canceled: &mut bool)            // API 93
fn native_screen_key_pressed_pre(&self, screen, key: &str, repeat: bool, modifiers: u32, canceled: &mut bool)       // API 104
fn native_register_commands(&mut self, commands: &mut Vec<NativeCommand>)
fn native_perform_command(&mut self, player: &NativeServerPlayer, command: &str, ran: &mut bool)
fn native_net_from_client(&mut self, player: u128, channel: &str, data: &[u8])  // server, tick after arrival
fn native_net_from_server(&mut self, channel: &str, data: &[u8])               // window only
```
⏳ `_v2` variants of the drag and scroll hooks lend `&mut self` (`screen_mouse_input_v2`), but they are not numbered yet.

**Helpers**
```rust
fn native_mod_screen_open(&mut self, mod_id: &str) -> bool  // only over Playing, with a player, on the window
fn native_mod_screen_close(&mut self, mod_id: &str)
fn native_mod_screen(&self) -> Option<&str>
fn native_fill(&self, batch: &mut HudBatch, rect: [f32; 4], argb: u32)
fn native_quad(&self, batch: &mut HudBatch, points: [[f32; 2]; 4], argb: u32)
fn native_text(&self, batch: &mut HudBatch, text: &str, at: [f32; 2], rgb: u32, shadow: bool)
fn native_text_width(&self, text: &str) -> f32
fn native_item_stack(&self, batch: &mut HudBatch, stack: &ItemStack, at: [f32; 2], size: f32) -> bool // API 313
fn native_item_model(&self, batch, item: &str, model: &str, at: [f32; 2], size: f32) -> bool        // API 111
fn native_cursor(&self) -> [f32; 2]
fn native_mod_key(&self, mod_id: &str, name: &str, default: &str) -> String
fn native_playing(&self) -> bool      // playing with nothing over the world
fn native_window(&self) -> bool       // this Game is the window's
fn native_chat(&mut self, text: &str) // a client-side chat line
fn native_net_send_to_server(&mut self, channel: &str, data: &[u8]) -> Result<(), String>        // needs self.remote, joined
fn native_net_send_to_player(&mut self, player: u128, channel: &str, data: &[u8]) -> Result<(), String>
fn native_feedback(&mut self, line: Text)        // inside perform_command
fn native_tell(&mut self, uuid: u128, line: Text) -> bool
```
**Types**:
- `NativeScreenView { id, menu, title, rect, slots, mouse, partial_tick }`: `rect` is the **whole window** for a mod screen.
- `ItemStack { item: Arc<str>, count: u32, damage: u32, extra: Option<Box<StackData>> }` (`horizon_client::items`).
- `Text::literal(impl Into<String>)`.
- `NativeCommand::new(usage)` / `::op(usage, level)`.
- `NativeServerPlayer { uuid, name, permission, dimension, position, game_mode, health, yaw, pitch, entity_id, address, … }`.
- Paths used in helpers: `crate::game::Game<'_>`, `crate::game::native_server::{NativeCommand, NativeServerPlayer}`, `crate::game::native_hooks::NativeMod`, `horizon_client::hud::HudBatch`, `horizon_client::extras_hud::Corners`, `horizon_client::extras::Corner`, `horizon_client::mod_api::NativeScreenView<'_>`.
- A target function's body has no `cfg` and no macros (horizon-mixins refuses them); helpers and modules may use them.
- Other files: `crates/horizon-client/src/game/native_text_field.rs` (a text field helper, to look at for the editor) and `native_permissions.rs`.

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
| **Advancement** | `native_player_advancement_done(player, name, advancement)`. **Not called for the host of a single-player world** (fires only while a guest's state is swapped in: `lan_acting_uuid` is None for the host). Fallback: `native_run_command("execute if entity @a[name=X,advancements={id=true}]")` returns whether it succeeded (verified in source 2026-11-10) | ✅ API 249 (guests only) |
| | `native_player_advancement_progress` (320), `native_player_advancement_criterion_grant` (339) | ✅ |
| **Dimension** | `native_player_changed_dimension(player, name, from, to)`. ⚠ **Not heard outside a dedicated server** at `fdc87ae`: we poll the player's place instead (v0.0.6) | ✅ API 154 |
| | `native_player_portal` (271), `native_player_teleported` (170) | ✅ |
| **Position / zone** | `native_player_tick` (every tick for every player) or polling `native_server_players()` | ✅ API 31 |
| | `native_player_move(...)` | ⏳ |
| **Biome** | server: `native_chunk_snapshot(...).noise_biome` + `native_biome_name` | ⏳ |
| **Structure** | ❌ **not documented** | ❌ |
| **Stat** | `native_player_stat_award(player, name, stat_type, stat, value, cancel)` | ✅ API 321 |
| | reading `native_player_stats(uuid)`, `native_player_stat(...)` | ⏳ |
| **XP** | `native_player_xp_change` (394), `native_player_level_change(player, name, from: i32, to: i32)` (347, after the new level is applied; a guest's state is swapped in). **347 is never called for the single-player host** (`native_player_level_changed` needs `lan_acting_uuid`, set only for guests; seen in game 2026-11-10: `/xp add` did not move the task). Fallback used since v0.0.7: `native_run_command` (API 21, returns true when the command succeeded, works on the integrated server of every single-player world) with `execute if entity @a[name=X,level=N..]`, binary search up to the task's goal. The selector reads the host's own state for the host (`Target::Player`). Reading the level without an event: `NativeServerPlayer.xp_level` is ⏳ (`@API:server_player_state@`) | ✅ |
| **Craft** | `native_item_crafted(player, recipe: &str, stack: &ItemStack)` (161, after the take); `native_furnace_extract(player, name, pos, item: &str, count: u32)` (228, no player swapped in) | ✅ |
| Use an item | `native_use_item` (13), `native_use_item_on` (13), `native_living_entity_use_item_finish` (90, eat/drink) | ✅ |
| Interact with an entity | `native_interact_entity` (29) | ✅ |
| Fish, tame, breed | `native_player_fish` (160), `native_entity_tame` (220), `native_entity_breed` (213) | ✅ |
| Chat | `native_player_chat(player, name, message, replies, cancel)` | ✅ API 399 |
| **Custom trigger** | `native_trigger_criterion(uuid, trigger, test)`: fires a datapack advancement | ✅ API 69 |
| Tick | `native_server_tick` (4), `native_level_tick_pre/post` (23) | ✅ |

### Whose state is in place (read in `ext/horizon` `1453e0e`, 2026-11-10; corrected at the pin `fdc87ae` on 2026-11-10, Session 13)
- The **item helpers of API 9** (`native_carried_items`, `native_take_items`, `native_count_items`…) act on the **acting player**: inside `native_perform_command` that is the command's player; in a hook with no player swapped in, it is the **host**.
- `native_net_from_client` runs from `run_native_net` "with the hub back in the game": **no player swapped in**. So a book request that takes items must reach the player from outside (below).
- ❌ `native_as_player(uuid, f)` (`native_server.rs`) is a **private** method at the pin `fdc87ae` (plain `fn`, not `pub(super)` as this note said): a mod's module is a sibling of `native_server`, so calling it is `error[E0624]: method native_as_player is private`. v0.0.4 and v0.0.5 called it and **did not compile** (proved by `tools/check-glue.ps1`, Session 13). The engine keeps it for `native_give_experience` / `native_give_levels`. **Never call an engine method that is not in `native-hooks.json` or `native-mods.md`.**
- Since v0.0.6 a book hand-in uses catalog helpers only. The host (not among `native_server_players()`): the state in place, `native_carried_items` + `native_take_items` (9). A hub guest: `native_inventory(uuid, Player)` (132) to count their plain stacks, then `native_run_command("clear <name> <item> <n>")` (21), which runs where no player is swapped in. `/clear` goes by item id, so an item the guest also holds in a damaged, named or enchanted stack is left alone (`hq_session::hand_in_plan`).
- `native_server_players()` (8) lists the **hub's guests** that are placed ("empty on a client and in a world nobody else can join"); `NativeServerPlayer` has `dimension` and `position`. The host is not in it: their place is `native_player()` (`x`, `y`, `z`, `dimension`), the game's own state outside a player's turn.
- `native_inventory(uuid, kind)` / `native_set_inventory_slot` / `native_take_stacks` look the player up among the **LAN hub's guests**; anyone else is read from (or written to) their **player file**, and with no hub at all they answer `NoServer`. ❓ **To check in game**: in single player, is the host one of the hub's guests? If not, item polling (`look_at_inventory`) would see the host's saved file, not their live inventory, or nothing.

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
| Config | `native_config_dir()` (26); `native_register_configs(&mut self, configs: &mut Vec<NativeConfig>)` + `NativeConfig::new(mod_id, NativeConfigKind::{Client,Common,Server}, default)` → `config/<id>-<kind>.toml` (397), hooks `native_config_loading/reloading(&mut self, config: &NativeConfigFile)` and `native_config_unloading`. Types in `crate::game::native_configs`. A `Server` config is read before each world opens, from `<world>/serverconfig/<file>` when there is one; files are re-read on change once a second. Every mod hears every config: check `mod_id`. Present at `fdc87ae` (API 411). | ✅ used since v0.0.5 (`example_chapter`) |
| Datapacks | `native_add_server_reload_listeners(&mut Vec<NativeReloadListener>)` + `NativeReloadListener::json(id, folder)` (`crate::game::native_reload_listeners`) reads `data/<ns>/<folder>/**/*.json` from the resource root and the world's enabled packs (top pack wins per id), then `native_server_reload_listener_apply(listener, files: &[(String, serde_json::Value)])` with ids `<ns>:<path>` sorted; on every data load (world opening, `/reload`), before `native_server_data_loaded`; every mod hears every listener (400) | ✅ used since v0.4.0 (type-checked on `fdc87ae`, not yet seen in game) |
| Embedded data | `native_data!("<mod id>", "path")` (41) | default quests |
| Client settings | `native_setting(&self, mod_id, key) -> Option<String>` / `native_set_setting(&self, mod_id, key, value)` (one file per key in `native_data_dir`) | ✅ used since v0.5.0: the tracked quest (key `tracked`) |
| Recipes (client) | `native_recipes(&mut self) -> Option<Arc<NativeRecipes>>` (API 2; `crate::game::native_hooks::{NativeRecipes, NativeRecipe, NativeRecipeKind, NativeRecipeStack, NativeItem}`, all `pub(crate)`): every item (id, name in the player's language, tags) and every recipe the window knows (kind, shaped grid `width` x `height`, `slots` of alternatives, `results`, cooking time); None outside a world. `&mut self`: call it from the client tick, not from a screen hook | ✅ used since v0.5.0: the book's recipe view |

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
  - `native_fill`, `native_text`, `native_text_width`, `native_sprite(&self, batch, name, src: [x,y,w,h] in the sprite's pixels, dst: GUI rect) -> bool` (API 19; GUI textures `assets/<ns>/textures/gui/<path>.png` of every non-minecraft namespace are packed into the HUD atlas as `<ns>:gui/<path>`; no tint, false when missing; **a GUI texture over 1024 wide or 512 tall is left out of the atlas** without a warning (`horizon-client-render/src/hud.rs`, checked 2026-11-11 on `fdc87ae`); `block/` and `item/` textures and those with an `animation` mcmeta keep only their first square frame) ✅ used since v0.6.0 for the book's node frames;
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

## Client player gaps (verified 2026-11-10)
- On the client, `native_player()` is **None during a dimension change** (`enter_remote_dimension` sets `self.player = None` until the next level is in), not only out of a world. Do not treat None as "left the world" for state a server message may refill meanwhile (our toasts were lost that way until v0.0.9).

## Keys and items (verified in source 2026-11-10)
- **Controls > Key Binds cannot take a mod's key**: the page lists `keybinds::all_mappings()` (static `MAPPINGS` + `HORIZON_MAPPINGS`). A native mod's keys (`NativeMod.keys` from `native_mods`) are rebindable on its page in Options > Mods; `native_mod_key(mod_id, name, default)` reads the choice (`key.<mod>.<name>` in `horizon-mods.txt`).
- **Items in code**: `native_items(items: &mut Vec<horizon_mods::content::Item>)` (API 145, static, asked once at start on every side). `Item { id: "ns:path", template, max_stack_size, rarity, components (patch, `!minecraft:x` removes), crafting_remainder, place_block }`. Look = the template's unless a content mod whose id is the namespace is on (its assets). Counted in the joining client's content check.
- **Right-click with an item, client side**: `native_claim_items(mod_id, &[ids])` + `native_poll(mod_id)` → `NativeModEvent::UseStart(NativeHeldItem, ticks)` / `UseStop` (API 116, module `native_claimed_items`). The server never hears a claimed item's use.
