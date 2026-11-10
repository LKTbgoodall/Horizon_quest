# Horizon Quests: the native mod

The bundle the Horizon launcher builds into the game. Version 0.0.3 added the **graphical quest book** (`hq_book`) and **network synchronization** (`hq_net`) to the quest engine (`hq_session`, `hq_server_glue`). Version 0.0.4 fixes what the review of 0.0.3 found: the build (dead spike code), claims never shown as claimed in the book, and book hand-ins that took the host's items; it also removes the spike's ping/pong. Version 0.0.5 adds the server config file, whose `example_chapter = false` turns the built-in example chapters off. Version 0.0.6 starts phase 3: new tasks (craft, experience levels, dimension, location, advancement), new rewards (command, toast, random and choice from **reward tables**), **repeatable** quests and tasks done **in order**, with a second example chapter, "Further afield". It also fixes the build: 0.0.4 and 0.0.5 called a private engine method (`native_as_player`) and did not compile. Version 0.0.7 fixes what the first in-game test of 0.0.6 found: experience-level and advancement tasks never moved for the single-player host (Horizon tells neither for the host, so the server now asks vanilla's `level=` and `advancements=` selectors), and `/quests check` / `/quests submit` now work without an argument (they list the open tasks, or act on the only one) and take a quest's short id. Version 0.0.8 asks a player's level and advancements at most once per `detection_delay` (one second by default), even while item pickups make the server look at the inventory more often. Version 0.0.9 fixes the toast that never showed: it arrived while the client was loading the next dimension, when the mod forgot its state, toasts included. **Version 0.1.0** is the first private release: everything above passed the in-game test in solo and LAN (2026-11-10). Version 0.1.1 removes the HUD hint in the top-left corner and adds the **Quest Book** item (`horizon_quests:quest_book`): every player gets one on their first join (`quest_book = true` in the server config), `/quests book` gives another, and a right-click with it opens the book as the key does. The key is changed in Options > Mods > Horizon Quests (Horizon does not let a mod add rows to Controls > Key Binds yet). Version 0.1.2 gives the Quest Book its own 16 x 16 texture in Horizon's colours (`mod/content`, drawn by `tools/make-book-texture.py`) and packs code and content as one `horizon_quests.hmod`. **Version 0.2.0 adds parties** (`hq_teams`): `/quests team create <name>` starts a party whose progress is shared by its members (the creator's progress so far becomes the party's), `/quests team invite <player>` and `/quests team join <name>` bring players in (what the joiner had done is added to the party's progress), `/quests team leave` gives a player their own progress back with the rewards they claimed kept, and the owner can `/quests team kick <player>`. A task done by any member counts for the whole party; an item task counts the member who holds the most, an experience task the highest level; rewards are still claimed by each player for themselves. Parties are kept in `<world>/horizon-quests/teams.json`, a party's progress in `progress/<party id>.json`.

| Feature | What it provides | Hooks / Modules |
|---|---|---|
| **Graphical Book** | Interactive quest canvas: chapter sidebar, zoom, pan, quest details panel, buttons | `hq_book`, `hq_canvas`, `hq_client_glue`, `native_mod_screen_open`, `native_screen_render_post`, input hooks |
| **Network Sync** | Reliable framing (<=30KB frames, 16 frames/tick) for definitions, progress, actions | `hq_net`, channel `horizon_quests:book`, `native_net_send_*`, `native_net_from_*` |
| **Server Engine** | Tasks (item, hand-in, kill, checkmark, craft, experience level, dimension, location, advancement), rewards (item, XP, levels, command, toast, random, choice), repeatable quests, atomic JSON saves, `/quests` | `hq_server_glue`, `hq_session`, `hq_progress`, `hq_store` |

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
    └── data/quests/                               # the example pack: chapters/ and reward_tables/
```

The game builds every module with `-D warnings`, and there the modules are private: code that no glue calls is dead code and breaks the build, even when `cargo test` is clean. Remove what the glue stops calling.

## Unit tests (any PC with Rust)
```bash
cd mod
cargo test
```

## Type-check the glue (a PC whose launcher has built Horizon once)
`cargo test` never compiles the glue: it needs the game. This does, without building or running the game (no `--trust-native-code`): it stages the launcher's copy of the pinned Horizon source with the mod applied (`horizon-mixins apply`) and runs `cargo check` on it.
```powershell
powershell -ExecutionPolicy Bypass -File toolseck-glue.ps1
```
It ends with "The glue type-checks against Horizon fdc87ae", or with the compiler's errors. Run it before asking for an in-game test.

## Test in game
1. Get the latest code (on another PC):
   ```bash
   git pull
   ```
2. Package, install and build it. Run this yourself from the repo root; an agent must never run the build step, which passes `--trust-native-code`. Add `-Instance NAME` if your instance is not `mon-horizon`.
   ```powershell
   powershell -ExecutionPolicy Bypass -File tools\build-mod.ps1
   ```
   The script packs **`build/horizon_quests.hmod`, the one file of the mod**: its native code (`horizon-native.json`, `mixins/horizon_quests/`) and its content (`horizon-mod.toml`, `assets/` from `mod/content`: the Quest Book's model and texture), Horizon's "one `.hmod` for content and code" format. Every entry has a fixed date, so the same sources give the same file. The launcher of the pinned Horizon still builds native code only from `horizon-native-mods/<id>.hnmod`, so to install, the script cuts the file's two halves (`build/install/`): the code goes to the instance's `horizon-native-mods`, the content to its `horizon-mods`. It turns the mod on in `horizon-mods.txt` and runs `horizon-launcher native-build --instance mon-horizon` with every native mod of the instance. `-NoBuild` stops before the build; `-Uninstall` takes the mod out.

   **On Linux (or macOS)**, the same with bash:
   ```bash
   bash tools/build-mod.sh
   ```
   Without options it installs into the default instance, which on Linux is the game home `~/.local/share/horizon` itself (it holds `horizon-native-mods`), and builds the launcher's selected instance. For another instance add `--instance NAME` (looked for under `~/.config/horizon/instances/` and `~/.local/share/horizon/instances/`) or `--instance-dir DIR`. The launcher is found on the `PATH`, else through its applications-menu entry `~/.local/share/applications/horizon-launcher.desktop`; else set `HORIZON_LAUNCHER=/path/to/horizon-launcher`. `--no-build` and `--uninstall` work as on Windows. It needs `bsdtar` (always there on Arch), `zip` or `python3`.

   Do **not** use `horizon-launcher native-build mod/mixins/horizon_quests` alone: the mod is then missing from the launcher's list, and the next build the launcher starts (its "Compile" button) drops it.

   For faster rebuilds while testing, you can first run `horizon-launcher settings native-profile dev`, and later `horizon-launcher settings native-profile performance`.
3. Start the game and check, in order. The Options > Mods row now says **0.2.0**.
   Use a **new single-player world** in Survival:
   - [ ] **Build**: it succeeds. If not, send `native/build.log`.
   - [ ] **Open the Book (K)**: opens the graphical quest book. You should see the chapters on the left sidebar ("Getting started" and "Further afield"), the canvas with quest nodes, dependency lines, icons. (Since 0.1.1 there is no HUD hint in the top-left corner.)
   - [ ] **Pan & Zoom**: drag to pan the canvas, mouse wheel to zoom in and out.
   - [ ] **Quest selection**: clicking a quest opens its details panel on the right with description, tasks, and reward previews.
   - [ ] **Checkmark task**: select the "Welcome" quest and click the confirmation button. It completes, in the book too.
   - [ ] **Inventory task**: gather 8 logs. The quest completes and the book shows it without reopening. (This also tells us whether the host's inventory is read in single player.)
   - [ ] **Claim rewards**: click the claim button in the quest panel. Items and XP arrive, **the button goes away and the quest shows as done** (fixed in 0.0.4). Leftovers drop at your feet if the inventory is full.
   - [ ] **Hand in from the book**: the optional "Leftovers" quest (after "First night") takes 8 rotten flesh. Press Submit in its panel: the flesh leaves your inventory and the task counts it (in single player you are the host: the items are taken from the state in place).
   - [ ] **Saving**: leave the world and rejoin: pressing K shows your progress again, read from `saves/<world>/horizon-quests/progress/<your uuid>.json`.
   - [ ] **Quest Book (new in 0.1.1)**: on joining the world, chat says "Here is your Quest Book" and a book named **Quest Book** (since 0.1.2 a 16 x 16 violet book lying tilted, pages along its lower edge and a small round gold sun on its cover, with a dark violet outline like vanilla items; purple name, shining, "Right-click to open your quests" under it) is in your inventory. A right-click with it in hand opens the quest book. It does not stack and cannot be enchanted. Leave and rejoin: no second book. `/quests book` gives another. `quest_book = false` in the server config stops the first one.
   - [ ] **Key**: Options > Mods > Horizon Quests: change "Open the quest book" to another key; the new key opens the book.
   - [ ] **`/quests` commands**: `/quests` (first line: "Horizon Quests 0.2.0 is running on this server. Hello <you>!"), `progress`, `check`, `claim`, `submit`, `reload`, `validate`. **New in 0.0.7**: `/quests check` alone lists the checkmark tasks you can tick (or ticks the only one); `/quests check welcome` (a quest's id) is enough. Same for `/quests submit` with hand-in tasks.
   - [ ] **Optional, a server without the mod**: on a vanilla server, K opens the book and after about 5 seconds it says "No answer from the server: it may not have Horizon Quests."
   - [ ] **Config file** (new in 0.0.5): after opening a world, `config/horizon_quests-server.toml` exists in the instance's `config/` folder (beside `options.txt`), with `example_chapter = true`.
   - [ ] **Example chapter off**: with the world open, set `example_chapter = false` and save. Within a second or two the log says the file changed, and the book (press K again) shows "This world has no quests." Set it back to `true` and save: "Getting started" comes back, with your progress intact.
   - [ ] **A world's own copy**: copy the file to `saves/<world>/serverconfig/horizon_quests-server.toml` with `example_chapter = false`; that world alone has no example chapter (reopen the world).
   - [ ] **New in 0.0.6, chapter "Further afield"** (its quests open as you advance in "Getting started"):
     - [ ] **Craft, repeatable, random reward** ("Daily bread", after "A place to work"): craft 3 bread; the count goes up as you take each loaf from the grid. Its panel says "Repeatable" and lists what the reward can be. Claim it: one entry of the list arrives. Five minutes after the completion (and once claimed), chat says "You can do this quest again" and the quest is open again, "Repeatable, done once".
     - [ ] **Tasks in order** ("Armed and ready", after "Into the rock"): its panel says "Tasks, in order". Kill a skeleton first: it does not count. Craft a stone sword, then two skeletons count.
     - [ ] **Advancement** ("Stone age", after "First pickaxe"): it completes when you earn "Stone Age". If you already have it, it completes by itself within a second of the quest opening (0.0.7: the server asks vanilla's `advancements=` selector, so this also covers advancements earned before the mod was installed).
     - [ ] **Experience level, choice reward** ("Seasoned", after "Armed and ready" or "Stone age"): reach level 5 (`/xp add @s 5 levels` with cheats). **Fixed in 0.0.7**: the task follows your level within a second, even as the host of a single-player world. Its reward has one "Pick" button per entry: pick one, that one arrives, the others go away. `/quests claim` skips it and says to pick in the book.
     - [ ] **Location** ("The middle of the map", optional): stand within 16 blocks of X 0, Z 0 in the Overworld (`/tp  0 100 0`); it completes within a second. **This tells us whether the single-player host's position is read.**
     - [ ] **Dimension, toast and command rewards** ("Somewhere warmer", optional): enter the Nether. A box "Welcome to the Nether / Mind the lava." shows in the top-right corner for 5 seconds (if you never opened the book this session, it is a chat line instead), and you get one minute of fire resistance.
     - [ ] **Furnace**: smelting counts for craft tasks too. No example quest asks for it; skip unless you write one.
4. **LAN test** (needs a second Minecraft account on another PC, with Horizon and this mod):
   - [ ] On the home PC, open the world to LAN; the guest joins.
   - [ ] The guest presses K: the book opens with **their own** progress (not the host's).
   - [ ] The guest completes "Welcome" (checkmark) and claims its reward: the reward goes to the guest, and the host's book does not change.
   - [ ] **Hand-in as a guest** (rewritten in 0.0.6): the guest carries 8 rotten flesh and presses Submit on "Leftovers": the flesh leaves the **guest's** inventory, not the host's. (The server reads the guest's inventory, then runs vanilla's `/clear` for them.)
   - [ ] **Toast as a guest**: the guest enters the Nether: the toast shows on the guest's screen only.
   - [ ] The guest leaves and rejoins: their progress is back (`progress/<guest uuid>.json`).
   - [ ] **Parties (new in 0.2.0)**, still with the guest on:
     - [ ] The host types `/quests team`: "You play on your own progress...". Then `/quests team create Miners`: "You started the party Miners...". `/quests team` now lists "Party Miners: <host> (owner)."
     - [ ] The host types `/quests team invite <guest>`: the guest's chat says "<host> invited you to the party Miners: /quests team join Miners". The guest types it: both hear that the guest joined; `/quests team` lists both.
     - [ ] **Merged progress**: both books (press K again) show every quest either of you had done before.
     - [ ] **Shared tasks**: the guest kills a zombie for "First night" (or ticks a checkmark): it moves in the **host's** book too, and when the quest completes both of you get "Quest completed".
     - [ ] **Rewards stay per player**: the host claims a reward; the guest's book still shows it to claim, and the guest gets it too when they claim.
     - [ ] **Items**: for an item task, the one of you who holds the most counts (the host holds 5 logs and the guest 3: the task shows 5).
     - [ ] The guest leaves the game and comes back: still in the party, on the shared progress. In `saves/<world>/horizon-quests/` there is a `teams.json` and a `progress/party-....json`.
     - [ ] The guest types `/quests team leave`: they are back on their own progress as it was before joining, and the rewards they claimed in the party show as claimed. The host is told; `/quests team` lists only the host.
     - [ ] Invite the guest again, join, then the host types `/quests team kick <guest>`: the guest is told and is back on their own progress.
     - [ ] Wrong uses answer a sentence, not an error: `/quests team join Nobody`, `/quests team invite <yourself>`, `/quests team create` (usage line), a second `/quests team create Other` while in a party.
5. Report back: what worked, what didn't, and screenshots of the book!
