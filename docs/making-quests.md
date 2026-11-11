# Making quests for Horizon Quests

A quest pack is a few JSON files: one for the pack's settings, one per chapter, and optionally reward tables. This guide shows where they go, what each field means, and how to check your work.

## Where a pack goes

The server uses the first of these that exists:

1. **The world's own folder**: `saves/<world>/horizon-quests/quests/`, with a `quests.json` in it.
2. **Data packs**: the folder `data/<namespace>/horizon_quests/` of any enabled data pack of the world (including content mods). Every namespace's chapters and tables make one pack. The settings are the first namespace's `quests.json`.
3. **The built-in example pack**, unless the server's config says `example_chapter = false`.

A pack folder looks like this (the same inside a data pack's `horizon_quests/` folder):

```
quests.json            the pack's settings (optional in a data pack)
chapters/
  01_getting_started.json
  02_the_nether.json
reward_tables/         optional
  common_loot.json
```

After editing, run `/quests reload` (a world folder) or `/reload` (data packs). `/quests validate` lists the problems found. A chapter with an error is left out, never the whole pack, and the rest keeps working.

Complete, working examples:
- the built-in pack: [`mod/mixins/horizon_quests/data/quests/`](../mod/mixins/horizon_quests/data/quests/);
- a data pack: [`examples/quest-datapack/`](../examples/quest-datapack/). Copy it into `saves/<world>/datapacks/` and run `/reload`.

## The rules of the files

- Strict JSON (no comments, no trailing commas). A byte-order mark is accepted.
- Ids are lowercase letters, digits, `_`, `-` and `.`. A quest's full id is `chapter/quest`, a task's `chapter/quest/task`. **Keep ids once players have progress**: progress is saved by id.
- Items, entities, dimensions and advancements are registry ids: `minecraft:oak_log`, or `oak_log` (the `minecraft:` namespace can be left out). An item task can name a tag: `#minecraft:logs`.
- Unknown fields are ignored, so a typo in an optional field name is silently dropped: check with `/quests validate`.

## `quests.json`

```json
{
  "version": 1,
  "progression": "linear",
  "default_shape": "square",
  "default_size": 1.0,
  "auto_claim_rewards": false,
  "detection_delay": 20,
  "groups": [ { "id": "main", "title": "Main story" } ]
}
```

| Field | Default | Meaning |
|---|---|---|
| `version` | 1 | The format version. A pack newer than the mod is refused. |
| `progression` | `linear` | `linear`: a task counts only once its quest is open. `flexible`: tasks count at any time; the quest completes when its dependencies are met too. |
| `default_shape` | `square` | The node shape: `square`, `circle`, `diamond`, `hexagon`, `octagon`. |
| `default_size` | 1.0 | The node size (1 is standard). |
| `auto_claim_rewards` | false | Give rewards without a click. A reward can override it. |
| `detection_delay` | 20 | Ticks between two looks at a player's inventory (the server's config can override it). |
| `groups` | none | Headings that gather chapters in the book's chapter list. |

## A chapter

```json
{
  "id": "getting_started",
  "title": "Getting started",
  "subtitle": "Your first days in a new world",
  "icon": "minecraft:oak_sapling",
  "group": "main",
  "quests": [ ... ]
}
```

| Field | Default | Meaning |
|---|---|---|
| `id` | required | The chapter's id. |
| `title`, `subtitle` | none | Shown in the chapter list and the book's header. |
| `icon` | none | An item shown in the chapter list. |
| `group` | none | A group id of `quests.json`. |
| `order` | 0 | Chapters are listed by `order`, then by file name. |
| `progression`, `default_shape`, `default_size` | the pack's | Override the pack's settings for this chapter. |
| `hidden` | false | Not listed in the book (its quests still count). |

## A quest

```json
{
  "id": "first_wood",
  "title": "Timber",
  "subtitle": "Every journey starts with a tree",
  "description": ["Gather a few logs.", "Any kind of wood will do."],
  "icon": "minecraft:oak_log",
  "x": 2, "y": 0,
  "dependencies": ["welcome"],
  "tasks": [ { "id": "logs", "type": "item", "item": "#minecraft:logs", "count": 8 } ],
  "rewards": [ { "id": "xp", "type": "xp", "points": 10 } ]
}
```

| Field | Default | Meaning |
|---|---|---|
| `id` | required | The quest's id, unique in the chapter. |
| `title`, `subtitle` | none | Its name, and a line under it. |
| `description` | none | Paragraphs shown in the quest window. |
| `icon` | from its first task | The item on its node. |
| `x`, `y` | 0 | Its place on the chapter's grid (in nodes; halves are fine). |
| `shape`, `size` | the chapter's | Its node's shape and size. |
| `dependencies` | none | Quests to do first: `quest` in the same chapter, `chapter/quest` in another. |
| `dependency_mode` | `all_completed` | `all_completed`, `one_completed`, `all_started`, `one_started`. |
| `min_required` | none | Open once this many dependencies are done, instead of all. |
| `visibility` | `always` | `always`, `until_deps_visible` (shown once its dependencies show), `until_deps_complete` (shown once they are done), `secret` (shown once completed). |
| `optional` | false | Not needed to complete the chapter. |
| `sequential_tasks` | false | Its tasks are done in order: a task counts only once the ones before it are done. |
| `repeatable` | false | Starts over once completed and its rewards claimed. It needs a task that is used up (kill, craft, checkmark, hand-in). |
| `repeat_cooldown` | 0 | Seconds before a repeatable quest starts over. |

## Tasks

Every task has an `id` (unique in its quest), a `type`, and an optional `title` that replaces the words the book makes from its type.

| `type` | Fields | Done when |
|---|---|---|
| `item` | `item`, `count` (1), `consume` (false) | the player holds `count` of the item. With `consume: true` the items are handed in: the player clicks the task in the quest window (or `/quests submit`), and they are taken. |
| `craft` | `item`, `count` (1) | the player takes `count` of the item out of a crafting grid or a furnace. |
| `kill` | `entity`, `count` (1) | the player kills `count` of the mob. |
| `checkmark` | none | the player clicks it in the quest window (or `/quests check`). |
| `xp_levels` | `levels` | the player reaches that experience level. |
| `dimension` | `dimension` | the player enters the dimension (`minecraft:the_nether`). |
| `location` | `from` `[x, y, z]`, `to` `[x, y, z]`, `dimension` (any) | the player stands inside that box of blocks (both corners included). |
| `advancement` | `advancement` | the player has the advancement (`minecraft:story/mine_stone`), even earned before. |

In a party, a task done by any member counts for everyone; an item task counts the member who holds the most, an experience task the highest level.

## Rewards

Every reward has an `id` (unique in its quest), a `type`, and optionally:
- `title`: the name shown instead of the one made from its type;
- `auto_claim`: `default` (the pack's setting), `enabled` or `disabled`;
- `team`: `true` for a reward claimed once for the whole party (by default each player claims their own).

| `type` | Fields | Gives |
|---|---|---|
| `item` | `item`, `count` (1) | the items (what does not fit drops at the player's feet) |
| `xp` | `points` | experience points |
| `xp_levels` | `levels` | experience levels |
| `command` | `command` | runs one line as the server console; `{player}` is the player's name, `{uuid}` their UUID |
| `toast` | `text` | a box on the player's screen; the reward's `title` is its heading |
| `random` | `table` | one entry of a reward table, drawn by weight |
| `choice` | `table` | one entry of a reward table, picked by the player in the quest window |

## Reward tables

A file in `reward_tables/`:

```json
{
  "id": "common_loot",
  "entries": [
    { "weight": 3, "type": "item", "item": "minecraft:bread", "count": 4 },
    { "weight": 1, "type": "item", "item": "minecraft:golden_apple", "title": "A treat!" },
    { "type": "xp_levels", "levels": 2 }
  ]
}
```

Each entry is a reward without an `id`: any type but `random` and `choice`, with a `weight` (1) and an optional `title`.

## Checking a pack

`/quests validate` (or the server log when the world opens) lists every problem with its place, for example `error: chapters/01_start.json > first_wood: unknown dependency "x"`.

- **Errors** leave the chapter out: invalid JSON, an unknown task or reward type, a missing, malformed or duplicate id, an unknown dependency, a dependency cycle, a `count` or `levels` of 0, a malformed item or entity id, an unknown reward table, a repeatable quest with nothing used up, an empty or multi-line command. An error in `quests.json` refuses the whole pack.
- **Warnings** keep the chapter: no title, an unknown group, two quests at the same place, a dependency listed twice, a reward that gives nothing, `repeat_cooldown` without `repeatable`, a choice reward set to claim itself.
- The validator does not know yet whether an item or a mob exists in the game: a misspelled `minecraft:oak_logg` passes, and shows as a missing icon in the book.

## Tips

- Lay quests out on whole or half grid steps (`"x": 2.5`), left to right, with room between them for the lines.
- Give every quest a `title` and an `icon`: the icon is what players see on the canvas.
- Test with a fresh world, then `/quests complete <you> <chapter/quest>` and `/quests reset <you> <chapter/quest>` to jump around.
