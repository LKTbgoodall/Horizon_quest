---
tags: [design, format]
status: draft
date: 2026-11-10
---
# Quest file format

Inspiration: [[FTB file format]] · Decision: [[ADR-006 Quest definitions as JSON per chapter]]

## Layout
```
config/horizon_quests/quests/
├── quests.json            ← version + global settings + chapter groups
├── chapters/
│   ├── 01_getting_started.json
│   └── 02_nether.json
├── reward_tables/         ← V1
│   └── common_loot.json
└── lang/                  ← V1 (if texts are translatable)
    ├── fr_fr.json
    └── en_us.json
```

## Chapter example (v0)
```json
{
  "id": "getting_started",
  "title": "Getting started",
  "icon": "minecraft:oak_sapling",
  "progression": "linear",
  "quests": [
    {
      "id": "first_wood",
      "title": "Wood!",
      "description": ["Punch a tree. Yes, with your bare hands."],
      "x": 0, "y": 0,
      "tasks": [ { "id": "t1", "type": "item", "item": "#minecraft:logs", "count": 8 } ],
      "rewards": [ { "id": "r1", "type": "item", "item": "minecraft:apple", "count": 2 },
                   { "id": "r2", "type": "xp", "points": 20 } ]
    },
    {
      "id": "crafting_table",
      "title": "Crafting table",
      "x": 2, "y": 0,
      "dependencies": ["first_wood"],
      "tasks": [ { "id": "t1", "type": "item", "item": "minecraft:crafting_table", "count": 1 } ],
      "rewards": [ { "id": "r1", "type": "xp_levels", "levels": 1 } ]
    }
  ]
}
```
- `item`: `"minecraft:x"` for an id, `"#ns:tag"` for a tag.
- Task and reward ids are unique **within the quest**. The full id is `chapter/quest/task`.
- For any missing field, the chapter default is used, then the file default.
- Strict JSON, for now (`serde_json` is in the Horizon lock; a JSON5 parser is not → check whether we want comments).

## Validation on load
- Duplicate ids, unknown dependencies, **cycles**, unknown items (warning) → clear report in the console and in `/quests validate`.
- An invalid chapter is **ignored**, not the whole file.
