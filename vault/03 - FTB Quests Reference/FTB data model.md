---
tags: [reference, ftb-quests, data-model]
source: Quest.java, Chapter.java, BaseQuestFile.java, DependencyRequirement.java, ProgressionMode.java (FTB-Quests main) + Settings docs
verified_on: 2026-11-10
---
# FTB Quests data model

Back: [[FTB Quests - Overview]]

## Identifiers
Every object (file, group, chapter, quest, task, reward, table) has a **long `id` encoded as 16 hex characters**. Dependencies reference these ids.

## Quest: stored keys
| Category | Keys | Notes |
|---|---|---|
| Position | `x`, `y` | coordinates on the chapter grid (double) |
| Size / shape | `size` (0 = chapter default, 0..8), `icon_scale` (0.1..2), `min_width`, `shape`, `preset` | shapes: default, circle, square, rsquare, diamond, pentagon, hexagon, octagon, heart, gear |
| Dependencies | `dependencies` (list of ids, may include a chapter), `dep_control_pts`, `dependency_requirement`, `min_required_dependencies`, `hide_dependency_lines`, `hide_dependent_lines`, `max_completable_dependents` | |
| Visibility | `invisible`, `invisible_until_tasks`, `hide_until_deps_visible`, `hide_until_deps_complete`, `hide_details_until_startable`, `hide_text_until_complete`, `hide_lock_icon` | many **tristates** (true/false/inherited default) |
| Behavior | `optional`, `can_repeat`, `repeat_cooldown` (s), `ignore_reward_blocking`, `progression_mode`, `require_sequential_tasks` | |
| Misc | `guide_page`, `disable_recipe_mod` (the UI label is "disable JEI") | |
| Content | `tasks[]`, `rewards[]` (each with `id` + `type` + its fields) | |

> [!note] Text is not in the JSON
> The title, subtitle, description and tags are stored in the `lang/` translation files. Quests thus become translatable. **Idea to reuse.**

## Dependencies
- `dependency_requirement`: `all_completed` (default), `one_completed`, `all_started`, `one_started`.
- `min_required_dependencies > 0` **takes precedence** over `dependency_requirement` ("N out of M").
- `max_completable_dependents`: once N dependent quests are completed, the others get blocked. This allows **exclusive branches** ("choose your path").
- Loop protection (`DependencyLoopException`) and max depth 1000.

## Progression modes
`progression_mode`: `default` (inherited chapter → file), `linear` (file default), `flexible`.
- **Linear**: tasks only progress once the dependencies are fulfilled.
- **Flexible**: progress is counted even before. When the dependencies are completed, the quest completes if the tasks are already fulfilled.

## Chapter: keys
`id`, `filename`, `group` (empty = default group), `order_index`, `always_invisible`, `default_quest_shape`, `default_quest_size`, `default_hide_dependency_lines`, `default_min_width`, `progression_mode`, `consume_items`, `hide_quest_details_until_startable`, `hide_quest_until_deps_visible`, `hide_quest_until_deps_complete`, `hide_text_until_complete`, `default_repeatable_quest`, `require_sequential_tasks`, `preset`, `autofocus_id`, `quests[]`, `quest_links[]` (`id`, `linked_quest`, position), `images[]`.

## Global file (`data`): keys
`version` (13), `default_reward_team`, `default_reward_team_by_type`, `default_consume_items`, `default_autoclaim_rewards`, `default_quest_shape` ("circle"), `default_quest_disable_jei`, `emergency_items`, `emergency_items_cooldown` (300 s), `drop_loot_crates`, `loot_crate_no_drop`, `disable_gui`, `grid_scale` (0.5), `pause_game`, `lock_message`, `progression_mode`, `detection_delay` (20 ticks), `show_lock_icons`, `drop_book_on_death`, `hide_excluded_quests`, `fallback_locale`, `verify_on_load`, `suppress_all_autoclaiming`, `presets`.

> [!tip] `emergency_items`
> These are items that can be given back if the player loses their book (with a cooldown).

See also: [[FTB file format]], [[Horizon Quests data model]]
