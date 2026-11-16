---
tags: [adr, content, layout]
status: accepted
date: 2026-11-16
---
# ADR-020: Compact vertical chapter layout

## Context
After [[ADR-019 Big main chapters, few side paths]], the nine chapters were the old chapters stacked as bands, quests two cells apart, up to 30 x 16 cells. Bastien could not find his way in them. The book opens a chapter zoomed out to fit all of it (`hq_book::fit`), so a sprawling chapter opens with tiny nodes. He pointed to All the Mods 10 / 11 as the model ([[FTB modpack layouts (ATM)]]): a line of milestones, short branches, tight spacing, images.

## Options (preview page drawn by `hq_book`, 2026-11-16)
- A. Horizontal lines: the main line left to right, each big theme on a titled line of its own.
- B. A vertical spine with branches left and right.
- C. Compact islands per theme.
- E. **Compact vertical** (asked after B: "I like vertical, but it is far too spread out; put it all in one block; the tree part of Getting started far away on its own, not a fan").

## Decision
**E** (Bastien, 2026-11-16), done in v0.21.2:
- The milestones of a chapter in a column, 1.5 cells apart (the main lines planned per chapter, `LANES` in the generator).
- Every branch off a milestone is laid out as a small tree (1.25 cells outwards, 1 cell between siblings), then packed left or right of the column at the free place that grows the chapter least (measured symmetrically around the column), close to its milestone, sides kept balanced. No quest closer than 1 cell to another.
- Shapes with a meaning: the chapter's first quest a circle, milestones hexagons (size 1.25), ordinary quests squares, optional quests diamonds, hidden section ends big hexagons.
- One dependency changed for a clean column: iron armour after the iron tools.

## Consequences
- Chapters are about half the size (Getting started 14 x 11.5 cells) and open less zoomed out.
- The layout came from a script (kept in the session scratchpad, not in the repo): later edits are made by hand or in the in-game editor, keeping the same rules.
- Not done: titles and section labels on the canvas, the closest thing to the modpacks' images. It would be a new feature (chapter decorations drawn by the book).
