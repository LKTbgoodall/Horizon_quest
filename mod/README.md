# Horizon Quests: the native mod

The bundle the Horizon launcher builds into the game. Version 0.0.2 adds the **server half** of phase 2 (quest engine, inventories, rewards, saving, `/quests`) to the **phase 0 spikes**, which checked that the platform can carry the graphical quest book.

| Spike | What it proves | Hooks |
|---|---|---|
| 1. HUD hint | the bundle builds and runs in the client | `native_mods`, `native_hud` |
| 2. Test book | a mod screen can draw a pan/zoom canvas with item icons and lines, and take clicks, drags and the wheel | `native_key`, `native_mod_screen_open`, `native_screen_render_post`, `native_screen_mouse_*_pre`, `native_item_stack`, `native_quad` |
| 3. Ping/pong | mod messages work between the window and the integrated server in single player | `native_net_send_to_server`, `native_net_from_client`, `native_net_send_to_player`, `native_net_from_server` |
| 4. `/quests` | the server half registers and answers a command | `native_register_commands`, `native_perform_command` |

## Layout
```
mod/
├── Cargo.toml            # cargo test: the pure modules through src/lib.rs
├── src/lib.rs
├── tests/example_pack.rs # loads the example pack and plays it to the end
├── compatibility.json    # game 26.3, hooks 313, pinned Horizon commit
├── LICENSE               # MIT
└── mixins/horizon_quests/
    ├── horizon-mixin.toml
    ├── hq_meta.rs, hq_protocol.rs, hq_canvas.rs   # pure, tested, in the bundle
    ├── hq_model.rs, hq_validate.rs, hq_logic.rs,
    │   hq_progress.rs, hq_store.rs, hq_net.rs,
    │   hq_book.rs, hq_commands.rs                 # pure, tested, NOT in the bundle yet (phase 1)
    ├── hq_client_glue.rs, hq_server_glue.rs       # touch the game
    ├── *.rs                                       # one helper per hook
    └── data/quests/                               # the example pack "Getting started"
```

The phase 1 modules (quest files, rules, engine, saving) are only built by `cargo test` for now. They join `horizon-mixin.toml` together with the glue that calls them, because the game build refuses unused code.

## Unit tests (any PC with Rust)
```bash
cd mod
cargo test
```

## Test in game
1. Get the latest code (on another PC):
   ```bash
   git pull
   ```
2. Package, install and build it. Run this yourself from the repo root; an agent must never run the build step, which passes `--trust-native-code`. Add `-Instance NAME` if your instance is not `mon-horizon`.
   ```powershell
   powershell -ExecutionPolicy Bypass -File tools\build-mod.ps1
   ```
   The script packs `build/horizon_quests.hnmod`, copies it into the instance's `horizon-native-mods`, turns it on in `horizon-mods.txt`, and runs `horizon-launcher native-build --instance mon-horizon` with every native mod of the instance. `-NoBuild` stops before the build; `-Uninstall` takes the mod out.

   Do **not** use `horizon-launcher native-build mod/mixins/horizon_quests` alone: the mod is then missing from the launcher's list, and the next build the launcher starts (its "Compile" button) drops it.

   For faster rebuilds while testing, you can first run `horizon-launcher settings native-profile dev`, and later `horizon-launcher settings native-profile performance`.
3. Start the game and check, in order. Phase 0 (all passed on 2026-11-10, recheck quickly): the Options > Mods row now says **0.0.2**, the HUD hint, K opens the test book with `NET OK`.
   Then **v0.0.2, the server half** (no real book yet: everything shows in chat). Use a **new single-player world** in Survival:
   - [ ] **Build**: it succeeds. If not, send `native/build.log`: the server glue is compiled for the first time.
   - [ ] **`/quests`**: the help, with `/quests progress`, `claim`, `check`, `submit` (and the admin forms for an operator).
   - [ ] **`/quests progress`**: `Getting started: 0/11 quests` and ` - Welcome: open (...)`.
   - [ ] **Checkmark**: `/quests check getting_started/welcome/ready`. Chat: `[Horizon Quests] Quest completed: ...`.
   - [ ] **Items held**: cut 8 logs of any wood (a tag). Within a second of the last pickup, chat says the quest is completed. Then craft the next items the quests ask for (`/quests progress` lists them): crafting must also complete them.
   - [ ] **Items from a chest**: put the items of an open quest in a chest first (creative or `/give` before the quest opens), then take them out: the quest completes within a second.
   - [ ] **Rewards**: `/quests claim`. Chat: `Reward: ...` and the items or XP arrive. With a full inventory, the items drop at your feet.
   - [ ] **Kill task** (if the pack has one open): kill the mob with a melee blow (bow kills are not counted yet).
   - [ ] **Hand-in** (if the pack has one open): `/quests submit <task>` takes the items.
   - [ ] **Saving**: leave the world and come back: `/quests progress` shows the same progress. The file is `saves/<world>/horizon-quests/progress/<your uuid>.json`.
   - [ ] **Admin** (cheats on): `/quests complete <you> getting_started/<quest>`, `/quests reset <you> getting_started/<quest>`, `/quests validate`, `/quests reload`.
4. Report back: what worked, what didn't, and screenshots of the book if possible.
