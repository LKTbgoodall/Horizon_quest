---
tags: [adr]
status: accepted
date: 2026-11-12
---
# ADR-016: Edit mode in the book, saved per world or for the modpack

## Context
Bastien wants to edit quests as in FTB Quests: an operator gets a pencil button in the book that turns on an edit mode where everything can be changed. Questions: who may edit, where the edits are saved, and how the server and the books stay in step.

## Options considered
1. **Edits saved in the world** (`<world>/horizon-quests/quests/`): + already the first place the loader looks, + saved with the world / − a modpack author has to copy files by hand.
2. **Edits saved in `config/`** for every world: + right for a modpack / − one world's test edits leak into every world.
3. **Both** (Bastien's answer, 2026-11-12): edits go to the world; a button copies the pack to the modpack's folder, another puts the world's own copy aside to use the modpack's.

## Decision
Option 3.
- Load order: world folder > **modpack folder** `config/horizon_quests/quests/` > data packs > built-in pack.
- Every edit is a small `hq_edit::Edit` (new, move, duplicate, delete, link in v0.11.0) sent by the book (`ToServer::Edit`). The server applies it to the pack in use, checks the result with the loader (`hq_edit::check`: no error, nothing left out), saves the **whole** pack as the world's folder (`hq_store::save_pack`, chapters numbered `NN_<id>.json`, stale `.json` removed) and sends the new pack to every book. The book applies the same edit to its own copy at once (no jump while the server answers).
- "Save these quests for the modpack" writes the pack in use to the modpack folder. "Use the modpack's quests" renames the world's folder `quests-aside-<unix time>` (never deleted) and reloads.
- Who may edit: op level 2 (`hq_edit::EDIT_LEVEL`, like `/quests reload`) for a player among `native_server_players()`; a player not among them is the world's owner and may edit. `/quests edit` (level 2; the owner's mod commands run at level 4) shows the pencil anyway, as a fallback. `native_player_permission` would be the clean answer but is ⏳ (unnumbered).
- Ids never change once made (progress refers to them); new ids come from the title (`new_quest`, `new_quest_2`).

## Consequences
- An edited pack is saved through serde: the JSON files are complete but more verbose than hand-written ones (`"subtitle": null`...).
- Every edit rewrites every chapter file: fine at pack sizes (tens of chapters).
- Progress for a deleted quest stays in the progress files (they keep unknown ids by design).
- The modpack folder is the instance's; on a dedicated server (later) it is the server's.
- To watch in game: is the single-player owner among `native_server_players()`, and with which level when cheats are off?

## Links
- [[Decision index]] · [[UI - Quest book]] · [[FTB editor and UI]] · [[Persistence and sync]]
