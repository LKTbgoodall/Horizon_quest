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
| **Phase** | Phase 0 ✅ (2026-11-10); phase 1 done; **phase 2 in progress**: steps 1, 2 & 3 (server engine, net sync, graphical book) coded, review fixes in v0.0.4, config file in v0.0.5: **everything that needs no game is done, waiting for the in-game test (solo + LAN)** → [[Roadmap]] |
| **Last session** | [[2026-11-10]] § Session 13: **v0.0.6 finished and merged into `main`** (glue, manifest, client toasts, 25 new tests). Found and fixed a build break of v0.0.4 / v0.0.5 (`native_as_player` is private at the pin). New `tools/check-glue.ps1` type-checks the glue against the real Horizon source: **clean**. **133 + 2 tests pass** |
| **Code** | `mod/` v0.0.6. Full client/server quest engine with real graphical book, net sync over `horizon_quests:book` (protocol 2), inventory polling, 9 task types, 7 reward types, reward tables, repeatable quests, tasks in order, HUD toasts, saving, `/quests`. Two example chapters |
| **Key choices** | `horizon_quests`, MIT, graphical book first, solo/LAN first, English-only in-game text, published for everyone |
| **Blocking** | Nothing known: the in-game test of v0.0.6 (solo, then LAN). **v0.0.4 and v0.0.5 never compiled: do not test them.** Watch the single-player host's inventory and place reads, and `native_player_logged_in` for the host ([[Open questions]]) |
| **In progress** | Nothing half done. Next steps of phase 3, one version each, after the in-game test: teams, more config, datapack quests, book polish, assets |

## ➡️ Next step
0. Claude, before any in-game test request: `powershell -ExecutionPolicy Bypass -File tools\check-glue.ps1` must end with "The glue type-checks" ([[Build, test and tools]])
1. Bastien: `git pull`, `powershell -ExecutionPolicy Bypass -File tools\build-mod.ps1`, then the checklist in `mod/README.md` § Test in game (v0.0.6: solo, config file, the "Further afield" chapter, then LAN)
2. Verify in-game: book opening with K, pan/zoom, chapter sidebar, quest panel, checkmarks, item tasks, claims, and atomic saving
3. Fix what the build shows if any (`native/build.log`)
4. If every box is ticked: phase 2 done, release 0.1.0 (private testing), then the rest of phase 3 (teams first)
5. Bastien: accept or change [[ADR-011 Repeatable quests start over]]

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
