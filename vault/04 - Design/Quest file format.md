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
├── reward_tables/         ← since v0.0.6
│   └── common_loot.json
└── lang/                  ← V1 (if texts are translatable)
    ├── fr_fr.json
    └── en_us.json
```

## Where a pack is read from (v0.4.0)
1. `<world>/horizon-quests/quests/` when it has a `quests.json` (the layout above).
2. Else the world's **data packs**: `data/<namespace>/horizon_quests/` with the same files (`quests.json` optional, `chapters/*.json`, `reward_tables/*.json`). The chapters and tables of every namespace and every enabled pack make one pack; the settings are the first namespace's `quests.json` (others are warned about); any other file is warned about and left out. Problems name the file as `data/<ns>/horizon_quests/<path>.json`, or `chapters/<ns>:<file>.json` for what the validator finds inside. Read again on `/reload`. Example: `examples/quest-datapack/` in the repo.
3. Else the built-in example chapters, unless `example_chapter = false`.

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
A real, complete example is the shipped pack: `mod/mixins/horizon_quests/data/quests/` (`quests.json`, `chapters/01_getting_started.json`, `chapters/02_further_afield.json`, `reward_tables/*.json`).

| File / object | Fields (default) |
|---|---|
| `quests.json` | `version` (1), `progression` (`linear` \| `flexible`), `default_shape` (`square` \| `circle` \| `diamond` \| `hexagon`; `octagon` reads as `hexagon` since 0.8.7)\| `octagon`), `default_size` (1.0), `auto_claim_rewards` (false), `detection_delay` (20 ticks), `groups` (`[{ id, title }]`) |
| chapter | `id`*, `title`, `subtitle`, `icon` (item id), `group`, `order` (0), `progression`, `default_shape`, `default_size`, `hidden` (false), `quests` |
| quest | `id`*, `title`, `subtitle`, `description` (list of paragraphs), `icon`, `x`, `y`, `shape`, `size`, `dependencies` (`quest` or `chapter/quest`), `dependency_mode` (`all_completed` \| `one_completed` \| `all_started` \| `one_started`), `min_required`, `visibility` (`always` \| `until_deps_visible` \| `until_deps_complete` \| `secret`), `optional` (false), `sequential_tasks` (false), `repeatable` (false), `repeat_cooldown` (0 seconds), `tasks`, `rewards` |
| task | `id`*, `title`, `type`*: `item` (`item`*, `count` 1, `consume` false) · `checkmark` · `kill` (`entity`*, `count` 1) · `craft` (`item`*, `count` 1) · `xp_levels` (`levels`*) · `dimension` (`dimension`*) · `location` (`dimension`, `from`* `[x, y, z]`, `to`* `[x, y, z]`: a box of blocks, both corners included) · `advancement` (`advancement`*) |
| reward (progress: `picked` since v0.8.1 records the table entry a random or choice reward gave) | `id`*, `title`, `auto_claim` (`default` \| `disabled` \| `enabled`), `team` (false; since v0.5.0, true: one member claims it for the whole party), `type`*: `item` (`item`*, `count` 1, `max_count`: since v0.8.0, a count drawn from `count` to `max_count` at the claim) · `xp` (`points`*) · `xp_levels` (`levels`*) · `command` (`command`*, one line, `{player}` and `{uuid}` replaced, run as the console) · `toast` (`text`; the reward's `title` is the toast's title) · `random` (`table`*) · `choice` (`table`*) |
| reward table (`reward_tables/*.json`) | `id`*, `entries`*: `[{ weight (1), title, type* + that type's fields }]`; an entry is any reward type but `random` / `choice` |

\* required. Chapters list by `order`, then by file name.

## Validation on load (`hq_validate`, done 2026-11-10)
- Every problem is reported with its place: `error: chapters/01_start.json > first_wood: unknown dependency "x"`. Shown in the console and, later, by `/quests validate`.
- **Errors** (the chapter is left out): file that is not valid JSON or has an unknown task/reward type, missing or badly formed id, duplicate chapter / quest / task / reward id, unknown dependency, a quest depending on itself, **dependency cycles** (across chapters too), `min_required` above the number of dependencies, `count` or `levels` 0, an icon, item, entity, dimension or advancement that is not a registry id, a size ≤ 0, an unknown reward table, an empty or multi-line `command`, **a repeatable quest without a task that is used up** (kill, craft, checkmark, hand-in). A reward table: bad or duplicate id, no entries, every weight 0, an entry that is another table.
- **Warnings** (the chapter loads): no title, unknown group, two quests at the same position, a dependency listed twice, `min_required` 0, a reward that gives nothing, `repeat_cooldown` without `repeatable`, a choice reward with `auto_claim: enabled` (a choice is never claimed automatically).
- An invalid chapter is **left out**, not the whole pack; chapters that depended on it are then left out too. A reward table with an error is left out the same way, and then the chapters that name it. An error in `quests.json` (invalid JSON, a `version` newer than the mod, `default_size` ≤ 0) **refuses the whole pack**.
- Not checked yet: whether an item or entity id **exists** in the game (needs the registries: glue, phase 2).
