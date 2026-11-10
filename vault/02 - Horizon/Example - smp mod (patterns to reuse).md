---
tags: [horizon, reference, example, patterns]
source: horizon-mods/native/smp (v0.36.0), recipes, coords-hud, fps-overlay
verified_on: 2026-11-10
---
# Reference example: the `smp` mod

> [!important] Why this note
> `smp` (economy, claims, market, auctions, casino…) is the **largest server-side native mod** in the registry, and it is made by the Horizon team. It is **the model to copy** for Horizon Quests: server commands, chest menus, JSON saving, global state and tests.
> Code: `horizon-mods/native/smp/` (https://github.com/horizon-rust/horizon-mods/tree/main/native/smp)

Back: [[Horizon hooks and API]] · [[Native mods - structure and manifest]]

## 1. Project layout
```
native/smp/
├── Cargo.toml            # package horizon-mod-smp, edition 2024, MIT ; deps : serde, serde_json
├── src/lib.rs            # #[path = "../mixins/smp/smp_X.rs"] pub mod smp_X;  → cargo test outside the game
├── compatibility.json    # game 26.3, hooks 413, horizon_revision, horizon_mixins_revision
└── mixins/smp/
    ├── horizon-mixin.toml
    ├── register.rs, command.rs, tick.rs…   # 1 helper = 1 injection (parameters = bindings)
    ├── glue.rs, *_glue.rs                  # code that touches the game (not in lib.rs)
    └── smp_*.rs                            # PURE RULES, testable (≈190 #[test])
```
- **Rules / glue separation**: the `smp_*.rs` modules do not depend on game types, which makes them testable with `cargo test`. The `*_glue.rs` files bridge to `Game`. **We reuse this split**: see [[Architecture]].
- Modules become `crate::game::<name>` (`[[module]] file="crates/horizon-client/src/game.rs"`). They reference each other via `super::smp_store`.
- `chrono`, `log`, `ureq` and `flate2` come from the Horizon workspace, without being declared in Cargo.toml. Only crates from Horizon's `Cargo.lock` are allowed.

## 2. Injections used (all `kind="HEAD"`, target `Game<'_>::…` in `horizon_client::game::native_hooks`)
| Hook | Bindings | Use for us |
|---|---|---|
| `native_register_commands` | `commands: &mut Vec<NativeCommand>` | register `/quests …` |
| `native_perform_command` | `game, player: &NativeServerPlayer, command: &str, ran: &mut bool` | run our commands |
| `native_server_tick` | `game, phase: ServerTickPhase` | task detection loop (1×/s), saving |
| `native_level_tick_post` | `game, dimension: &str` | — |
| `native_world_lifecycle` | `phase: WorldPhase` | flush on `WorldCloseFinal \| WorldCloseFailed` |
| `native_menu_clicked` | `game, uuid: u128, window: i32, slot: i32, mode: i32` | **clicks in the book (chest menu)** |
| `native_menu_closed` | `game, uuid, window` | close the view |
| `native_destroy_block` | `game, pos: [i32;3], canceled: &mut bool` | "break a block" task (⚠ the block id is not provided) |
| `native_right_click_block`, `native_use_item_on`, `native_interact_entity` | … | interaction tasks |
| `native_attack_entity` | `game, kind: &str, pos, canceled` | ⚠ does not give the outcome (the kill) |
| `native_living_incoming_damage` | `entity: i32, damage_type: &str, canceled` | ⚠ neither attacker nor amount |
| `native_entity_join_level` | `entity: u32, kind, pos, loaded: bool` | — |
| `native_player_command` | `player: u128, command: &str` | — |
| `native_player_logged_out` | `game, player: u128, name: &str` | save on disconnect |
| chat: `horizon_client_lib::lan::hub::chat::native_player_chat` | `player: u128, message: &str` | — |

What smp does **not** have: no kill, death or login hook (login is detected by diffing `native_server_players()` every second, death by polling `health <= 0`), no pickup, craft or advancement, no toast. → To check in the catalog: [[Horizon hooks and API]].

## 3. Commands
```rust
commands.push(NativeCommand::new("pay <player:word> <amount:integer>"));   // everyone
commands.push(NativeCommand::op("setwarp <name:word>", 3));                 // op level 3
```
- Argument types: `word`, `integer`, `item`, `text` (greedy). `[x]` = optional. **One usage string per subcommand form.** The client gets autocompletion.
- Dispatch: the line arrives without the `/`. We check `game.native_on(ID)` and the first word, then `*ran = true`.
- `NativeServerPlayer`: `uuid` (u128), `name`, `permission` (u8, 0-4), `game_mode`, `position`, `dimension`, `health`, `yaw`, `pitch`, `entity_id`.
- Parsing: `split_whitespace()` then slice patterns (`match args { [verb @ ("give"|"take"), who, amount, ..] => … }`).
- Permission rechecked in the rules: `if me.level < 3 { return vec![Out::Error(..)] }`.
- **Pure output type**: `enum Out { Reply(String), Error(String), Tell(u128,String), All(String) }`, translated by the glue into `native_feedback` / `native_tell` / `native_broadcast`. `native_feedback` only works **inside a command**.

## 4. UI without a client mod: chest menus
```rust
let window: Option<i32> = game.native_open_menu(uuid, 6 /*rows*/, Text::literal("Quests"), items /*Vec<Option<ItemStack>>*/);
game.native_set_menu_items(uuid, items);   // redraw (next page…)
game.native_close_menu(uuid);
```
- Views: `HashMap<u128, (Option<i32> /*window*/, Page)>`. We insert `(None, page)` before opening, then store `Some(w)`.
- Clicks: `mode` 0 = click, 1 = shift-click. `slot` counts from 0. `native_menu_inventory_slot(SIZE, slot)` converts a click in the player's inventory.
- **Pure layout** (`smp_market_menu.rs`): `layout() -> Vec<Cell>` and `click(slot) -> Click`, testable. The last row (45..54) serves as the nav bar: `BACK=45`, `PREV=47`, `NEXT=51`, `CLOSE=53`. `PER_PAGE=45`.
- Item with name and lore:
```rust
let mut stack = ItemStack { item: item.into(), count: 1, damage: 0, extra: None };
stack.data_mut().custom_name = Some(component_nbt(&json!({"text": name, "italic": false, "color": "gold"})));
stack.data_mut().set_other("minecraft:lore", Some(Tag::List(lines)));
```
- Usual icons: `gray_stained_glass_pane` (background), `arrow` (nav), `barrier` (close), `book` (help), `lime_concrete`/`red_concrete` (buttons).
- Bonus: a 128×128 image on a map (`map_id` + `native_send_map`).

## 5. Other visual outputs
- Chat: `Text::literal(s).with_color(text::GOLD).bold()`, clickable text `ClickEvent::RunCommand` / `SuggestCommand` + `hover`.
- Action bar: `game.native_action_bar(uuid, Text)`.
- **Sidebar**: `game.native_sidebar(uuid, Some(&NativeSidebar { title, lines }))`. This is ideal for the **tracked / pinned quest**.
- Tab list: `game.native_tab_list(uuid, &header, &footer)`.
- Titles, boss bars and sounds: via **console commands** `game.native_run_command("title … / bossbar … / playsound …")`, run **after releasing the lock**.
- Toast: ❌ no API (only the sound `minecraft:ui.toast.challenge_complete`).

## 6. Items and inventory
- Inside a command: `native_count_items(id)`, `native_take_items(id,n)`, `native_give_items(id,n)`, `native_room_for(id)`, `native_carried_items()`, `native_held()`.
- Outside a command (tick, click): `native_player_stacks(uuid)`, `native_give_stack(uuid, stack) -> remainder`, or `native_run_command("give …" / "clear …")`.
- Inventory even when offline: `native_inventory(uuid, NativeInventoryKind::Player|Ender)`.
- Serialization: `item_to_nbt` / `item_from_nbt` + `write_network`, in hex.

## 7. Persistence
- **JSON via serde** in `game.native_world_dir()?.join("horizon-smp")`, with one file per game. Players indexed by UUID string (`uuid_string(u128)` / `parse_uuid`).
- `smp_store`:
  - `read_json`: missing file → `Default`; corrupted file → `Err`, and the game disables itself (**we never overwrite**).
  - `save_json`: asynchronous, via an `smp-writer` thread.
  - `write_atomic`: temp + `sync_all` + rename.
  - `flush()`.
- Saving: at the end of the tick, **only the modified games** (`changed`), plus a flush when the world closes.
- `config.json`: `#[serde(default)]` sections, rewritten when a new version adds keys.
- No `AttachmentType` used.

## 8. State and threads
- `static STATE: Mutex<Option<Smp>>`, opened lazily and reopened if the world folder changes. Poison-tolerant locks: `.lock().unwrap_or_else(|e| e.into_inner())`.
- Economical tick: `TICKS: AtomicU64`, work is only done every 20 ticks or if `BUSY`.
- Re-entrancy: `try_lock()` in *fire-and-forget* hooks. Effects that retrigger hooks (commands) go through a `Vec<String>` queue run after `drop(guard)`.
- Events received while a lock is held (logout): static queue processed on the next tick.
- Client side: `thread_local!` + `RefCell`.

## 9. Tests
- `#[cfg(test)] mod tests` in each rules file, with a temp folder `std::env::temp_dir().join(format!("…-{}", process::id()))`.
- **E2E proofs**: `proof.py` launches `horizon-client --dedicated --nogui` and two `horizon-ctl --offscreen` clients that read the chat, the menus (slots numbered from 300) and the inventory. Command: `native.py proof`.

## 10. Client side (`recipes` and `coords-hud` mods)
- Panel next to container screens: `native_panel_items`, `native_panel_recipes`, `native_panel_event` (`Search`, `Clicked`, `Tab`, `Action`, `Back`…). The game renders the item icons itself.
- HUD: `native_hud(game, batch, corners)`, `native_fill`, `native_text`, `native_text_width`.
- Options > Mods row: `NativeMod { id, title, description, keys }`. Keys via `native_mod_key`. Settings via `native_setting` / `native_set_setting` (`horizon-mods/.horizon-mod-data/<id>/<key>`).
- Client command: `native_command(game, command, ran)`, to be distinguished from the server-side `native_perform_command`.
