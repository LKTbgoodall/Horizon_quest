# Changelog

Versions follow [the project's rule](vault/05%20-%20Decisions/ADR-013%20Version%20numbers.md): the middle number for a new feature, the last for a small addition or a fix, and 1.0.0 for the first public release. Versions tested in game have a git tag.

## Unreleased: towards 1.0.0

## 0.7.6
- Counts: smooth anti-aliased digits from a texture (`textures/gui/digits.png`, made by `tools/make-digit-textures.py`), white with a dark edge; the half-pixel digits stay as the fallback without the content half.

## 0.7.5
- Counts: finer digits drawn in half pixels, with a dark edge all round so they read on any item.
- The checkmark button is smoother (half pixels too); once ticked its own check turns green instead of getting a second check on top.

## 0.7.4
- A locked quest shows its icon greyed out and a small padlock on its bottom-right corner.
- A checkmark task is a round grey button with a white check instead of a book.
- Counts on icons are small digits tucked in the bottom-right corner, as in FTB-style books, so they no longer cover the item.

## 0.7.3
- The check sits on the icon's top-right corner.
- Escape closes the window on top (recipes, choice, then the quest window) before the book.
- The choice window and the recipe view open over the quest window, not in the middle of the whole book.
- Fix: after a world whose server never answered, the next world now greets its server and reads the tracked quest again.
- The code no longer carries the early "the glue will call it" dead-code allowances, nor the old side panel's layout: the game builds the mod with no warning.
- Documentation for players and pack makers: the README, [docs/making-quests.md](docs/making-quests.md), this changelog, `release/`.

## 0.7.2
- A thicker check on what is done; experience levels counted as a plain number.
- A choice reward is one chest icon ("?", then "!" once it can be claimed) that opens a "Choose one reward" window.

## 0.7.1
- A check on a task done or a reward claimed, a "!" on a reward to claim.
- Counts drawn as in an inventory; item tooltips as in Minecraft (name, mod, what a click does).
- A pin replaces the Track button.

## 0.7.0
- The quest window replaces the side panel: tasks and rewards as clickable icons, then the quest's state and description.
- Item tags are drawn as one of their items.

## 0.6.2
- White corners round the selected quest; the quest under the pointer is drawn a little bigger; its title in a violet-framed tooltip.

## 0.6.1
- Dependency lines as bands with chevrons; what the quest under the pointer needs in cyan, what it opens in yellow.

## 0.6.0
- Node frames drawn from a texture (5 shapes x 5 states), the flat shapes as a fallback.
- The registry icon, `mod/icon.png`.

## 0.5.1 (tested in game, solo and LAN)
- An item row's icon no longer disappears under the hover shade.

## 0.5.0
- A tracked quest in the top-left corner; the party's name in the book; a recipe view for items; team rewards (`"team": true`).
- The client greets the server on joining, so the tracked quest and toasts work before the book is opened. Protocol 3.

## 0.4.0
- Quests from data packs (`data/<namespace>/horizon_quests/`), read again on `/reload`. An example data pack.

## 0.3.0
- Server settings: `parties`, `party_size`, `detection_delay`, `completion_messages`. A per-player file: `toasts`, `toast_seconds`.
- Players' inventories are looked at on ticks of their own.

## 0.2.0
- Parties: create, invite, join, leave, kick, with one shared progress.

## 0.1.2
- The Quest Book's own texture; code and content packed as one `horizon_quests.hmod`.

## 0.1.1
- The Quest Book item, given on a player's first join; no more HUD hint.

## 0.1.0 (tested in game, solo and LAN)
- The first private release: the graphical book, 9 task types, 7 reward types, reward tables, repeatable quests, tasks in order, toasts, saving, `/quests`, and two example chapters.
