---
tags: [design, ui]
status: draft
date: 2026-11-10
updated: 2026-11-13
---
# UI: the quest book

Reference: [[FTB editor and UI]] · API: [[Horizon hooks and API]] §5 · Decision: [[ADR-008 Graphical client book first]]

> [!important] Decision (2026-11-10)
> The MVP ships the **graphical book** directly (Horizon client with the mod). The chest-menu book is **not planned**; it stays described at the end of this note as an optional fallback for vanilla clients.

> [!important] Coded, pure, in `hq_book.rs` (2026-11-10): not seen in game yet
> `BookView::scene(&Screen)` returns a **draw list** (`Fill`, `Quad`, `Text`, `Item`) and **click zones**; `click`, `drag` and `scroll` handle input. The glue will only carry the draws out. What it does, as built:
> - **Regions since v0.19.0**: no band at the top or the bottom. The left strip of 18 px (the closed chapter list), the canvas as tall as the window, and a **rail of 18 px on the right** (`Regions::rail`, `canvas::RAIL`): close, pencil, "?" (help as a tooltip), the party's sign, "claim all" with its count, then the chapter's progress as a bar filling from the bottom (`draw_rail`; `Action::Tip` for the parts that only tell). The chapter's name and tally are a tag at the canvas's top left, the notice and the mode's help tags at its bottom left (`draw_tags`, `tag`). Slides: `Anim` (set in `hover`, once a frame) remembers when the quest window, the chapter list, a menu and an edit panel appeared; `scene` draws them in place then moves the draws back along their way (`shift`, `arrived`, 4 ticks eased); zones never move. Shared looks: `raised`, `sunken`, `rounded`, `cross`, `tone` (used by `hq_inline` too).
> - Regions (v0.8.10 to v0.18.1, history): no top band; a strip of 18 px on the left (the closed chapter list, opened on hover as a panel of 30 % of the width, 110 to 170 px, over the canvas), the canvas, the quest window over it, and one bottom band of 14 px (chapter name and tally, "Claim all (n)", the last notice, the party, help, close).
> - Chapter list: ungrouped chapters, then each group with its chapters; icon, title, `done/total`; hidden chapters are not listed.
> - Canvas: dependency lines (grey, green once the dependency is completed; only inside the chapter), nodes as fans of quads for the 5 shapes, frame in the state's colour (locked grey, available white, in progress yellow, rewards waiting blue, done green), gold ring when selected, item icon, tooltip with the title on hover. Opening a chapter fits it to the canvas (never zoomed in past 1).
> - Panel: title bar with a close button; subtitle, state, "Needs:" for a locked quest, description (wrapped), tasks with `[x]` marks, counts and **Check** / **Submit** buttons, rewards with **Claim** buttons or "(claimed)". Scrolls with the wheel. Opening it does not move the quests on screen.
> - No clip: the canvas is drawn first and the opaque bands cover what spills out; icons and text are only drawn where they fit whole.
> - Names of items and entities are made from their ids (`minecraft:oak_log` → "Oak log") until the book can ask the game for real names.

## Previews (drawn without the game)
Made by `cargo run --example book_preview` in `mod/` (`mod/examples/book_preview.rs`): it runs `hq_book` on a few situations and writes each draw list as an SVG in `previews/`. **Real**: every position, size, colour, text and button. **Stand-ins**: the font, and the item icons (boxes with initials; the game draws the real icons). Run it again after any change to `hq_book`.

A new player opens the book:
![[book-1-first-open.svg]]

A click on a quest opens its panel:
![[book-2-quest-panel.svg]]

Later (green done, blue rewards waiting, yellow in progress, white open, grey locked):
![[book-3-in-progress.svg]]

A hand-in task, the hidden last quest revealed, a refusal in the footer:
![[book-4-hand-in.svg]]

Another pack: chapter groups, the five shapes, sizes:
![[book-5-shapes-and-groups.svg]]

Edit mode (v0.11.0) with a quest's menu:
![[book-8-edit-menu.svg]]

The quest form (v0.12.0):
![[book-9-title-typed.svg]]
![[book-10-quest-settings.svg]]
![[book-11-add-task.svg]]
![[book-12-item-picker.svg]]
![[book-13-task-panel.svg]]

## Layout (MVP)
```
┌────────────┬─────────────────────────────────────────────┬──────────────────┐
│ Chapters   │            CANVAS of the chapter            │ Quest panel      │
│ ▸ Getting  │                                             │ (on click)       │
│   started  │     (◯)────►(◯)────►(◇)                     │ [icon] Title     │
│ ▸ Nether   │       └────►(◯)                             │ subtitle         │
│ ▸ …        │                                             │ description…     │
│            │  drag = pan · wheel = zoom · click = select │ Tasks   [⛏ 3/5]  │
│            │                                             │ Rewards [💎][XP] │
│            │                                             │ [Submit][Claim]  │
├────────────┴─────────────────────────────────────────────┴──────────────────┤
│ [Claim all]                                       progress 12/40     [✖]    │
└──────────────────────────────────────────────────────────────────────────────┘
```
- Quest states by colour and icon: locked (grey + 🔒), visible/startable (white), in progress (yellow), completed (green), rewards waiting (pulsing).
- Dependency lines: grey while locked, coloured once the dependency is done.

## Rendering and input
- **Screen**: `native_mod_screen_open(mod_id)` / `native_mod_screen_close`, drawn in `native_screen_render_post` (mouse position in GUI pixels, partial tick). Escape closes it.
- **Canvas transform** (pure, tested in phase 1): `screen = (world - pan) * zoom + origin`, with zoom clamped and centred on the cursor.
- Input:
  - pan: `native_screen_mouse_dragged_pre(screen, button, delta, modifiers, canceled)`;
  - zoom: `native_screen_mouse_scrolled_pre`;
  - click: `native_screen_mouse_button_pressed_pre` / `_released_pre`;
  - keys: `native_screen_key_pressed_pre`, text later via `native_screen_char_typed_pre`.
  - ⚠ Several of these are **not in the typed trait yet**: bind them as mixin helpers (see [[Open questions]]).
- Drawing:
  - shapes: `native_sprite` (textures in `assets/horizon_quests/textures/gui/shapes/`, needs the combined `.hmod`) or `native_quad` until assets exist;
  - icons: `native_item_stack(batch, stack, at, size)`;
  - dependency lines: `native_quad` (an oriented quad gives thickness);
  - text: `native_text`, `native_text_width` for wrapping;
  - panels and buttons: `native_fill`, nine-slice (API 411) when documented.
- ⚠ **No documented 2D clip/scissor**: cull what is outside the canvas area ourselves (skip quests whose box is outside, clamp lines).
- **Homemade widgets** (pure layout + hit testing, testable): button, scroll list for chapters and descriptions, tooltip.
- Clicking an item task: `native_show_recipes(stack, lookup)` opens JEI/REI if one is installed (V1).

## Data flow
- The server half sends definitions and progress through `hq_net` (chunked under 30,000 bytes; see [[Persistence and sync]]).
- Buttons send requests (`Claim`, `Submit`, `Check`) that the server **re-validates** before doing anything.
- The book only shows what the server says; it never decides progress itself.

## Opening
- A rebindable key (`native_mod_key`, default to pick, e.g. `key.keyboard.l`... check for conflicts).
- A client command (`native_command`), e.g. `/quests`.
- Later: a quest book item (`native_claim_items`) and a button on the inventory screen.

## Notifications (MVP)
- Chat message and sound on quest completion. Client toasts (`native_add_toast`, ⏳) when available.

## In-game editor (edit mode, since v0.11.0)
> **Since v0.15.0 the forms below are replaced by editing in place** ([[ADR-017 Edit in place]]): the quest window is the editor (title / subtitle / description typed in place, `Typed`, `edit_lines`, `Caret`), the icon opens `hq_inline::Picker`, the gear and a click on a task or reward open an `hq_inline::panel` beside the window (`BookView.panel: Option<Subject>`, `picker`, `typing`); "+" opens a menu of types with icons (`Menu.icons`). What follows about `hq_form` is history.

→ [[ADR-016 Edit mode in the book]]
- **Pencil** (`pencil_button`, pixel art `PENCIL_PIXELS`) left of the "x" in the bottom band, only when the server said `ToClient::Editor { allowed: true }`. It toggles `BookView.editing`: orange band (`EDIT_BAND`), "Editing (<source>)" on the left, a dot per grid cell (`draw_grid`), help "right-click: menu | drag a quest: move it".
- **Drag a quest**: a press on a node starts `Moving` (no selection yet); `drag` moves it (`place_of` draws it on its snapped half cell); `release` (hook `native_screen_mouse_button_released_pre`, API 80) sends `Edit::MoveQuest`, or opens the quest when it did not move (2 px).
- **Right click** (`right_click`, button 1 of the pressed hook) opens a `Menu` (`draw_menu`: a dim title and rows, a click elsewhere closes it, Escape too):
  - empty spot: New quest here, Save these quests for the modpack, Use the modpack's quests (when the world has its own: asks first);
  - quest: Duplicate (one cell right), Pick what this quest needs... / opens... (link picking: `linking`, each quest clicked toggles the link, Escape or a click elsewhere ends it), Delete (asks first).
- The glue sends each `Reaction::Edit` and applies it to its own pack at once (`edit_here`: `hq_edit::apply` + `check`); `pack_changed` keeps the open quest and link picking across packs by id.
- **Quest form** (v0.12.0, `hq_form::QuestForm`, menu entry "Edit..."): a window over the canvas (`QuestForm::rect`, 360 wide at most), rows of label + control (`ROWS`): text fields (`hq_text::TextField`: blinking cursor, the cursor's line scrolled into sight, click to place the cursor), choices cycling on a click (`< square >`), yes / no flags; icon suggestions from the item names under the icon field (`suggestions`, `PickIcon`); the wheel scrolls the rows; bottom bar with the hint or the error, Cancel, Save (`info()` → `Edit::SetQuestInfo`). While it is up, `BookView::key` takes every key (the glue cancels the press, so `native_key` never sees K) and `typed` every character (new hook `typed.rs`, `native_screen_char_typed_pre`). No paste (no clipboard helper).
- **Tasks and rewards** (v0.13.0): `popup_layout(.., editing)` turns each task and reward icon's action into `EditTask` / `EditReward` ("Click to edit") and adds a green "+" (`PLUS_ICON`, `plus_icon`). The form engine is generic since then: `hq_form::Form` with rows (`Row`: text, choice, flag; `items` for item suggestions), a `Target` (quest, task, reward), `save()` → the `Edit`, `delete()` for an existing task or reward (red Delete button). A task or reward form's "Type" row rebuilds the rows (`task_rows`, `reward_rows`) and `memory` keeps the values of every type tried.
- **Chapters, groups, tables, settings** (v0.14.0): in edit mode `draw_sidebar` adds "+ New chapter" / "+ New group" rows (`EDIT_INK`); `right_click` over the open list gives a chapter's menu (Edit..., Move up / down, Delete with `AskDeleteChapter`) or a group's (Rename..., Delete the group). The canvas menu adds "Reward tables..." (`Action::Tables` → a `Menu` of tables → `OpenTable` → a `Menu` of entries with "+ Add an entry" and "Delete this table") and "Pack settings..." (`EditSettings`). Forms: `Form::chapter`, `group`, `settings`, `entry` (entries: the first five reward types, no table in a table, plus a weight). `pack_changed` keeps the chapter shown by id and turns to a new chapter.

The chapter list in edit mode (v0.14.0):
![[book-14-edit-chapters.svg]]

Linking by dragging the yellow dot (v0.15.1):
![[book-15-link-drag.svg]]

Edit mode's quest window and the task form (v0.13.0):


## Optional fallback: chest-menu book (not planned)
If vanilla-client support is ever wanted: pure pages in `hq_book_menu` (`layout()` / `click()`, like `smp_market_menu`), opened with `native_open_menu` (6 rows), the bottom row for navigation (`BACK=45`, `PREV=47`, `NEXT=51`, `CLOSE=53`), and the tracked quest in `native_sidebar`. See [[Example - smp mod (patterns to reuse)]] §4.
