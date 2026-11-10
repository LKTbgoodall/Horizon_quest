---
tags: [reference, ftb-quests]
source: docs.feed-the-beast.com, github.com/FTBTeam/FTB-Quests (main branch), CurseForge
verified_on: 2026-11-10
---
# FTB Quests: overview

> [!info] Purpose of this note
> This is the **model** we want to reproduce on Horizon. This section serves as a memory aid so we don't forget a feature. We copy **neither code nor assets**: FTB Quests is licensed *All Rights Reserved*, we only reimplement the concepts.

## In one sentence
FTB Quests is a **team-oriented** quest mod, with a **quest book** (GUI with a pannable canvas) organized into **chapters**. Each quest contains **tasks** to complete and **rewards** to claim, and quests are linked by **dependencies**.

## Hierarchy
```
Quest file (data)
└── Chapter group
    └── Chapter
        ├── Quests ──┬── Tasks (1..n)
        │            └── Rewards (0..n)
        ├── Quest links (quest_links: a quest shown in another chapter)
        └── Images (decorative, on the canvas)
Reward tables (reward tables / loot crates)
```

## Details
- [[FTB data model]]: all the fields of quests and chapters, dependencies and visibility
- [[FTB task types]]
- [[FTB reward types]]
- [[FTB teams and progress]]
- [[FTB editor and UI]]
- [[FTB file format]]
- [[FTB commands]]

## Versions, platforms and license
- Loaders: Forge and NeoForge (and Fabric in the repository: `common/`, `fabric/`, `neoforge/`). Minecraft versions up to 26.1.2 (file 26.1.2.10 of 5 Nov 2026).
- License: **All Rights Reserved**.
- Integrations via *FTB XMod Compat*: JEI/REI, KubeJS, GameStages, FTB Filter System.

## Known complaints and limits (to do better)
- Task configuration (consuming items, NBT/components matching) is poorly documented.
- The in-game editor is powerful but hard to discover: lots of right-clicks and hidden menus.
- The system depends on FTB Teams and FTB Library.
- See [[Vision and goals]] for our improvement areas.
