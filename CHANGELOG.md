# Changelog

Versions follow [the project's rule](vault/05%20-%20Decisions/ADR-013%20Version%20numbers.md): the middle number for a new feature, the last for a small addition or a fix, and 1.0.0 for the first public release. Versions tested in game have a git tag.

## Unreleased: towards 1.0.0

## 0.10.3
- A line into a quest that cannot be started yet is a darker grey and half see-through, so the lines into quests you can do now stand out. A fifth colour in the line sheet (`LINE_LOCKED`, 0.55 opacity baked in: the game cannot fade a sprite), now 512 x 352.

## 0.10.2
- Fixed: 0.10.1 drew no lines at all. The game leaves out GUI textures over 1024 x 512 pixels, and the new sheet was 256 x 640; it is now 512 x 288 (16 directions a row, two rows per colour, the four beads on the last row).

## 0.10.1
- Dependency lines in the style picked from a preview page of a dozen candidates: thin outlined chevrons (7 canvas units) pointing to the quest the line opens, a round bead (2.8 units) between each two, a chevron every 8.5 units, no line under them. Same colours as before; on the lines of the quest in focus the chevrons and beads slide along. Sprites from `horizon_quests:gui/chevrons`, drawn again by `tools/make-line-textures.py` (32 directions and a bead per colour). The dashes, glow and comets of 0.10.0 are gone, and so are the unused line quad helpers.

## 0.10.0
- Dependency lines redone without chevrons: a thin thread (2 canvas units) with a fine dark outline, dashed while its quests are not both done, solid green once they are. The lines of the quest in focus are solid, a little thicker, sit in a soft glow of their colour (cyan: what it needs, yellow: what it opens) and carry small comets (a bright head, a fading tail) running towards the quest each line opens. The chevron texture and `tools/make-line-textures.py` are gone.

## 0.9.5
- The chevrons no longer stick out of their line: the line is 5 canvas units wide again (its width before 0.9.3) and each chevron is sized from it (1.2 times the width, its stroke spanning 0.8 of that), at every zoom; one every 9 units.

## 0.9.4
- Lines redone again: a thin line (2.5 canvas units) with a fine dark outline, and smooth chevrons from a texture (`horizon_quests:gui/chevrons`, drawn by `tools/make-line-textures.py`: 32 directions in each line colour, outlined, lighter at the tip) every 12 units, pointing to the quest the line opens. No glow, no thick band, no gleam. On the lines of the quest in focus the chevrons slide along.

## 0.9.3
- Lines redrawn: 7 canvas units thick (was 5) with a dark border, chevrons across the whole band every 8 units. The lines of the quest in focus also glow (a soft band round them) and a light gleam runs along them every 1.5 s, on top of the flowing chevrons. Other lines stay still.

## 0.9.2
- The lines of the quest in focus (cyan: what it needs, yellow: what it opens) are animated: their chevrons slide from the quest needed towards the quest it opens, one step every 0.6 s, smoothly between ticks. Other lines stay still.

## 0.9.1
- A dependency line is green only once both of its quests are completed, not as soon as the dependency is (optional quests included).

## 0.9.0
- The built-in pack is a vanilla progression to the end of the game, 81 quests in 9 chapters. "Main story": Getting started, Iron Age (iron gear, bucket, lava, ores, compass), Diamonds (deepslate, diamond gear, obsidian, enchanting), The Nether (portal, quartz, gold and piglins, fortress, blaze rods, nether wart, brewing), The End (pearls, eyes of Ender, stronghold, the dragon, its egg), Beyond the dragon (gateways, end cities, elytra, shulker boxes, netherite, the Wither, a beacon). "Side paths": Further afield, Farm and food, Monster hunter. Each chapter opens from the one before; the side paths open early.
- Item icons in the nodes are a little bigger (square 0.58, circle 0.5, hexagon 0.46, diamond 0.37 of the drawn box).
- The open chapter list is as see-through as its closed strip.
- A test checks that every file of the built-in pack is embedded by the server.

## 0.8.11
- Node frames: every shape's border (outline, ring, inner shadow) is half as thick; the nodes keep their size and their face is larger. The locked veil follows the face.

## 0.8.10
- No band at the top: the canvas starts at the window's top. The bottom band holds the chapter's name and tally, "Claim all", the last notice, the party, the help ("drag: move | wheel: zoom") and the close button. The chapter subtitle is no longer shown.
- The closed chapter list's strip is 18 pixels wide (was 10) and more see-through, and is not drawn while the list is open (it showed through the panel).
- The selected quest has no white corners: it stays drawn raised, like the quest under the pointer.

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
