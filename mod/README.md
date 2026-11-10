# Horizon Quests: the native mod

The bundle the Horizon launcher builds into the game. Version 0.0.1 contains only the **phase 0 spikes**. They check that the platform can carry the graphical quest book before the real work starts.

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

## Test in game (home PC)
1. Get the latest code:
   ```bash
   git pull
   ```
2. Build it into the instance. Run this yourself; an agent must never pass `--trust-native-code`. Add `--instance NAME` if you don't use the default instance.
   ```bash
   horizon-launcher native-build mod/mixins/horizon_quests --trust-native-code
   ```
   For faster rebuilds while testing, you can first run `horizon-launcher settings native-profile dev`, and later `horizon-launcher settings native-profile performance`.
3. Start the game and check, in order:
   - [ ] **Build**: it succeeds. If not, send `native/build.log` (launcher data folder, or the launcher's "Build log" button).
   - [ ] **Options > Mods**: there is a "Horizon Quests 0.0.1" row, with the key "Open the quest book: K".
   - [ ] **Spike 1**: in a single-player world, the top-left corner shows `Horizon Quests 0.0.1 - K opens the quest book`.
   - [ ] **Spike 2**: K opens the test book: 6 square nodes with item icons, linked by grey lines.
     - drag with the mouse: the canvas pans;
     - the wheel zooms around the pointer;
     - a left click on a node gives it a gold frame and shows its name at the top right;
     - Esc or K closes the book.
   - [ ] **Spike 3**: the footer of the book says `NET OK: pong #1 received`, and chat shows `[Horizon Quests] the server half answered pong #1`. If it says `NET FAILED: ...` or keeps `retrying ping: ...`, send the exact text.
   - [ ] **Spike 4**: `/quests` in chat answers `Horizon Quests 0.0.1 is running on this server. Hello <you>! ...`.
   - [ ] Optional: open the world to LAN and join from a second client that has the mod. Check spikes 2 to 4 there too.
4. Report back: what worked, what didn't, and screenshots of the book if possible.
