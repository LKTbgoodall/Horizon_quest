---
tags: [adr]
status: proposed
date: 2026-11-10
---
# ADR-002: Authoritative server and dual UI

## Context
- Progress must be reliable (anti-cheat) and shared across a team, so it is handled server-side.
- Mod screens (`native_mod_screen_*`) and mod messages exist **only for Horizon clients** with the mod installed.
- **Chest menus** (`native_open_menu`) work with **any client** and are already proven in `smp`.

## Options
1. Client only (local progress) → easy to cheat, no multiplayer.
2. Server and graphical client screen only → a big GUI effort before anything is playable.
3. **Authoritative server + chest book (MVP), then graphical client screen (V2)**.

## Proposed decision
Option 3. All the logic and data are server-side. The MVP is played with chest menus, the sidebar and chat. The graphical book comes afterwards and reads the same data via `hq_net`.

## Consequences
- + A quickly playable MVP, compatible with all clients.
- + The logic is testable without a GUI.
- − The chest book is not as pretty as FTB (no canvas).
- − Two interfaces to maintain (but a single logic, `hq_logic`).

See [[Architecture]] · [[UI - Quest book]] · [[Decision index]]
