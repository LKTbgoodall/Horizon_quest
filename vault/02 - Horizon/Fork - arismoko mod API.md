---
tags: [horizon, reference, fork]
source: github.com/arismoko/horizon (README, CLAUDE.md, docs/modding/, crates/horizon-mod-vocab) at c26fe59; Discord announcement 2026-11-13
verified_on: 2026-11-13
---
# Fork: arismoko/horizon (the mod API fork)

Back: [[Horizon - Overview]] · [[Horizon hooks and API]]

**Not the official Horizon.** A fork of `horizon-rust/horizon` where the mod API is built: `mod_api` (`horizon-mod-api`), the `Host`, handles, Parts, attachments, native and WebAssembly mods. Upstream's main is merged in regularly. Its own launcher, Nightlies, settings and accounts; its own Discord ("Modrizon"); mostly maintained by agents. Announced 2026-11-13: it wants mod developers, testers (logs) and contributors (`help wanted` issues).

## How it differs from what we build on
- **Clean break**: no compatibility paths; old mods rebuild and follow what moved. The API changes under you.
- **No `native-hooks.json` catalogue** and no `native_*` hook points: of the 75 `native_*` names our bundle uses, the client/server ones (`native_mod_screen*`, `native_net_*`, `native_hud`, `native_tick`, reload listeners, configs, inventory helpers...) are gone. Hooks are `mod_api::Hooks` methods generated from `crates/horizon-mod-vocab/src/hooks/` (700 files) with a typed `Host` (`ClientHost` / `ServerHost`).
- One package, `.hmod` (manifest `horizon-mod.toml`, `assets/`, `data/`, Rust source or `mod.wasm`). Native mods compile against a prebuilt SDK; the launcher **never rebuilds the game**. A "native route" (`[[native.mixin]]`, `[[native.bridge]]`, `[[native.module]]`) still uses horizon-mixins for game internals, tied to one exact build.

## What it has that upstream lacks for us (2026-11-13)
- ✅ Clipboard: `ClientHost::clipboard_text()` / `set_clipboard_text()` (paste in the editor).
- ✅ `player.permission_level(host)` (who may edit; upstream's `native_player_permission` is ⏳).
- ✅ Mod screens with game widgets, including a `TextField` drawn as vanilla's `EditBox` (`mod_api::client::widgets`); screen hooks for keys, chars, mouse, scroll, drag.
- ✅ Payload channels (`add_payload`, `add_client_payload_handler`), commands, server reload listeners.

## What a port would cost
- The pure modules (`hq_*.rs`, ~15,600 lines) move as they are into the mod's crate.
- The glue (`hq_client_glue.rs`, `hq_server_glue.rs`, ~1,700 lines) and the ~30 hook files are rewritten against `mod_api`, and the book's drawing goes through `DrawContext`.
- Publishing would be on the fork's side, not playhorizon.dev's registry, unless upstream adopts the API.

## Our position
- Stay on upstream for 1.0.0 ([[ADR-014 Publish one hmod with content and code]]). Revisit if upstream merges the fork's API → [[Open questions]].
