---
tags: [adr, content]
status: accepted
date: 2026-11-16
---
# ADR-019: Big main chapters, few side paths

## Context
v0.21.0 grew the built-in pack from 81 to 324 quests by adding 15 chapters, for 24 in all (6 main, 18 side, in six groups). Bastien liked the number and the variety of quests, but not the chapter count: "too many side chapters, you get lost". He wants a lot to do in the main chapters and only a few side chapters.

## Options (asked 2026-11-16)
1. **Six big main chapters + three side paths** (9 chapters). The main story keeps its six stages (Getting started, Iron age, Diamonds, The Nether, The End, Beyond), each absorbing the side content of its stage; three side paths keep what belongs to no stage.
2. Three huge main chapters (Overworld, Nether, End) + three side paths (6 chapters).
3. Six big main chapters + two side paths (8 chapters).

## Decision
**Option 1** (Bastien). Done in v0.21.1:
- Main story: Getting started (45), Iron age (49), Diamonds (41), The Nether (33), The End (11), Beyond (29).
- Side paths: Farming and animals (39), Monsters and villages (35), Building and collecting (42).
- Each old chapter is a **band** of its new chapter: its own layout, starting two steps to the right of the quest it hangs from, bands stacked above and below the main line. A band whose first quests only needed quests of other chapters hangs from a quest of its new chapter instead (Wide world after the compass, Archaeology after the copper tools, the deep dark after diamond armour, Trial chambers after the diamond sword).

## Consequences
- Fewer, bigger canvases: the main line reads left to right, the bands give each theme its own row. To check in game that a 45-quest chapter still reads well at the default zoom.
- Moved quests got a new full id (`chapter/quest`), so progress on them from 0.21.0 (and on the old Further afield quests) is lost. Acceptable before 1.0.0; after it, never move a quest between chapters in the built-in pack.
- Future content goes into these nine chapters (as a new band if it is a theme of its own) rather than into new chapters.
