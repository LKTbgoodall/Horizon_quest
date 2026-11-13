---
tags: [home, moc]
updated: 2026-11-13
---
# 🧭 Horizon Quests: home

> **An FTB Quests-style quest mod for Horizon** (Minecraft Java 26.3 rewritten in Rust).
> This vault is the project's "second brain". Everything that matters is here, so nobody has to reread old conversations.

## 📍 Where we are
| | |
|---|---|
| **Phase** | Phase 0 ✅, phase 1 ✅, **phase 2 ✅ (2026-11-10): release 0.1.0** (private testing), everything tested in game in solo and LAN. **Phase 3 in progress**: its new tasks, rewards, reward tables, repeatable quests and tasks in order are tested too → [[Roadmap]] |
| **Last session** | [[2026-11-13]]: **v0.18.1**: arrows in the quest window (what it needs / opens, as in FTB Quests; hidden quests left out, not even counted), replacing the "Needs:" lines. Before: **re-pinned on `nightly-2026-11-13-9f42064`**, the first Nightly with native `.hmod` content (launcher 0.2.63); the glue (v0.17.1) type-checks against it with no warning, mixins unchanged, hooks only added. Before: [[2026-11-12]]: **v0.16.0** (Bastien's 7 points on 0.15.1: links by right-dragging, picked from a preview page; drawn rewards with a table of their own edited from their panel; a new quest stays open; XP defaults; `@mod` search; advancement picker). Before: **v0.15.1** (Bastien's 9 points: links by dragging a yellow dot, numbers typed, "Up to" fixed, "Given at once", one Experience reward...) after **editing in place (v0.15.0)** after Bastien found the forms horrible: the quest window is the editor, item / mob picker, panels with visual controls ([[ADR-017 Edit in place]]). Before: **the in-game editor (v0.11.0 to v0.14.0)**: v0.14.0 chapters, groups, reward tables and pack settings; v0.13.0 tasks and rewards (click to edit, green "+"), one generic form engine; v0.12.0 **the quest form**: "Edit..." on a quest opens a form (title, subtitle, description, icon with item suggestions, shape, size, visibility, needs, flags, cooldown) with text fields of our own (`hq_text`, `native_screen_char_typed_pre`). Before it, **edit mode, groundwork (v0.11.0)**: a pencil for operators turns on edit mode in the book: drag quests, right-click for new / duplicate / delete / link picking; the server checks, saves the world's own pack and sends it to every book; "Save for the modpack" (`config/horizon_quests/quests/`, used by worlds without their own) and "Use the modpack's quests" ([[ADR-016 Edit mode in the book]]). Glue type-checked on Linux (`tools/check-glue.sh`). Before: [[2026-11-11]]: **how to publish the content found** ([[ADR-014 Publish one hmod with content and code]]: one combined `.hmod`, supported by Horizon `main`; re-pin on the newest Nightly first). **v0.7.3** (check on the corner, Escape closes the quest window first, windows over the quest window, a client reset bug fixed, last read done), **getting ready for V1**: code cleaned for review (no dead-code allowances, no warning in the game build), release documents (`README.md`, `docs/making-quests.md`, `CHANGELOG.md`, `release/`). Before: **thicker checks, levels as numbers, a window to choose a reward** (v0.7.2), **marks, counts, item tooltips, pin** (v0.7.1), **the quest window** (v0.7.0: a window over the canvas instead of the side panel, tasks and rewards as clickable icons, description under them), **hover and selection** (v0.6.2: white viewfinder corners, hovered node a little bigger, title tooltip in a violet-framed box), **dependency lines** like FTB's (v0.6.1: chevron bands, cyan for what the hovered quest needs, yellow for what it opens), after **assets** (v0.6.0: node frame textures, registry icon `mod/icon.png`). Before: Bastien's **solo and LAN tests of v0.2.0 to v0.5.1 passed** (server settings not tried); the hover hiding an item row's icon fixed in **v0.5.1**, tagged `v0.5.1`. Before: [[2026-11-10]] § Session 17: **book polish** (v0.5.0: tracked quest top-left, party name in the header, recipe view, team rewards), **quests from data packs** (v0.4.0, `data/<ns>/horizon_quests/`, example in `examples/quest-datapack/`), **settings** (v0.3.0: parties on/off, party size, detection delay, completion messages, a client file for toasts) after **parties** (v0.2.0): `/quests team create / invite / join / leave / kick`, one shared progress per party with FTB's copy / merge / keep-claims rules ([[ADR-012 Parties share one progress]]); 154 + 4 tests pass, glue type-checks. To test in LAN. § Session 16: Quest Book texture redrawn at 16 x 16 (hand-placed lying book, small gold sun). Before that, § Session 14: in-game tests of v0.0.6 to v0.0.9, solo then LAN (Windows host under an offline name, Arch Linux guest). Fixed on the way: XP and advancement tasks for the single-player host (v0.0.7, selectors through `native_run_command`), short ids for `/quests check` / `submit` (v0.0.7), probe at most once a second (v0.0.8), the Nether toast lost during the dimension load (v0.0.9). New `tools/build-mod.sh` for Linux. **0.1.0 released** (tag `v0.1.0`). 137 + 2 tests pass |
| **Code** | `mod/` **v0.18.1** (needs / opens arrows in the quest window, hidden quests not counted; v0.17.1 chance steps 0.1 to 5; v0.17.0 hidden quests shown in edit mode, mark tasks and rewards done; v0.16.2 chances set in the list, Even, tables renamed; v0.16.1 chances in percent, table menu, tables pruned; v0.16.0 right-drag links, inline reward tables, advancement picker, to test in game; v0.15.1 links by dragging, panel fixes; v0.15.0 editing in place, to test in game; v0.14.0 chapters, groups, tables, settings in edit mode; v0.13.0 tasks and rewards; v0.12.0 quest form, all to test in game; v0.11.0 edit mode groundwork, to test in game; v0.10.7 choice window opaque; v0.10.6 a hand-in that works says nothing; v0.10.5 a wordier hand-in message; v0.10.4 chapter list: red "!" for rewards to claim, names grey / blue / green for not started / started / done; v0.10.3 lines into locked quests dimmed and half see-through; v0.10.2 seen working in game; v0.10.1 drew no lines: GUI textures over 1024 x 512 are left out by the game, the sheet is now 512 x 288; lines picked by Bastien from a preview page: thin chevrons with beads between them, sliding in focus; v0.10.0 lines without chevrons: thin dashed threads, solid green when done, glowing with comets in focus; v0.9.5 chevrons inside the line; v0.9.4 thin lines with smooth chevron sprites, flowing in focus; v0.9.3 thick bands, rejected; v0.9.2 focused lines animated; v0.9.1 lines green only when both ends are done; v0.9.0 vanilla progression pack of 81 quests to the end of the game, bigger node icons, see-through list; v0.8.11 node borders half as thick; v0.8.10 no top band, all in the bottom band; wider strip; selection raised instead of corners; v0.8.9 chapter list on hover from a side strip, restyled; v0.8.8 foldable chapter groups, example pack grouped; v0.8.7 octagon gone, blue column gone, smaller node icons; v0.8.6 rewards waiting: done green with a red "!" badge; v0.8.5 diamond ring thinner, hexagon bigger; v0.8.4 non-square shapes drawn bigger with a thicker smooth ring and per-shape icon sizes; v0.8.3 every node shape in the example chapters; v0.8.2 the locked veil covers the frame face in its shape; v0.8.1 a claimed random or choice reward shows only what it gave, padlocks scale with the zoom; v0.8.0 item count ranges "3-5", table rewards cycling their items with plain-list tooltips, smooth signs from a texture; v0.7.7 bigger count digits; v0.7.6 smooth digits from a texture for the counts; v0.7.5 finer counts with a dark edge, smoother checkmark button that turns green; v0.7.4 locked quests greyed with a padlock, checkmark tasks as a round button, small counts in the corner; v0.7.3 in-game fixes and last read; v0.7.2 choice window, thicker checks; v0.7.1 marks, inventory counts, item tooltips, pin; v0.7.0 the quest window; v0.6.2 selection corners and hover; v0.6.1 chevron lines with hover colours, to see in game; v0.6.0 node frame textures, seen in game); v0.5.1 tested in game solo and LAN (tag `v0.5.1`): v0.5.0 book polish; v0.4.0 quests from data packs; v0.3.0 settings; v0.2.0 parties with a shared progress, `teams.json`. Since 0.1.1/0.1.2: no HUD hint, Quest Book item with its 16 x 16 Horizon-style texture, one `horizon_quests.hmod` for code and content). Full client/server quest engine with real graphical book, net sync over `horizon_quests:book` (protocol 2), inventory polling, 9 task types, 7 reward types, reward tables, repeatable quests, tasks in order, HUD toasts, saving, `/quests`. Two example chapters |
| **Key choices** | `horizon_quests`, MIT, graphical book first, solo/LAN first, English-only in-game text, published for everyone |
| **Blocking** | Nothing |
| **In progress** | The in-game editor: all 4 steps coded (v0.11.0 to v0.14.0), waiting for the in-game test → [[Roadmap]] phase 5. Every phase 3 step is coded; v0.6.0 (assets) waits for a look in game, then 1.0.0, the public release |

## ➡️ Next step
- **"Next level" look**: Bastien picks among the six concepts of `vault/04 - Design/previews/next-level-concepts.html` (3D board, constellation, block island, quest dioramas, completion wave, quests in the world); then an ADR and the code. See [[2026-11-13]].
- **Bastien tests v0.18.1** (`mod/README.md` § "New in 0.18.0"): the arrows of the quest window. Build with `tools/build-mod.sh` (two halves: the textures; `--combined` loses them until horizon-rust/horizon#15933 is fixed).
- **New Nightly `9f42064` (2026-11-13)**: Bastien runs `tools\build-mod.ps1 -Combined` and checks that the textures come from the one `.hmod`; that unblocks 1.0.0 (`release/v1-checklist.md` § 4).
- **Edit mode**: Bastien tests v0.15.1 and v0.15.0 first (`mod/README.md` § "New in 0.15.1" and "New in 0.15.0"), then the rest of 0.11.0 to 0.14.0 (moving, links, chapters, tables, modpack; does the pencil show in solo with cheats off?). Then fixes, and ideas: undo, reorder tasks and rewards, move a quest to another chapter.

0. Claude, before any in-game test request: `powershell -ExecutionPolicy Bypass -File tools\check-glue.ps1` (Linux: `tools/check-glue.sh`) must end with "The glue type-checks" ([[Build, test and tools]])
1. **V1**: follow `release/v1-checklist.md`. Bastien: the in-game test of 0.6.0 to 0.7.3 and the server settings, 1.0.0 waited for the Nightly that ships native `.hmod` content support: **out on 2026-11-13 (`9f42064`), pins updated**; earlier notes: (Bastien confirmed after the `-Combined` test, which built the code but installed no textures; our pin `fdc87ae` is still the newest Nightly): when it is out (`horizon-launcher versions`), Claude re-checks the glue, updates the pins, and Bastien rebuilds with `-Combined` to see the textures. The content half ships in the one `.hmod` ([[ADR-014 Publish one hmod with content and code]]). Then **1.0.0**: the public release on playhorizon.dev ([[Publishing and mod policy]]). Later: JEI's recipes when installed, a party tab outside the book
2. Small follow-ups Bastien put aside: short ids and a list for `/quests complete` / `reset`, the quest id in the book panel for operators

## 🗺️ Vault map
### 01 · Project
- [[Vision and goals]]: why, what, non-goals
- [[Roadmap]]: phases 0 to 5 with checkboxes
- [[Open questions]]: things to decide or check
- [[Glossary]]

### 02 · Horizon (the platform)
- [[Horizon - Overview]]: what Horizon is, mod types, client and server
- [[Native mods - structure and manifest]]: `horizon-mixin.toml`, `compatibility.json`, allowed crates
- [[Horizon hooks and API]]: ⭐ **the API reference** (networking, events, rewards, persistence, GUI, commands)
- [[Example - smp mod (patterns to reuse)]]: ⭐ **the model to copy**
- [[Build, test and tools]]: launcher, `native.py`, tests, Windows pitfalls
- [[Publishing and mod policy]]
- [[Links and resources]]
- [[Fork - arismoko mod API]]: the mod API fork (not official): what it has, what a port would cost

### 03 · FTB Quests reference (the functional model)
- [[FTB Quests - Overview]]
- [[FTB data model]] · [[FTB task types]] · [[FTB reward types]]
- [[FTB teams and progress]] · [[FTB editor and UI]] · [[FTB file format]] · [[FTB commands]]

### 04 · Design (our mod)
- [[Architecture]]: ⭐ events → pure rules → effects
- [[Horizon Quests data model]]
- [[Tasks - hook mapping]] · [[Rewards - implementation]]
- [[UI - Quest book]]: the graphical book (MVP)
- [[Persistence and sync]] · [[Quest file format]] · [[Commands and permissions]]

### 05 · Decisions
- [[Decision index]]

### 06 · Journal
- One note per session, named `YYYY-MM-DD` (template: `99 - Templates/Template - Journal`)

## 🔁 Session routine
**Start**: read this page → the latest journal note → the current phase in the [[Roadmap]].
**End**:
1. Create or update today's note in `06 - Journal`.
2. Tick off the [[Roadmap]].
3. Update "Where we are" and "Next step" above.
4. Add questions to [[Open questions]] and record decisions in an ADR.
5. If something about Horizon changed, update the reference note and its `verified_on`.

## 📏 Conventions
- Every project file is written in **English**: notes, code, comments, commits. (Chat with Bastien is in French.)
- Frontmatter: `tags`, `status`, `date` / `verified_on`, and `source` for reference notes.
- API status: ✅ available · ⏳ pending (not numbered yet) · ❌ does not exist · ❓ unknown.
- Priorities: 🔴 MVP · 🟠 V1 · 🟡 V2 · ⚪ later.
- Quick notes go in `00 - Inbox`, to be sorted later.
