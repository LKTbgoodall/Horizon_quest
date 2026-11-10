---
tags: [adr]
status: proposed
date: 2026-11-10
---
# ADR-006: Quest definitions as JSON, one file per chapter

## Context
FTB stores one file per chapter (SNBT, then JSON5). This gives readable git diffs and lets several people work at once. `serde_json` is in the Horizon lock.

## Proposed decision
Definitions go in `config/horizon_quest/quests/`: `quests.json` plus `chapters/*.json`, and later `reward_tables/` and `lang/`. We use strict JSON. Datapack loading (`NativeReloadListener::json`) will come in V1.

## Consequences
- + Hand-editable from the MVP, before having an in-game editor.
- + An FTB import remains possible (JSON5/SNBT → JSON conversion).
- − No comments in strict JSON.

[[Decision index]] · [[Quest file format]]
