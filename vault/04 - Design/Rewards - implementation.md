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
| **Command** | `native_run_command(line)` on the tick (level 4, console), with `{player}` and `{uuid}` replaced. Later `native_run_command_as` (⏳) | ✅ | V1 |
| **Toast** | Horizon client: net message → `native_add_toast` (⏳). Vanilla client: injected advancement (`native_data_pack_files`) + `native_trigger_criterion` (69), or a title or an action bar | ⏳ / ✅ | V1 |
| **Choice** | opens a chest menu listing the table, a click picks | ✅ | V1 |
| **Random / Loot / AllTable** | **our own weighted tables** (`hq_rewards`, injected RNG for tests) → Item | ✅ | V1/V2 |
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
