---
tags: [adr]
status: accepted (UI part superseded by ADR-008)
date: 2026-11-10
---
# ADR-002: Authoritative server and dual UI

> [!warning] Partly superseded
> The **authoritative server** part is accepted. The **UI order** (chest book first) is superseded by [[ADR-008 Graphical client book first]].

## Context
- Progress must be reliable (anti-cheat) and shared across a team, so it is handled server-side.
- Mod screens (`native_mod_screen_*`) and mod messages exist **only for Horizon clients** with the mod installed.
- **Chest menus** (`native_open_menu`) work with **any client** and are already proven in `smp`.

## Options
1. Client only (local progress) → easy to cheat, no multiplayer.
2. Server and graphical client screen only → a big GUI effort before anything is playable.
3. **Authoritative server + chest book (MVP), then graphical client screen (V2)**.

## Decision
- ✅ All the logic and data are server-side; clients only display and send requests that the server re-validates.
- ❌ (superseded) ~~The MVP is played with chest menus, the sidebar and chat; the graphical book comes afterwards.~~ → Option 2 was chosen instead, see ADR-008.

## Consequences
- + The logic is testable without a GUI.
- + Works the same in solo (integrated server) and multiplayer.

See [[Architecture]] · [[UI - Quest book]] · [[Decision index]]
