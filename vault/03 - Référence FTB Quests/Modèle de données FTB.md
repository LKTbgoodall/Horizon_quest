---
tags: [reference, ftb-quests, data-model]
source: Quest.java, Chapter.java, BaseQuestFile.java, DependencyRequirement.java, ProgressionMode.java (FTB-Quests main) + docs Settings
verifie_le: 2026-11-10
---
# Modèle de données FTB Quests

Retour : [[FTB Quests - Vue d'ensemble]]

## Identifiants
Chaque objet (fichier, groupe, chapitre, quête, tâche, récompense, table) a un `id` **long encodé en 16 caractères hexa**. Les dépendances référencent ces id.

## Quête : clés stockées
| Catégorie | Clés | Notes |
|---|---|---|
| Position | `x`, `y` | coordonnées sur la grille du chapitre (double) |
| Taille / forme | `size` (0 = défaut chapitre, 0..8), `icon_scale` (0.1..2), `min_width`, `shape`, `preset` | formes : default, circle, square, rsquare, diamond, pentagon, hexagon, octagon, heart, gear |
| Dépendances | `dependencies` (liste d'id, peut inclure un chapitre), `dep_control_pts`, `dependency_requirement`, `min_required_dependencies`, `hide_dependency_lines`, `hide_dependent_lines`, `max_completable_dependents` | |
| Visibilité | `invisible`, `invisible_until_tasks`, `hide_until_deps_visible`, `hide_until_deps_complete`, `hide_details_until_startable`, `hide_text_until_complete`, `hide_lock_icon` | beaucoup de **tristate** (true/false/défaut hérité) |
| Comportement | `optional`, `can_repeat`, `repeat_cooldown` (s), `ignore_reward_blocking`, `progression_mode`, `require_sequential_tasks` | |
| Divers | `guide_page`, `disable_recipe_mod` (le libellé UI est « disable JEI ») | |
| Contenu | `tasks[]`, `rewards[]` (chacun `id` + `type` + ses champs) | |

> [!note] Les textes ne sont pas dans le JSON
> Le titre, le sous-titre, la description et les tags sont stockés dans les fichiers de traduction `lang/`. Les quêtes deviennent ainsi traduisibles. **Idée à reprendre.**

## Dépendances
- `dependency_requirement` : `all_completed` (défaut), `one_completed`, `all_started`, `one_started`.
- `min_required_dependencies > 0` **prime** sur `dependency_requirement` (« N parmi M »).
- `max_completable_dependents` : une fois que N quêtes dépendantes sont terminées, les autres se bloquent. Cela permet des **branches exclusives** (« choisis ta voie »).
- Protection contre les boucles (`DependencyLoopException`) et profondeur max 1000.

## Modes de progression
`progression_mode` : `default` (hérité chapitre → fichier), `linear` (défaut du fichier), `flexible`.
- **Linear** : les tâches ne progressent qu'une fois les dépendances remplies.
- **Flexible** : la progression est comptée même avant. À la complétion des dépendances, la quête se termine si les tâches sont déjà remplies.

## Chapitre : clés
`id`, `filename`, `group` (vide = groupe par défaut), `order_index`, `always_invisible`, `default_quest_shape`, `default_quest_size`, `default_hide_dependency_lines`, `default_min_width`, `progression_mode`, `consume_items`, `hide_quest_details_until_startable`, `hide_quest_until_deps_visible`, `hide_quest_until_deps_complete`, `hide_text_until_complete`, `default_repeatable_quest`, `require_sequential_tasks`, `preset`, `autofocus_id`, `quests[]`, `quest_links[]` (`id`, `linked_quest`, position), `images[]`.

## Fichier global (`data`) : clés
`version` (13), `default_reward_team`, `default_reward_team_by_type`, `default_consume_items`, `default_autoclaim_rewards`, `default_quest_shape` ("circle"), `default_quest_disable_jei`, `emergency_items`, `emergency_items_cooldown` (300 s), `drop_loot_crates`, `loot_crate_no_drop`, `disable_gui`, `grid_scale` (0.5), `pause_game`, `lock_message`, `progression_mode`, `detection_delay` (20 ticks), `show_lock_icons`, `drop_book_on_death`, `hide_excluded_quests`, `fallback_locale`, `verify_on_load`, `suppress_all_autoclaiming`, `presets`.

> [!tip] `emergency_items`
> Ce sont des items qu'on peut redonner si le joueur perd son livre (avec un cooldown).

Voir aussi : [[Format de fichiers FTB]], [[Modèle de données Horizon Quest]]
