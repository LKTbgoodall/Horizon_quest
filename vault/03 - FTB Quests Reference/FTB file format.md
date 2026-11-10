---
tags: [reference, ftb-quests, storage]
source: ServerQuestFile.java, BaseQuestFile.java (main)
verified_on: 2026-11-10
---
# File format (FTB Quests)

Back: [[FTB Quests - Overview]]

> [!note] SNBT → JSON5
> Older versions (1.16 to 1.20) used **SNBT**. The current branch uses **JSON5**, file version 13. The exact extension comes from `Json5Util.FILE_EXT` (probably `.json5`, not confirmed).

## Quest definition (shared, versionable)
```
config/ftbquests/quests/
├── data.json5               ← global settings (see FTB data model)
├── chapter_groups.json5     ← list of groups
├── chapters/
│   └── <filename>.json5     ← one file per chapter (quests, links, images)
├── reward_tables/
│   └── <name>.json5         ← id, order_index, data
└── lang/
    └── en_us.json5 …        ← titles, subtitles, descriptions
```
One file per chapter allows **readable git diffs** and teamwork. It is a good choice to reuse.

## Progress (per world)
```
<world>/ftbquests/<team-uuid>.json5   ← TeamData: tasks, completed quests, claimed rewards
```

## Synchronization (general knowledge, not verified in the code)
The server is **authoritative**: it sends the full quest file to the client on connection, then progress deltas. In edit mode, the client sends its changes back to the server, which saves and rebroadcasts.

Our format: [[Quest file format]]
