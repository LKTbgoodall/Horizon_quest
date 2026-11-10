---
tags: [home, moc]
updated: 2026-11-10
---
# 🧭 Horizon Quests: home

> **An FTB Quests-style quest mod for Horizon** (Minecraft Java 26.3 rewritten in Rust).
> This vault is the project's "second brain". Everything that matters is here, so nobody has to reread old conversations.

## 📍 Where we are
| | |
|---|---|
| **Phase** | Phase 0 ✅ (2026-11-10); phase 1 done; **phase 2 in progress**: steps 1, 2 & 3 (server engine, net sync, graphical book) coded, review fixes in v0.0.4, **waiting for the in-game test** → [[Roadmap]] |
| **Last session** | [[2026-11-10]] § Session 10: v0.0.4 fixes every finding of the v0.0.3 review (dead spike code removed, claims keyed by UUID in the book, book hand-ins through `native_as_player`, sync apart from saving, `Hello` flood, "no answer" loading line, ping/pong removed). **105 tests pass**, glue control compile clean |
| **Code** | `mod/` v0.0.4. Full client/server quest engine with real graphical book, net sync over `horizon_quests:book`, inventory polling, rewards, saving, `/quests` |
| **Key choices** | `horizon_quests`, MIT, graphical book first, solo/LAN first, English-only in-game text, published for everyone |
| **Blocking** | Nothing known: the in-game test of v0.0.4. Watch the single-player host's inventory reads and `native_player_logged_in` for the host ([[Open questions]]) |

## ➡️ Next step
1. Bastien: `git pull`, `powershell -ExecutionPolicy Bypass -File tools\build-mod.ps1`, then the checklist in `mod/README.md` § Test in game (v0.0.4)
2. Verify in-game: book opening with K, pan/zoom, chapter sidebar, quest panel, checkmarks, item tasks, claims, and atomic saving
3. Fix what the build shows if any (`native/build.log`)
4. Release 0.1.0 preparation (polishing, settings toggle)

## 🗺️ Vault map
### 01 · Project
- [[Vision and goals]]: why, what, non-goals
- [[Roadmap]]: phases 0 to 5 with checkboxes
- [[Open questions]]: things to decide or check
- [[Glossary]]

### 02 · Horizon (the platform)
- [[Horizon - Overview]]: what Horizon is, mod types, client and server
- [[Native mods - structure and manifest]]: `horizon-mixin.toml`, `compatibility.json`, allowed crates
- [[Horizon hooks and API]]: ⭐ **the API reference** (networking, events, rewards, persistence, GUI, commands)
- [[Example - smp mod (patterns to reuse)]]: ⭐ **the model to copy**
- [[Build, test and tools]]: launcher, `native.py`, tests, Windows pitfalls
- [[Publishing and mod policy]]
- [[Links and resources]]

### 03 · FTB Quests reference (the functional model)
- [[FTB Quests - Overview]]
- [[FTB data model]] · [[FTB task types]] · [[FTB reward types]]
- [[FTB teams and progress]] · [[FTB editor and UI]] · [[FTB file format]] · [[FTB commands]]

### 04 · Design (our mod)
- [[Architecture]]: ⭐ events → pure rules → effects
- [[Horizon Quests data model]]
- [[Tasks - hook mapping]] · [[Rewards - implementation]]
- [[UI - Quest book]]: the graphical book (MVP)
- [[Persistence and sync]] · [[Quest file format]] · [[Commands and permissions]]

### 05 · Decisions
- [[Decision index]]

### 06 · Journal
- One note per session, named `YYYY-MM-DD` (template: `99 - Templates/Template - Journal`)

## 🔁 Session routine
**Start**: read this page → the latest journal note → the current phase in the [[Roadmap]].
**End**:
1. Create or update today's note in `06 - Journal`.
2. Tick off the [[Roadmap]].
3. Update "Where we are" and "Next step" above.
4. Add questions to [[Open questions]] and record decisions in an ADR.
5. If something about Horizon changed, update the reference note and its `verified_on`.

## 📏 Conventions
- Every project file is written in **English**: notes, code, comments, commits. (Chat with Bastien is in French.)
- Frontmatter: `tags`, `status`, `date` / `verified_on`, and `source` for reference notes.
- API status: ✅ available · ⏳ pending (not numbered yet) · ❌ does not exist · ❓ unknown.
- Priorities: 🔴 MVP · 🟠 V1 · 🟡 V2 · ⚪ later.
- Quick notes go in `00 - Inbox`, to be sorted later.
