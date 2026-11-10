---
tags: [project, questions]
updated: 2026-11-10
---
# Open questions

> Format: `- [ ] **Question**: context → lead`. Once resolved, check it off and note the answer with the date. If it leads to a decision, create an ADR.

## Decisions to make (Bastien)
> The code of 2026-11-10 (phase 1) took a default for each of these so it could move on. Each is cheap to change now, costly once packs and saves exist.

- [ ] **Quest text**: plain text in the quest files, or translation keys like FTB? **Default taken: plain text** (`title`, `subtitle`, `description` are strings in the chapter file). In-game text is English-only for now → [[Horizon Quests data model]]
- [ ] **A quest without tasks**: **default taken: it completes by itself as soon as its dependencies are met** (a milestone, or a welcome gift when it has no dependency). Alternative: refuse such quests at load
- [ ] **Item tasks that do not take the items**: **default taken: progress follows the inventory and goes back down** if the player drops the items, until the task reaches its goal; then it stays done. Alternative: progress only ever goes up
- [ ] **Default node shape**: **default taken: `square`** (what the spike draws with `native_quad`). FTB's default is a circle, which needs textures (V1 assets) or a polygon
- [ ] **Rewards**: **default taken: claimed by hand in the book** (`auto_claim_rewards: false`), with a per-reward `auto_claim` override. Alternative: auto-claim by default
- [ ] **Flexible progression**: **default taken: tasks count always**, even for a locked or hidden quest; the quest completes when its dependencies are met too. (The Tasks note said "if visible" before; a `secret` quest could then never progress.)
- [ ] **Chapter tally** (`3/5` in the chapter list): **default taken: every non-optional quest counts**, hidden ones too, which hints that hidden quests exist. Alternative: count only visible quests
- [ ] **Strict JSON** for quest files (no comments): fine for now? JSON5 is not in Horizon's lock
- [ ] **Inventory polling frequency & scaling (multiplayer)**: `detection_delay` defaults to 20 ticks (1s) in `Settings`. On servers with many online players, reading 41 slots across everyone on the same tick could cause spikes. Keep `detection_delay` configurable in pack settings / server TOML, and stagger player checks across ticks in V1?

## Technical unknowns (to check in the code or with Horizon)
### 🔴 Critical for the graphical book ([[ADR-008 Graphical client book first]]): phase 0 spike
- [ ] **Mod messages in solo**: do `native_net_send_to_server` / `native_net_from_client` work between the window game and the integrated server? ⚠ The code returns an error when the window game has no `remote` connection (`"the server carries no mod messages"`), and we don't know yet whether single player uses one → **spike 3** answers it (the book's footer shows the exact error)
- [ ] **Mod screen behaviour in game**: drawing, icons, drag, wheel, click → **spike 2**. (Signatures are known now: [[Horizon hooks and API]] § 0.)
- [ ] **2D clip/scissor** for the canvas: is there an undocumented helper (`gui_paint`?). For now the book culls by hand (`hq_canvas::overlaps`)

## Answered by reading the code (2026-11-10)
- [x] *Real signatures of the screen and drawing helpers?* → [[Horizon hooks and API]] § 0. Screen hooks take `&self`; the mod screen's `rect` is the whole window.
- [x] *A single client + server bundle: how to separate the two states?* → client state in a `thread_local!` (window thread), server state in statics (atomics / `Mutex`); every client hook checks `native_window()` or `native_mod_screen()`. See `mod/mixins/horizon_quests/hq_client_glue.rs` and `hq_server_glue.rs`.
- [x] *Do we need the typed trait for screen input?* → No: we use mixin injections on the `Game::native_*` targets (smp style, [[ADR-004 Mixin bundle style like smp]]), and every hook we need has a target.

- [ ] **Are item icons drawn in the same pass as fills?** `hq_book` covers what spills out of the canvas with opaque bands drawn after it. If `native_item_stack` icons are drawn in a later pass they would show over the bands, so `hq_book` only draws an icon that is wholly inside the canvas. Look at spike 2: pan a node under the header band and see whether its icon shows through
- [ ] **Quad winding**: does `native_quad` care which way round the corners go? `hq_book::shape_points` goes round like `hq_canvas::line_quad` (the spike's lines). If the spike's lines show, shapes will too
- [ ] **Command arguments with `/`**: full ids are `chapter/quest/task`. `hq_commands::USAGES` declares them as `text` (rest of the line) in case `word` refuses `/` → test `/quests check getting_started/welcome/ready` in phase 2

### Other
- [ ] **Dedicated server**: the public doc says "Not there yet", but `native-mods.md` documents `--server-output`. Is it usable today? → test later (solo/LAN comes first)
- [ ] **Exact signatures** of `native_hud`, `native_key`, `native_mod_key`… → read `crates/horizon-client/src/game/native_hooks.rs`
- [ ] **`native_destroy_block`**: how to get the id of the broken block?
- [ ] **`native_player_changed_dimension`**: the doc contradicts itself on solo (NM:2278 vs NM:1291) → test
- [ ] **Ranged and indirect kills (bow, crossbow, trident, potion, pets)**: `native_mob_killed` (API 40) only attributes direct melee blows in current testing. Can projectile damage sources be tracked back to the player, or should we wire `native_living_death(..., attacker, ...)` (API 392) once projectile owner resolution is verified?
- [ ] **Hooks ⏳ pending**: when will `player_inventory_changed`, `native_add_toast`, `native_title`, `native_boss_bar`, `native_has_permission` get a number? → watch the Nightlies, ask on Discord
- [ ] Is **`/loot give`** implemented by Horizon?
- [ ] **Command aliases** (`/q`): possible?
- [ ] **JSON5**: is a parser available in Horizon's `Cargo.lock`? (for the FTB import)

## Resolved
- [x] *Can Horizon load FTB Quests?* → **No**, no JVM or Java Mixins. It has to be rewritten (2026-11-10, [[ADR-001 Native Rust mod for Horizon]])
- [x] *Is there already a quest mod on Horizon?* → **No** (158 projects reviewed on 2026-11-10)
- [x] *Can a server mod show a GUI to vanilla clients?* → **Yes**, via chest menus (`native_open_menu`), like smp (2026-11-10)
- [x] *License?* → **MIT** (2026-11-10, [[ADR-009 MIT license and mod id horizon_quests]])
- [x] *Mod id and name?* → **`horizon_quests`**, "Horizon Quests" (2026-11-10, ADR-009)
- [x] *Language of in-game text?* → **English only** for now; translations later (2026-11-10)
- [x] *Priority target?* → **Solo/LAN first**, code kept ready for dedicated servers (2026-11-10)
- [x] *Teams?* → **Built-in system in V1**; the MVP has per-player progress (2026-11-10)
- [x] *Inventory full on claim?* → **Drop the rest at the player's feet** (2026-11-10, [[Rewards - implementation]])
- [x] *UI order?* → **Graphical book directly**, no chest-menu book (2026-11-10, [[ADR-008 Graphical client book first]])
- [x] *ADRs 002 to 006?* → **All accepted** (2026-11-10, ADR-002's UI part superseded by ADR-008)
- [x] *FTB Quests import?* → **Yes, later** (phase 5); keep our format convertible (2026-11-10)
- [x] *Example chapter?* → **Yes**, a "Getting started" chapter that a config option can turn off (2026-11-10)
- [x] *Audience?* → **Published for everyone** on playhorizon.dev (2026-11-10)

## Found in the first in-game test (2026-11-10)
- ❓ Other mods' HUD (the minimap) draws over our mod screen, top right. Can a mod screen hide other HUDs, or must the book keep that corner free? Check in phase 2 with the real book's quest panel.
