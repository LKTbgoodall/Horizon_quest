---
tags: [adr]
status: accepted
date: 2026-11-10
---
# ADR-008: Graphical client book first

Supersedes the UI part of [[ADR-002 Authoritative server and dual UI]].

## Context
ADR-002 proposed a chest-menu book for the MVP and the FTB-style graphical book later. Bastien chose to **go straight to the graphical book**. The mod targets public release, so an FTB-like experience matters more than vanilla-client support.

## Options
1. Chest book first, then the graphical book (ADR-002 as proposed).
2. **Graphical book directly** (Horizon client with the mod).
3. Chest book only.

## Decision
Option 2. The MVP ships the graphical quest book: a mod screen with a pan/zoom canvas, quest detail panel, claim and submit buttons. It is fed by the authoritative server half through mod messages (`hq_net`). The server stays authoritative (the rest of ADR-002 holds).
A chest-menu book for vanilla clients may come later as an optional fallback; it is not planned.

## Consequences
- + The first playable version already looks and feels like FTB Quests.
- + One UI to build and maintain.
- − Players need Horizon with the mod installed on the client (fine: the mod is published for Horizon players anyway).
- − Longer path to the first in-game test: we need `hq_net` and our own widgets (no widget library, no 2D clip helper documented).
- − **Critical risk to check first (phase 0 spike):** do mod messages work in solo between the window game and the integrated server, and can a mod screen draw and take input as documented?

[[Decision index]] · [[UI - Quest book]] · [[Persistence and sync]]
