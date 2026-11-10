---
tags: [design, ui]
status: draft
date: 2026-11-10
---
# UI: the quest book

Reference: [[FTB editor and UI]] · API: [[Horizon hooks and API]] §5 · Decision: [[ADR-002 Authoritative server and dual UI]]

## Two interfaces, one logic
| | **A. "Chest" book** (server) | **B. Graphical book** (Horizon client) |
|---|---|---|
| Client required | any (vanilla included) | Horizon with the mod installed |
| Tech | `native_open_menu` (6 rows = 54 slots) | `native_mod_screen_open` + `native_screen_render_post` |
| Rendering | items with a name and lore | a free canvas: shapes, lines, icons, text |
| Phase | **MVP** | V2 |
| Editing | no (commands and files) | yes (V3) |

## A. Chest book (MVP)
These are pure pages in `hq_book_menu` (`layout()` / `click()`, testable like `smp_market_menu`).

```
Chapters page                Chapter page (list)            Quest page
┌─────────────────────────┐  ┌─────────────────────────┐   ┌─────────────────────────┐
│ [📘ch1][📗ch2][📕ch3]…  │  │ [✔q1][▶q2][🔒q3][?q4]…  │   │ [icon] Title            │
│                         │  │  (green = done, yellow =│   │ Tasks: [⛏ 3/5][✓]       │
│                         │  │   in progress, grey =   │   │ Rewards: [💎][XP]       │
│                         │  │   locked)               │   │                         │
│ ─────── nav bar ─────── │  │ ─────── nav bar ─────── │   │ [Submit][Claim]         │
│ [◀]   [?]   [▶]    [✖] │  │ [↩][◀]  [Claim all][▶]  │   │ [↩] [Track 📌]    [✖]  │
└─────────────────────────┘  └─────────────────────────┘   └─────────────────────────┘
```
- The bottom row (slots 45..53) is used for navigation: `BACK=45`, `PREV=47`, `NEXT=51`, `CLOSE=53` (smp convention). 45 items are shown per page.
- Status is shown by the icon (colored glass or the quest's item with a *glint*?) and by the lore ("✔ Completed", "🔒 Requires: …").
- Opening: the `/quests` command (or `/q`), and possibly a "book" item via `native_use_item`.
- **Tracked quest**: displayed in the sidebar (`native_sidebar`) with the task progress, updated once per second and only when lines change (`smp_board` pattern).
- Notifications: clickable chat message ("[View quest]" → `RunCommand /quests open <id>`), action bar, `ui.toast.challenge_complete` sound, title via the `title` command.

## B. Graphical book (V2)
- **Canvas**: transform `screen = (world - pan) * zoom + origin`. Pan with `screen_mouse_dragged_pre`, zoom with `screen_mouse_scrolled_pre`, click with `screen_mouse_button_pressed_pre`.
- Rendering:
  - shapes: `native_sprite` (shape textures in `assets/horizon_quest/textures/gui/shapes/`) or `native_quad`;
  - icons: `native_item_stack`;
  - dependency lines: `native_quad` (thickness via an oriented quad);
  - text: `native_text`.
- ⚠ **No documented 2D clip/scissor**: we'll have to crop ourselves (don't draw outside the area) or limit the canvas to full screen.
- Chapter sidebar, quest detail panel and buttons: **homemade widgets** (no library available).
- Click on an item task: `native_show_recipes(stack, lookup)` opens JEI/REI if present.
- Data: sent by the server via `hq_net` (definitions on connection and on reload, then progress deltas). Messages must be **chunked** (30,000-byte limit).
- Client toast: `native_add_toast` (⏳).
- Opening: a key (`native_mod_key`), the client command `native_command`, or the book in hand (`native_claim_items`).

## C. In-game editor (V3)
- Edit mode reserved for ops (level 2+, later with a permission node).
- Actions: create, move (drag), link dependencies (shift-click), edit fields (input via `screen_char_typed_pre`).
- Every change becomes a net message → the server validates, saves the chapter file and rebroadcasts.
- **Cheaper alternative**: edit the JSON by hand, plus `/quests reload`, plus an external web editor (?).
