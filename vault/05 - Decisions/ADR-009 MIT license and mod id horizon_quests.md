---
tags: [adr]
status: accepted
date: 2026-11-10
---
# ADR-009: MIT license and mod id `horizon_quests`

## Context
Publishing on playhorizon.dev requires a free license (SPDX name) and a permanent mod id (`a-z0-9_`, never changes once published).

## Decision
- License: **MIT**, like Horizon and smp.
- Mod id: **`horizon_quests`**. Display name: **Horizon Quests**.
- Derived names: `mixins/horizon_quests/`, `config/horizon_quests/`, `<world>/horizon-quests/`, net channel `horizon_quests:sync`, registry entry `mods/horizon_quests.toml`.
- The GitHub repository keeps its name `LKTbgoodall/Horizon_quest`.

## Consequences
- + Anyone can reuse the code, which suits a mod published for everyone.
- − The id is final once the first version is published.

[[Decision index]] · [[Publishing and mod policy]]
