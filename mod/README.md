# Horizon Quests: the native mod

The bundle the Horizon launcher builds into the game. Version 0.0.3 added the **graphical quest book** (`hq_book`) and **network synchronization** (`hq_net`) to the quest engine (`hq_session`, `hq_server_glue`). Version 0.0.4 fixes what the review of 0.0.3 found: the build (dead spike code), claims never shown as claimed in the book, and book hand-ins that took the host's items; it also removes the spike's ping/pong. Version 0.0.5 adds the server config file, whose `example_chapter = false` turns the built-in example chapter off.

| Feature | What it provides | Hooks / Modules |
|---|---|---|
| **Graphical Book** | Interactive quest canvas: chapter sidebar, zoom, pan, quest details panel, buttons | `hq_book`, `hq_canvas`, `hq_client_glue`, `native_mod_screen_open`, `native_screen_render_post`, input hooks |
| **Network Sync** | Reliable framing (<=30KB frames, 16 frames/tick) for definitions, progress, actions | `hq_net`, channel `horizon_quests:book`, `native_net_send_*`, `native_net_from_*` |
| **Server Engine** | Tasks (inventory, craft, furnace, kill, checkmark), rewards, atomic JSON saves, `/quests` | `hq_server_glue`, `hq_session`, `hq_progress`, `hq_store` |

## Layout
```
mod/
├── Cargo.toml            # cargo test: the pure modules through src/lib.rs
├── src/lib.rs
├── tests/example_pack.rs # loads the example pack and plays it to the end
├── compatibility.json    # game 26.3, hooks 397 (config files), pinned Horizon commit
├── LICENSE               # MIT
└── mixins/horizon_quests/
    ├── horizon-mixin.toml
    ├── hq_meta.rs, hq_canvas.rs, hq_model.rs,
    │   hq_validate.rs, hq_logic.rs, hq_progress.rs,
    │   hq_store.rs, hq_net.rs, hq_book.rs,
    │   hq_commands.rs, hq_session.rs, hq_config.rs # pure, tested, in the bundle
    ├── hq_client_glue.rs, hq_server_glue.rs       # touch the game
    ├── *.rs                                       # one helper per hook
    └── data/quests/                               # the example pack "Getting started"
```

The game builds every module with `-D warnings`, and there the modules are private: code that no glue calls is dead code and breaks the build, even when `cargo test` is clean. Remove what the glue stops calling.

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
3. Start the game and check, in order. The Options > Mods row now says **0.0.5**.
   Use a **new single-player world** in Survival:
   - [ ] **Build**: it succeeds. If not, send `native/build.log`.
   - [ ] **Open the Book (K)**: opens the graphical quest book. You should see the chapters on the left sidebar ("Getting started"), the canvas with quest nodes, dependency lines, icons. The HUD hint disappears while the book is open.
   - [ ] **Pan & Zoom**: drag to pan the canvas, mouse wheel to zoom in and out.
   - [ ] **Quest selection**: clicking a quest opens its details panel on the right with description, tasks, and reward previews.
   - [ ] **Checkmark task**: select the "Welcome" quest and click the confirmation button. It completes, in the book too.
   - [ ] **Inventory task**: gather 8 logs. The quest completes and the book shows it without reopening. (This also tells us whether the host's inventory is read in single player.)
   - [ ] **Claim rewards**: click the claim button in the quest panel. Items and XP arrive, **the button goes away and the quest shows as done** (fixed in 0.0.4). Leftovers drop at your feet if the inventory is full.
   - [ ] **Hand in from the book**: the optional "Leftovers" quest (after "First night") takes 8 rotten flesh. Press Submit in its panel: the flesh leaves your inventory and the task counts it (fixed in 0.0.4: it used to take from the host, which in single player is you, so solo looked right already).
   - [ ] **Saving**: leave the world and rejoin: pressing K shows your progress again, read from `saves/<world>/horizon-quests/progress/<your uuid>.json`.
   - [ ] **`/quests` commands**: `/quests` (first line: "Horizon Quests 0.0.5 is running on this server. Hello <you>!"), `progress`, `check`, `claim`, `submit`, `reload`, `validate`.
   - [ ] **Optional, a server without the mod**: on a vanilla server, K opens the book and after about 5 seconds it says "No answer from the server: it may not have Horizon Quests."
   - [ ] **Config file** (new in 0.0.5): after opening a world, `config/horizon_quests-server.toml` exists in the instance's `config/` folder (beside `options.txt`), with `example_chapter = true`.
   - [ ] **Example chapter off**: with the world open, set `example_chapter = false` and save. Within a second or two the log says the file changed, and the book (press K again) shows "This world has no quests." Set it back to `true` and save: "Getting started" comes back, with your progress intact.
   - [ ] **A world's own copy**: copy the file to `saves/<world>/serverconfig/horizon_quests-server.toml` with `example_chapter = false`; that world alone has no example chapter (reopen the world).
4. **LAN test** (needs a second Minecraft account on another PC, with Horizon and this mod):
   - [ ] On the home PC, open the world to LAN; the guest joins.
   - [ ] The guest presses K: the book opens with **their own** progress (not the host's).
   - [ ] The guest completes "Welcome" (checkmark) and claims its reward: the reward goes to the guest, and the host's book does not change.
   - [ ] **Hand-in as a guest** (the 0.0.4 fix): the guest carries 8 rotten flesh and presses Submit on "Leftovers": the flesh leaves the **guest's** inventory, not the host's.
   - [ ] The guest leaves and rejoins: their progress is back (`progress/<guest uuid>.json`).
5. Report back: what worked, what didn't, and screenshots of the book! If every box is ticked, phase 2 is done and the next version is 0.1.0.
