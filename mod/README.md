# Horizon Quests: the native mod

The bundle the Horizon launcher builds into the game. Version 0.0.3 adds the **graphical quest book** (`hq_book`) and **real network synchronization** (`hq_net`) to the quest engine (`hq_session`, `hq_server_glue`), replacing the temporary spike screen with the full interactive quest UI.

| Feature | What it provides | Hooks / Modules |
|---|---|---|
| **Graphical Book** | Interactive quest canvas: chapter sidebar, zoom, pan, quest details panel, buttons | `hq_book`, `hq_client_glue`, `native_mod_screen_open`, `native_screen_render_post`, input hooks |
| **Network Sync** | Reliable framing (<=30KB frames, 16 frames/tick) for definitions, progress, actions | `hq_net`, channel `horizon_quests:book`, `native_net_send_*`, `native_net_from_*` |
| **Server Engine** | Tasks (inventory, craft, furnace, kill, checkmark), rewards, atomic JSON saves, `/quests` | `hq_server_glue`, `hq_session`, `hq_progress`, `hq_store` |

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
3. Start the game and check, in order. The Options > Mods row now says **0.0.3**.
   Use a **new single-player world** in Survival:
   - [ ] **Build**: it succeeds. If not, send `native/build.log`.
   - [ ] **Open the Book (K)**: opens the real graphical quest book! You should see the chapters on the left sidebar ("Getting started"), the canvas with quest nodes, dependency lines, icons. The HUD hint disappears while the book is open.
   - [ ] **Pan & Zoom**: drag to pan the canvas, mouse wheel to zoom in and out.
   - [ ] **Quest selection**: clicking a quest opens its details panel on the right with description, tasks, and reward previews.
   - [ ] **Checkmark task**: select the "Welcome" quest and click the confirmation button. It marks as completed and sends updates to the server.
   - [ ] **Inventory task**: gather 8 logs. The quest completes and updates on both the server and in the graphical book in real time.
   - [ ] **Claim rewards**: click the claim button in the quest panel. Items and XP arrive; leftovers drop at your feet if inventory is full.
   - [ ] **Saving**: leave the world and rejoin: pressing K reloads your progress accurately from `saves/<world>/horizon-quests/progress/<your uuid>.json`.
   - [ ] **`/quests` commands**: `/quests progress`, `check`, `claim`, `submit`, `reload`, `validate` continue to work alongside the graphical book.
4. Report back: what worked, what didn't, and screenshots of the book!
