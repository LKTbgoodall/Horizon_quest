---
tags: [reference, ftb-quests, commands]
source: FTB docs "Commands"
verified_on: 2026-11-10
---
# FTB Quests commands

Back: [[FTB Quests - Overview]]
`<>` = required, `[]` = optional

| Command | Effect |
|---|---|
| `/ftbquests open_book [questId]` | opens the book, optionally on a quest |
| `/ftbquests editing_mode [true\|false] [player]` | enables or disables edit mode |
| `/ftbquests change_progress <player> <reset\|complete> <questId>` | changes the progress of a quest |
| `/ftbquests change_progress <player> <reset-all\|complete-all>` | resets or completes everything |
| `/ftbquests block_rewards [true\|false] [player]` | prevents claiming |
| `/ftbquests locked [true\|false] [player]` | locks the book |
| `/ftbquests reload` | reloads the config and the quests |
| `/ftbquests import_reward_table_from_chest <name> <pos>` | chest → table |
| `/ftbquests export_reward_table_to_chest <rewardId> <pos>` | table → chest |
| `/ftbquests delete_empty_reward_tables` | cleanup |
| `/ftbquests generate_chapter_with_all_items_in_game` | debug |
| `/ftbquests clear_item_display_cache` | client cache |

Our version: [[Commands and permissions]]
