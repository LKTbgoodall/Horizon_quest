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
- `item`: `"minecraft:x"` for an id, `"#ns:tag"` for a tag. The `minecraft:` namespace can be left out (`"apple"`).
- Task and reward ids are unique **within the quest**. The full id is `chapter/quest/task`.
- For any missing field, the chapter default is used, then the file default.
- Strict JSON, for now (`serde_json` is in the Horizon lock; a JSON5 parser is not → check whether we want comments). Unknown fields are ignored. A leading byte-order mark (Windows Notepad) is accepted.

## Fields (format version 1, as coded in `hq_model.rs`)
A real, complete example is the shipped pack: `mod/mixins/horizon_quests/data/quests/` (`quests.json` + `chapters/01_getting_started.json`).

| File / object | Fields (default) |
|---|---|
| `quests.json` | `version` (1), `progression` (`linear` \| `flexible`), `default_shape` (`square` \| `circle` \| `diamond` \| `hexagon` \| `octagon`), `default_size` (1.0), `auto_claim_rewards` (false), `detection_delay` (20 ticks), `groups` (`[{ id, title }]`) |
| chapter | `id`*, `title`, `subtitle`, `icon` (item id), `group`, `order` (0), `progression`, `default_shape`, `default_size`, `hidden` (false), `quests` |
| quest | `id`*, `title`, `subtitle`, `description` (list of paragraphs), `icon`, `x`, `y`, `shape`, `size`, `dependencies` (`quest` or `chapter/quest`), `dependency_mode` (`all_completed` \| `one_completed` \| `all_started` \| `one_started`), `min_required`, `visibility` (`always` \| `until_deps_visible` \| `until_deps_complete` \| `secret`), `optional` (false), `tasks`, `rewards` |
| task | `id`*, `title`, `type`*: `item` (`item`*, `count` 1, `consume` false) · `checkmark` · `kill` (`entity`*, `count` 1) |
| reward | `id`*, `auto_claim` (`default` \| `disabled` \| `enabled`), `type`*: `item` (`item`*, `count` 1) · `xp` (`points`*) · `xp_levels` (`levels`*) |

\* required. Chapters list by `order`, then by file name.

## Validation on load (`hq_validate`, done 2026-11-10)
- Every problem is reported with its place: `error: chapters/01_start.json > first_wood: unknown dependency "x"`. Shown in the console and, later, by `/quests validate`.
- **Errors** (the chapter is left out): file that is not valid JSON or has an unknown task/reward type, missing or badly formed id, duplicate chapter / quest / task / reward id, unknown dependency, a quest depending on itself, **dependency cycles** (across chapters too), `min_required` above the number of dependencies, `count` 0, an icon, item or entity that is not a registry id, a size ≤ 0.
- **Warnings** (the chapter loads): no title, unknown group, two quests at the same position, a dependency listed twice, `min_required` 0, a reward that gives nothing.
- An invalid chapter is **left out**, not the whole pack; chapters that depended on it are then left out too. An error in `quests.json` (invalid JSON, a `version` newer than the mod, `default_size` ≤ 0) **refuses the whole pack**.
- Not checked yet: whether an item or entity id **exists** in the game (needs the registries: glue, phase 2).
