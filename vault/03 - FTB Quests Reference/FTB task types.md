---
tags: [reference, ftb-quests, tasks]
source: FTB docs "Quest Types"
verified_on: 2026-11-10
---
# FTB Quests task types

Back: [[FTB Quests - Overview]]

| Type | Role | Key options (known) | Priority for us |
|---|---|---|---|
| **Item** | Obtain an item (n copies) | item, count, consume (take the items or just check), tag/components matching (via FTB Filter System) | 🔴 MVP |
| **Checkmark** | The player clicks to confirm | (none) | 🔴 MVP |
| **Kill Entity** | Kill n entities of a type | entity, count | 🔴 MVP |
| **XP Levels** | Have / spend n levels | value, points/levels, consume | 🟠 V1 |
| **Visit Dimension** | Go to a dimension | dimension | 🟠 V1 |
| **Location** | Enter an area (cuboid) | dim, x/y/z, w/h/d | 🟠 V1 |
| **Advancement** | Obtain a vanilla advancement | advancement, criterion | 🟠 V1 (if hook) |
| **Stat** | A vanilla stat ≥ value | stat, value | 🟡 V2 |
| **Observation** | Look at a block or entity for n ticks | target, duration | 🟡 V2 |
| **Visit Biome** | Be in a biome | biome | 🟡 V2 |
| **Find Structure** | Be in a structure | structure / tag | 🟡 V2 |
| **Fluid** | Provide a fluid (via a task block) | fluid, amount | ⚪ later |
| **Forge Energy** | Provide energy | amount | ⚪ not relevant on Horizon for now |
| **Stage** | Have a "game stage" | stage | ⚪ (depends on GameStages) |
| **Custom** | Script-driven (KubeJS) | — | 🟡 via our mod's public API |
| **Image** | (pseudo-type) decorative image | — | 🟡 |

> [!warning] Unverified info
> The precise task options (consume, NBT, count) are **not documented** in the pages read. The `task/` folder of the source code has not been read. The options above come from general knowledge of the mod.

Mapping to Horizon hooks: [[Tasks - hook mapping]]
