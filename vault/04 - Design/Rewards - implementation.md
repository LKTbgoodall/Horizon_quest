---
tags: [design, rewards]
status: draft
date: 2026-11-10
---
# Rewards: implementation

Sources: [[FTB reward types]] × [[Horizon hooks and API]]

| Reward | Implementation | Status | Phase |
|---|---|---|---|
| **Item** | `native_give_stack(uuid, stack)` → if anything is left: `native_run_command("summon item …")` at the player's position, the rest is **dropped at the player's feet** (decided 2026-11-10, like FTB) | ✅ | MVP |
| **XP / levels** | `native_give_experience(uuid, pts)` / `native_give_levels(uuid, lvls)`. If they return `false`, retry on the next tick | ✅ | MVP |
| **Command** | `native_run_command(line)` on the tick (level 4, console), with `{player}` and `{uuid}` replaced (`hq_session::reward_command`: refused for a name that is not a plain player name, or a command of several lines). Later `native_run_command_as` (⏳) | ✅ coded v0.0.6 | V1 |
| **Toast** | `ToClient::Toast` to a player whose book has talked to the server; the client draws it **on our own HUD** (`hq_book::Toasts`: top-right corner, 5 s, one at a time, its time paused while the book is open). There is no `native_add_toast`, and `native_chat` is not in the catalog. A player without the book gets a chat line (`native_tell`) | ✅ coded v0.0.6 | V1 |
| **Choice** | the quest panel lists the table's entries with a "Pick" button each → `ToServer::Claim { reward, choice }`. Never auto-claimed; `/quests claim` skips it (`NeedsChoice`) | ✅ coded v0.0.6 | V1 |
| **Random** | **our own weighted tables** (`reward_tables/*.json`, `hq_progress::draw`, SplitMix64 with the roll injected through `Actor.roll` for tests). An entry is any reward kind but another table. The effect carries the entry that came out, so the glue never sees a table | ✅ coded v0.0.6 | V1 |
| Loot / AllTable | not yet | ❓ | V2 |
| Currency | optional bridge to the `smp` economy? (optional `[dependencies]`) | ❓ | V2 |
| Advancement | `native_set_player_advancements` (⏳) or the `advancement grant` command | ⏳ / ✅ | V2 |

## Claim flow
1. The quest is completed: `hq_progress` emits `Effect::QuestCompleted` (notification, sound).
2. For each reward:
   - `auto_claim = Enabled | NoToast | Invisible`: immediate claim;
   - otherwise: the state becomes `CanClaim`, and the player clicks in the book or uses "Claim all".
3. Claiming:
   - `team: true`: only once for the team;
   - `team: false`: once **per player** (each member claims their share).
4. Effects are applied **on the tick, outside the lock**. The `claimed` state is recorded **before** giving (no duplication on crash), and a JSON lines audit log is kept like `smp/audit.log`.

## Security
- Never trust the client: a claim request via net message is **revalidated** on the server (quest completed? reward not yet claimed? player in the team?).
- Click anti-spam (short per-player cooldown).
