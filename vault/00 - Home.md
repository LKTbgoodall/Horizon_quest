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
| **Phase** | Phase 1 (pure core) **done as far as it goes without the game**; phase 0 spikes written and still **waiting for the first in-game test** → [[Roadmap]] |
| **Last session** | [[2026-11-10]]: research, vault, decisions, mod skeleton + 4 spikes; then the whole pure core: model, loading and validation, logic, engine, store, net protocol, book layout, commands, example pack |
| **Code** | `mod/` v0.0.1. In the bundle: the 4 spikes (glue never compiled yet, needs the home PC build). Outside the bundle for now (`src/lib.rs` only): `hq_model`, `hq_validate`, `hq_logic`, `hq_progress`, `hq_store`, `hq_net`, `hq_book`, `hq_commands`. **98 tests pass** |
| **Key choices** | `horizon_quests`, MIT, graphical book first, solo/LAN first, English-only in-game text, published for everyone |
| **Blocking** | **the home PC test**. Everything left is glue, and no glue has been compiled yet. Spike 3 (mod messages in solo) decides whether ADR-008 holds |

## ➡️ Next step
1. Bastien: `git pull` on the home PC, then the checklist in `mod/README.md` § Test in game (unchanged by phase 1)
2. Bastien: answer the design questions in [[Open questions]] § Decisions to make
3. Fix what the build or the test shows (`native/build.log`, screenshots, footer text of the book)
4. Phase 2: server glue (state, load, tick, events, save) → net sync → the real book (`hq_book` replaces the spike's drawing) → commands → [[Roadmap]]

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
