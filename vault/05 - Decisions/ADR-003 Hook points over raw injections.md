---
tags: [adr]
status: superseded
date: 2026-11-10
---
# ADR-003: Hook points over raw injections

> ⚠️ **Superseded by [[ADR-018 Port to the mod API]]** (accepted 2026-11-16), effective when the port to `mod_api` lands. It still governs the current code in the old format until then.

## Context
- The catalog's **hook points** (`native-hooks.json`) are only ever added to, and their contracts are frozen. A mod that only uses them compiles on every Horizon version ≥ `hooks`.
- An injection on a raw `target` can break at every commit, and forces pinning `horizon_revision`.

## Decision
Use **only hook points** (with `hook` + `hook_version`, or the `Game::native_*` targets like smp). A raw function is injected only as a last resort, with a dedicated ADR.
**⏳ pending** hooks (e.g. `player_inventory_changed`) are not used until they have a number. We rely on fallbacks (polling).

## Consequences
- + Horizon updates are painless.
- − Some features wait on hooks (biome, structure, toasts): see [[Open questions]].

[[Decision index]]
