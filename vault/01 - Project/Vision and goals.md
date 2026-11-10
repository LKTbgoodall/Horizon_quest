---
tags: [project]
date: 2026-11-10
updated: 2026-11-10
---
# Vision and goals

## The project
**Horizon Quests** (mod id `horizon_quests`, MIT): a quest mod for **[[Horizon - Overview|Horizon]]**, the Rust reimplementation of Minecraft Java 26.3. It is heavily inspired by **[[FTB Quests - Overview|FTB Quests]]**: a graphical quest book with chapters, quests linked by dependencies, tasks and rewards, and team-based progress.

- Repository: https://github.com/LKTbgoodall/Horizon_quest
- Local folder (dev PC): `C:\Users\bastieng.BE3\Desktop\Bastien\autre\Horizon_quest`
- No quest mod exists in the Horizon catalog (158 projects as of 2026-11-10): **the field is open**.
- Audience: **published for everyone** on playhorizon.dev.

## Why
- Modpacks need a common thread, and FTB Quests is the standard on Forge and NeoForge.
- Java mods do not run on Horizon: a native reimplementation is needed.

## Goals
1. **Functional parity** with the core of FTB Quests (see [[Roadmap]] for the MVP, V1 and V2 breakdown).
2. **An FTB-like graphical book from the first version** ([[ADR-008 Graphical client book first]]).
3. **Open, readable quest format**: one file per chapter, clean git diffs.
4. **Authoritative server**: progress is stored and validated server-side. Solo/LAN comes first; dedicated servers later.
5. **Publication** on the official catalog (https://playhorizon.dev/mods) under MIT (see [[Publishing and mod policy]]).

## Ways to do better than FTB
- A more discoverable editor, with fewer menus hidden behind right-click.
- **FTB Quests import** (JSON5/SNBT → our format) to port existing modpacks, planned for phase 5. This would be a big adoption argument.
- A public API for other Horizon mods (custom tasks and rewards, like KubeJS for FTB).
- No mandatory dependency on a third-party teams mod: teams are built in (V1).

## Non-goals (for now)
- Energy or fluid tasks (there is no equivalent tech ecosystem).
- Binary compatibility with FTB Quests: we aim for **import** of data, not code.
- Vanilla-client support: the book needs Horizon with the mod.
- Translations: in-game text is English-only until phase 4.

## Constraints
- FTB Quests is licensed *All Rights Reserved*: we reuse **neither code nor assets**, only concepts.
- Horizon is young and its native mod API is marked **experimental** (see [[Horizon hooks and API]]).
- In-game tests only happen on Bastien's home PC (Windows); the dev PC has no game. See [[Build, test and tools]] § Two-PC workflow.
