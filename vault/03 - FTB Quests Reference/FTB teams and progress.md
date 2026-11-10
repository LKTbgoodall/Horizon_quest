---
tags: [reference, ftb-quests, teams]
source: ServerQuestFile.java, Navigating docs, FTB Teams docs
verified_on: 2026-11-10
---
# Teams and progress (FTB Quests + FTB Teams)

Back: [[FTB Quests - Overview]]

## Principle
- Progress is stored **per team** (`TeamData`, indexed by the team UUID). A solo player has an implicit *personal team*.
- A task completed by **any member** is completed for the whole team.

## Team events
| Situation | Effect on progress |
|---|---|
| Creating a party | the creator's personal team progress is **copied** into the party (`copyData`) |
| Joining an existing party | the player's progress is **merged** into the party (`mergeData`), then auto-claims are checked |
| Leaving the party | the player **gets back** their old personal progress; rewards already claimed stay claimed (`mergeClaimedRewards`) |

## FTB Teams (for reference)
- Team types: personal, party, server (not joinable).
- Commands: `/ftbteams list`, `/ftbteams party …` (create, join, invite, leave…), `/ftbteams msg`, `/ftbteams info`, `/ftbteams server <create|delete|settings>` (OP), `/ftbteams force-disband` (OP).
- Party: invitations, open or invite-only, team chat, allies.

## Permissions and states
- Editor permission: `ftbquests.editor`. Edit mode is per player and turns off if the permission is lost.
- `locked`: the player can no longer open the book or complete quests.

> [!question] For us
> Horizon has no equivalent of FTB Teams. We will need either a **minimal team system built into** the mod, or a dependency on another mod. See [[Open questions]] and [[Persistence and sync]].
