---
tags: [ftb, reference, layout]
source: local ATM10 / ATM11 instances (config/ftbquests/quests), read 2026-11-16; screenshots from Bastien
verified_on: 2026-11-16
---
# How big modpacks lay their quests out (All the Mods 10 / 11)

Back: [[FTB Quests - Overview]] · [[FTB editor and UI]]

Read from the quest files of Bastien's local ATM10 (`.snbt`) and ATM11 (`.json5`) instances, 70 chapters each, to learn **how they arrange quests**, not to copy anything (their text and art stay theirs). SkyFactory 4 has no FTB Quests files (its progression is advancements).

## What they do
- **Many chapters, in about ten groups** (Main Questline, Exploration, Magic, Tech, Storage, Power, Resources...), but each chapter is **one theme**, so the list reads like a table of contents.
- **A spine of milestones**: the main questline chapters are one line (ATM10 chapter 1 and 2: left to right; ATM11 chapter 1: top to bottom) of milestones, with **short branches of one to three quests** off each milestone, perpendicular to the line, sometimes splitting in two diagonals.
- **Tight spacing**: the median distance between neighbours is 1 to 1.5 cells (FTB node size 1 ≈ one cell). Median line length 1.5 to 2.5 cells in the tidy chapters. Our node is 0.75 of a cell (`NODE` 24 / `GRID` 32), so the same spacing works for us.
- **Shape and size mean something**: milestones and bosses are big (size 1.25 to 2, often gears), ordinary quests size 1 in one shape per chapter, optional quests another shape.
- **Islands with headers** in reference chapters (storage, generators): each theme a compact block (a hub and its satellites, or a grid), separated by empty space.
- **Images on the canvas everywhere**: a big title image at the top of the chapter (ATM11 chapter 1: 8 x 2 cells), section banners over each island, item and block textures and mob art beside clusters (up to 116 images in one chapter). They are the main way a player finds their way around.
- Opening a chapter in FTB keeps its zoom; ours zooms out to fit the whole chapter (`hq_book::fit`), so a sprawling chapter opens with tiny nodes.

## What it means for us
- v0.21.1's chapters were the old chapters stacked as bands, sparse (2 cells apart): hard to read. See the layout candidates of 2026-11-16 (`build/quest-layouts.html`, rendered with `hq_book` itself) and [[2026-11-16]].
- A canvas decoration feature (a chapter title, section labels, big item icons) would be the closest thing to their images we can ship without art of our own: not built yet.
