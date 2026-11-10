---
tags: [adr]
status: proposed   # proposed | accepted | superseded | abandoned
date: 2026-11-10
---
# ADR-012: Parties share one progress

## Context
Phase 3 (V1) asks for teams. Horizon has no team system of its own and no FTB Teams, so the mod needs a small one built in ([[FTB teams and progress]]). It has to work in solo and LAN today, keep every existing progress file valid, and never lose a player's progress.

## Options considered
1. **Per-player progress, a party only shows the others' progress**: + nothing changes on disk / − not what FTB players expect: a task done by a friend does not count.
2. **One progress per team, FTB's rules (copy, merge, keep claims)**: + what FTB Quests players know; a lone player is just a team of one / − a quest done in a party is not in your own progress after you leave.
3. **One progress per team, the party's progress copied back to each member on leave**: + nothing is lost on leave / − a player could join a party, let it finish everything, leave with it all: progress becomes free.

## Decision
Option 2, reimplemented (no FTB code):
- A player in no party is **their own team**, whose id is their UUID string: `progress/<uuid>.json` is the same file as before teams.
- A party has an id `party-<16 hex>`, a name players type (1 to 16 letters, digits, `_`, `-`, unique ignoring case), an owner, members and invitations, kept in `<world>/horizon-quests/teams.json`; its progress is `progress/<party id>.json`.
- **Create**: the creator's progress so far is copied into the party.
- **Join** (needs an invitation from the owner): what the joiner did is merged into the party's progress (a quest or task done by either side is done; task counts take the higher; claims add up; the earliest time is kept).
- **Leave** (or kicked by the owner): the player gets back the personal progress they had before joining (it was never touched), plus every reward they claimed in the party, so nothing is claimed twice. When the owner leaves, the member with the lowest id owns the party; the last one out disbands it (its progress file stays on disk).
- **Inside a party**: a task done by any member counts for the party. An item task counts the member who holds the most, an experience task the highest level. **Rewards are claimed by each player for themselves** (FTB's default; no team-wide reward flag yet). Quest and chapter completions are told to every member on, and their automatic rewards are given at once.
- An unreadable `teams.json` turns party commands off and is never overwritten (same rule as progress files).

## Consequences
- The engine (`hq_progress`) did not change: it already worked on "a team's progress" with claims per player. The change is in `hq_session` (progress keyed by team) and the new `hq_teams`.
- A disbanded party's file stays in `progress/`: harmless, and it keeps the data if someone wants it back.
- Not done yet: admin commands for parties (force-disband, move a player), a team-wide reward option, showing the party in the book, party chat.
- To test in LAN: see `mod/README.md` § Test in game, "Parties".

## Links
- [[Decision index]]
- [[FTB teams and progress]]
- [[Horizon Quests data model]]
- [[Persistence and sync]]
