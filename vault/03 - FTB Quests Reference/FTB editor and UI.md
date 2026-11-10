---
tags: [reference, ftb-quests, ui]
source: FTB docs Navigating, Tips & Tricks, Styling, Opening
verified_on: 2026-11-10
---
# Editor and UI (FTB Quests)

Back: [[FTB Quests - Overview]]

## Opening the book
- Book icon at the top left of the inventory.
- **Quest book** item (craftable), right-click.
- Dedicated key (**unbound** by default).
- `/ftbquests open_book [questId]`.

## Screen layout
```
┌──────────┬────────────────────────────────────────┬─────┐
│ Chapters │        Chapter CANVAS                  │ i ⚙ │
│ (sidebar │   (o)──►(o)──►(◇)                      │Claim│
│ on hover │          └──►(o)                       │ all │
│ + pin)   │   drag = pan, wheel = zoom             │     │
└──────────┴────────────────────────────────────────┴─────┘
Click on a quest → window: title, subtitle, description,
tasks (click = recipe/info), rewards (click = claim),
arrows ◄ ► = dependencies / dependents, link icon, pin = track the quest
```
- The chapter sidebar opens when hovering the left edge, with a pin to keep it open.
- Drag an empty area to **pan**. The "i" icon lists the shortcuts and `Ctrl+F` searches for a quest.
- **Pinned quest**: it is tracked on screen (HUD).

## Edit mode
- `/ftbquests editing_mode [true|false] [player]`, with the `ftbquests.editor` permission.
- Right-click on empty space → create a quest by choosing a task type.
- A "+" in the tasks and rewards sections to add some.
- Right-click on a quest or a chapter → edit, move, delete.
- The green "+" in the sidebar creates a chapter or a group.
- The gear (bottom right) → edit the reward tables.

## Theme
- `ftbquests/ftb_quests_theme.txt` in a resource pack, with `[*]`, `[#tag]`, `[quest-id]` selectors.
- Custom shapes: `textures/shapes/<name>/{background,outline,shape}.png`.

Our version: [[UI - Quest book]]
