---
tags: [adr]
status: proposed   # proposed | accepted | superseded | abandoned
date: 2026-11-10
---
# ADR-011: Repeatable quests start over

## Context
v0.0.6 adds repeatable quests (`"repeatable": true`, `"repeat_cooldown": <seconds>`), like FTB Quests' "repeatable" flag. Several things had to be settled so the engine could be written:
- When does a completed quest become doable again, and what happens to its rewards?
- What do the quests that depend on it see while it is "not completed" again?
- How do we stop a quest from completing itself for ever (a task that stays true: holding an item, a level, a place)?
- The engine is pure and only runs on events: who notices that a cooldown is over?

## Options considered
1. **Reset on claim** (FTB's way: claiming the rewards resets the quest at once). + simple. − no cooldown; a pack cannot make a "daily" quest.
2. **Reset on a timer alone**, claimed or not. + simple. − unclaimed rewards of the last round are lost, or pile up with no record of which round they belong to.
3. **Start over once every reward is claimed *and* the cooldown is over** (chosen). + nothing is lost; a cooldown of 0 gives FTB's behaviour. − a player who never claims never repeats (that is their choice).

## Decision
Option 3, as coded in `hq_progress::start_over`:
- A repeatable quest **starts over** when it is completed, **each of its rewards has been claimed by someone**, and `now >= completed_at + repeat_cooldown`. The cooldown counts **from the completion**, not from the claim. A quest without rewards starts over as soon as the cooldown is over.
- Starting over forgets its tasks, its `started` / `completed` times and its claims, and adds 1 to `Progress.repeats[quest]`. Effect: `QuestStartedOver` (chat: "You can do this quest again: <title>"). The book's panel says "Repeatable, done N times".
- **What depends on it stays open**: dependencies, chapter tallies and visibility count a quest that was *ever* completed (`Progress::ever_completed`: completed now, or `repeats > 0`). Completing it again does not announce its chapter again.
- It happens **once per event, after `settle`**: a quest that starts over cannot complete again in the same event.
- **A repeatable quest needs a task that is used up** each time: a kill, a craft, a checkmark or a hand-in. Otherwise the pack is refused at load (error on the chapter): tasks that only look at what the player has or is (items held, level, place, advancement) would complete it again at once, for ever, giving its rewards in a loop.
- The engine only runs on events, so the glue wakes it: on each look at a player (`detection_delay`), `hq_session::observe` sends a `Refresh` when `hq_progress::watch(...).repeat` says a repeatable quest is completed.
- `/quests reset` forgets the repeats too; `/quests complete` can start a quest over like any event.
- With teams (later): the claims that count are "claimed by someone", so one member claiming is enough for the team's quest to start over; the others lose their share of that round. To revisit with the teams step.

## Consequences
- A random reward (`"type": "random"`) drawn again each round makes repeatable quests worth doing: the example quest "Daily bread".
- `Progress` gained `repeats`; older progress files read fine (the field defaults to empty).
- The cooldown is real time (Unix seconds), so it runs while the server is off. Game-time cooldowns would need the world clock.
- To watch: the per-member claim rule with teams; whether packs want "repeat at most N times".

## Links
- [[Decision index]] · [[Tasks - hook mapping]] · [[Horizon Quests data model]] · [[Quest file format]]
