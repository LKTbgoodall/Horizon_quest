---
tags: [design, ui]
status: draft
date: 2026-11-10
updated: 2026-11-10
---
# UI: the quest book

Reference: [[FTB editor and UI]] · API: [[Horizon hooks and API]] §5 · Decision: [[ADR-008 Graphical client book first]]

> [!important] Decision (2026-11-10)
> The MVP ships the **graphical book** directly (Horizon client with the mod). The chest-menu book is **not planned**; it stays described at the end of this note as an optional fallback for vanilla clients.

> [!important] Coded, pure, in `hq_book.rs` (2026-11-10): not seen in game yet
> `BookView::scene(&Screen)` returns a **draw list** (`Fill`, `Quad`, `Text`, `Item`) and **click zones**; `click`, `drag` and `scroll` handle input. The glue will only carry the draws out. What it does, as built:
> - Regions (since v0.8.10): no top band; a strip of 18 px on the left (the closed chapter list, opened on hover as a panel of 30 % of the width, 110 to 170 px, over the canvas), the canvas, the quest window over it, and one bottom band of 14 px (chapter name and tally, "Claim all (n)", the last notice, the party, help, close).
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

## In-game editor (phase 5)
- Edit mode for ops (level 2+, later a permission node).
- Create, move (drag), link dependencies (shift-click), edit fields (text input via `screen_char_typed_pre`).
- Every change becomes a net message → the server validates, saves the chapter file and rebroadcasts.
- Until then: edit the JSON files by hand and run `/quests reload`.

## Optional fallback: chest-menu book (not planned)
If vanilla-client support is ever wanted: pure pages in `hq_book_menu` (`layout()` / `click()`, like `smp_market_menu`), opened with `native_open_menu` (6 rows), the bottom row for navigation (`BACK=45`, `PREV=47`, `NEXT=51`, `CLOSE=53`), and the tracked quest in `native_sidebar`. See [[Example - smp mod (patterns to reuse)]] §4.
