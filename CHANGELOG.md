# Changelog

Versions follow [the project's rule](vault/05%20-%20Decisions/ADR-013%20Version%20numbers.md): the middle number for a new feature, the last for a small addition or a fix, and 1.0.0 for the first public release. Versions tested in game have a git tag.

## Unreleased: towards 1.0.0

## 0.8.9
- The chapter list is closed by default, a thin strip with an arrow on the book's left; the pointer on it opens the list over the canvas without a click, and leaving the list closes it. The canvas takes the whole width.
- The list's new look: a grey panel with a light edge, grey group headings with a triangle (the old gold one was drawn without alpha and never showed), 12-pixel chapter icons, chapters indented under their group, the current chapter outlined, no progress bars; a chapter's name is green when all its quests are done, cyan when it has rewards to claim.

## 0.8.8
- Chapter groups fold: a group heading in the chapter list has a triangle (down when open, right when folded) and a click folds or unfolds it, for as long as the client runs.
- The example pack gathers its chapters in two groups, "Basics" and "Exploration".

## 0.8.7
- The octagon shape is gone: "octagon" in a pack reads as a hexagon; the example quests that had one are hexagons. The node sheet is 4 shapes x 4 states (the blue rewards-waiting column is gone: those quests use the done cell and their "!" badge).
- Node icons are about 10 % smaller (square 0.54, circle 0.47, hexagon 0.43, diamond 0.34 of the drawn box).

## 0.8.6
- A quest whose rewards wait is drawn in the done green instead of blue, with a red "!" badge on its node's top-right corner, a little outside the frame (the sign sheet's alert, pixel art without it).

## 0.8.5
- Node shapes: the diamond's ring is thinner in its texture (drawn half as large again, it looked twice as thick as the others) and its icon a little bigger; hexagons are drawn bigger (1.28 x a square) with a slightly thinner ring. Each shape has its own ring depths.

## 0.8.4
- Node shapes: circles, hexagons and octagons are drawn a little larger and diamonds half as large again, so they weigh as much as a square; their ring is thicker; each shape has its own icon size, with a margin. The frame sheet is 64 pixels a cell, anti-aliased, so slanted edges are smooth.

## 0.8.3
- The example chapters use every node shape, to compare them; "Settled in" is an octagon instead of a diamond.

## 0.8.2
- A locked node's veil covers the frame's whole face in its shape, instead of a square over the icon.

## 0.8.1
- A random or choice reward, once claimed, shows only the entry it gave (icon and tooltip). The progress file remembers it (`picked`); rewards claimed before keep the old look.
- The padlock is a third of the node at any zoom (it stayed big when zoomed out).

## 0.8.0
- Item rewards can give a count range: `"count": 3, "max_count": 5` draws 3 to 5 at the claim; the book shows "3-5". The example "Leftovers" gives 2-4 emeralds.
- Random and choice rewards show their entries' items in turn (every 0.8 s) instead of a chest; the random one has a "?" until it can be claimed. Their tooltips are a plain list ("- 4x Iron ingot"), with each entry's chance for a random reward ("[33.33%]").
- Smooth signs from a texture (`textures/gui/marks.png`, made by `tools/make-mark-textures.py`): the check, "?", "!", padlock and checkmark button; a ticked checkmark button turns green with a white check. The pixel art stays as the fallback.

## 0.7.7
- Bigger count digits (4.2 x 5.6 GUI pixels a cell instead of 3.3 x 4.4).

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
