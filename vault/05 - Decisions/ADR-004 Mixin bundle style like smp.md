---
tags: [adr]
status: proposed
date: 2026-11-10
---
# ADR-004: Mixin bundle style like smp (not a pure trait mod)

## Context
There are two styles:
1. **trait mod** (`[api]`, `Hooks`, `Host`), cleaner;
2. **mixin bundle** (`[[module]]` + `[[injection]]` + helpers), the `smp` style.

The trait does **not yet have all the hooks**: it lacks `player_command`, keyboard and wheel input for screens, `player_respawned`, `server_data_loaded`… It also lacks some helpers (`sidebar`, `player`).

## Proposed decision
smp-style mixin bundle for the MVP, because it is the most complete server example and is maintained by the Horizon team. Helpers can call the typed API via `&mut Game as &mut dyn Host` when that is simpler. We'll reassess the pure trait once it is complete.

## Consequences
- + We can copy the smp patterns as they are.
- − More `.toml` files and helpers to maintain.

[[Decision index]] · [[Native mods - structure and manifest]]
