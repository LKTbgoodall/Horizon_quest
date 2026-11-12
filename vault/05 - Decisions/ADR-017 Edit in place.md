---
tags: [adr]
status: accepted
date: 2026-11-12
---
# ADR-017: Edit in place, with pickers and panels

## Context
v0.12 to v0.14 edited quests through big forms (labels and fields, `< choice >` buttons, yes / no boxes, ids typed by hand). Bastien tried the quest form in game (2026-11-12): "horrible to edit". Asked what was worst: **typing ids** and **the look**. Asked where to go: **edit in place**, like FTB Quests.

## Options considered
1. **Edit in place** (chosen): the quest window is the editor; small panels beside it for settings; a grid to pick items and mobs.
2. Keep a form, compact and beside the quest, with visual pickers.

## Decision
- The quest window in edit mode: title, subtitle and description typed where they are shown (`Typed`, wrapped text in `hq_text`); the icon opens the picker; a gear opens the quest's settings panel.
- `hq_inline`: `Control` (text line, number with - / + and the wheel, switch, segmented choices, drawn node shapes with "A" for automatic, item slot, red button), `panel` beside the window (the window moves aside when both fit), `Picker` (every item, or every mob by spawn egg, with a search line, tags for tasks), and per `Subject` (quest, task, reward, chapter, group, settings, table entry) the controls and the `Edit` each click makes, applied at once.
- New tasks and rewards: a menu of types with icons; items and mobs through the picker; the new one's panel opens (`hq_edit::apply` returns its full id). A task's type is not changed: delete it and add another.
- `hq_form` is removed.

## Consequences
- No paste (no clipboard helper in Horizon). Advancement ids and commands are still typed (no advancement list on the client).
- Every click sends an edit and the server rewrites the pack: fine at pack sizes.

## Links
- [[ADR-016 Edit mode in the book]] · [[UI - Quest book]] · [[Decision index]]
