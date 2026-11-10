---
tags: [design, commands]
status: draft
date: 2026-11-10
---
# Commands and permissions

Reference: [[FTB commands]] · API: [[Horizon hooks and API]] §6 · Pattern: [[Example - smp mod (patterns to reuse)]] §3

> Reminder: one usage-string is registered **per form**. Parsing is done by hand in `hq_commands` (pure), which returns `Vec<Out>`.

## Players (level 0)
| Usage-string | Effect | Phase |
|---|---|---|
| `quests` | opens the book | MVP |
| `quests open <quest:word>` | opens a quest | MVP |
| `quests track [quest:word]` | tracks a quest (sidebar), or stops tracking if the argument is empty | MVP |
| `quests claim [quest:word]` | claims (everything, or a single quest) | MVP |
| `quests check <task:word>` | validates a checkmark task | MVP |
| `quests submit <quest:word>` | submits the items (consume tasks) | MVP |
| `quests team create <name:word>` | creates a party | V1 |
| `quests team invite <player:player>` / `accept` / `leave` / `info` | team management | V1 |

## Admins (level 2+)
| Usage-string | Level | Effect | Phase |
|---|---|---|---|
| `quests reload` | 2 | reloads the definitions | MVP |
| `quests validate` | 2 | validation report | MVP |
| `quests complete <player:player> <quest:word>` | 2 | forces completion | MVP |
| `quests reset <player:player> <quest:word>` | 2 | resets a quest | MVP |
| `quests reset_all <player:player>` | 3 | resets everything | MVP |
| `quests progress <player:player> <task:word> <amount:integer>` | 2 | custom progress | V2 |
| `quests lock <player:player> <state:boolean>` | 3 | locks the book | V1 |
| `quests block_rewards <player:player> <state:boolean>` | 3 | blocks claims | V1 |
| `quests edit [state:boolean]` | 2 | edit mode | V3 |
| `quests import_ftb <path:text>` | 4 | FTB Quests import | V3 |

## Alias
`q` → `quests` (to check: is a separate usage-string needed?).

## Permissions
- MVP: op levels (`NativeCommand::op(usage, level)`), plus a re-check in the rules.
- Later: `horizon_quests.admin`, `horizon_quests.editor` nodes once `native_has_permission` ships (⏳).
