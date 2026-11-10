---
tags: [reference, ftb-quests, rewards]
source: FTB docs "Quest Rewards" + reward/*.java
verified_on: 2026-11-10
---
# FTB Quests reward types

Back: [[FTB Quests - Overview]]

| Type | Role | Priority for us |
|---|---|---|
| **Item** | Gives an item (and a count) | 🔴 MVP |
| **XP** | Gives XP points | 🔴 MVP |
| **XP Levels** | Gives levels | 🔴 MVP |
| **Command** | Runs a command on completion (permission level 2 must be set in the reward) | 🟠 V1 |
| **Choice** | The player **chooses** an item from a table | 🟠 V1 |
| **Random** | A random item from a weighted table (always at least 1) | 🟠 V1 |
| **Loot** | Weighted draw from a table, may give nothing (*empty weight*) | 🟡 V2 |
| **All Table** | All the items of a table, unweighted | 🟡 V2 |
| **Toast** | Shows a toast with a message | 🟠 V1 |
| **Advancement** | Grants an advancement | 🟡 V2 |
| **Stage** | Adds or removes a game stage | ⚪ |
| **Currency** | Currency (present in the code, not in the doc) | 🟡 (possible link with `smp`'s economy) |
| **Custom** | Does nothing, used for scripts | 🟡 |

## Claiming
- `RewardAutoClaim`: `default`, `disabled`, `enabled`, `no_toast`, `invisible`. The file-level default is `disabled`.
- `RewardClaimType`: `CAN_CLAIM`, `CANT_CLAIM`, `CLAIMED`.
- A reward can be a **team reward**: claimed only once for the whole team, instead of once per player.
- A "Claim all" button claims everything. "Exclude from claim all" option (exact key not verified).
- `/ftbquests block_rewards` blocks claiming for a player.

## Reward tables / loot crates
- A table is a weighted list of items, in `reward_tables/<name>`.
- It can be converted into a **loot crate**: an `ftbquests:lootcrate` item that can be placed in vanilla loot tables, and crates can be nested.
- Commands: `import_reward_table_from_chest`, `export_reward_table_to_chest`.

Our implementation: [[Rewards - implementation]]
