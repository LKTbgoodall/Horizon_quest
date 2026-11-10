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
| **Phase** | Phase 0 ✅ (2026-11-10); phase 1 done; **phase 2 in progress**: step 1 (server half) coded as v0.0.2, **waiting for the in-game test** → [[Roadmap]] |
| **Last session** | [[2026-11-10]]: research, vault, pure core, first in-game test (all 4 spikes pass), then phase 2 step 1: the server half (`hq_session` + `hq_server_glue`) |
| **Code** | `mod/` v0.0.2. Every module is in the bundle now; the server glue (engine, inventories, rewards, saving, `/quests`) is **not compiled yet** (Bastien's build is its first compile). `allow(dead_code)` marks on items the glue does not call yet. **108 tests pass** |
| **Key choices** | `horizon_quests`, MIT, graphical book first, solo/LAN first, English-only in-game text, published for everyone |
| **Blocking** | The in-game test of v0.0.2 |

## ➡️ Next step
1. Bastien: `git pull`, `tools/build-mod.ps1`, then the checklist in `mod/README.md` § Test in game (v0.0.2: the server half)
2. Fix what the build shows (`native/build.log`): the server glue is compiled for the first time there
3. Phase 2 step 2: net sync (`hq_net`), server → book
4. Phase 2 step 3: the real book (`hq_book` replaces the test book), then remove the `allow(dead_code)` marks
5. Bastien: answer the design questions in [[Open questions]] § Decisions to make

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
