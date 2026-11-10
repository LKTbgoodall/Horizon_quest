---
tags: [horizon, links, reference]
verified_on: 2026-11-10
---
# Links and resources

## Project
- Repository: https://github.com/LKTbgoodall/Horizon_quest

## Horizon: website
- Home: https://playhorizon.dev
- Mod catalog: https://playhorizon.dev/mods
- Submit a mod: https://playhorizon.dev/mods/submit
- Mod policy: https://playhorizon.dev/mods/policy
- Docs: https://playhorizon.dev/docs
  - Make a mod: https://playhorizon.dev/docs/make-a-mod
  - Manifest: https://playhorizon.dev/docs/mod-manifest
  - Mod code: https://playhorizon.dev/docs/mod-code
  - Test: https://playhorizon.dev/docs/test-a-mod
  - Publish: https://playhorizon.dev/docs/publish-a-mod
  - APIs: https://playhorizon.dev/docs/apis
  - Server API: https://playhorizon.dev/docs/api/server
  - Content mods (not read): https://playhorizon.dev/docs/content-mods

## Horizon: GitHub
- Engine: https://github.com/horizon-rust/horizon (MIT)
  - **Hook catalog**: `docs/native-mods.md`, and `native-hooks.json` at the root
  - Hook code: `crates/horizon-client/src/game/native_hooks.rs`
  - Issues: https://github.com/horizon-rust/horizon/issues
- Mod registry and tools: https://github.com/horizon-rust/horizon-mods
  - Server example: `native/smp` → [[Example - smp mod (patterns to reuse)]]
  - Small examples: `native/fps-overlay`, `native/coords-hud`, `native/zoom`, `native/keystrokes`, `native/minimap`, `native/recipes`
  - Tool: `native/tools/native.py`
- Mixins: https://github.com/horizon-rust/horizon-mixins
- Template: https://github.com/horizon-rust/horizon-native-mod-template
- Discord: https://discord.gg/HcTeMZxe5E

## Local copies (scratchpad, temporary)
The `--depth 1` clones of `horizon` (docs only) and `horizon-mods` were made on 2026-11-10 in the Claude session's scratchpad. **They are not persistent**: they will need to be recloned into an `ext/` folder of the project if needed (see [[Roadmap]], phase 0).

## FTB Quests (model)
- CurseForge: https://www.curseforge.com/minecraft/mc-mods/ftb-quests-forge
- Docs: https://docs.feed-the-beast.com/mod-docs/mods/suite/Quests/
- Source: https://github.com/FTBTeam/FTB-Quests (All Rights Reserved: **read to understand, never copy**)
- FTB Teams: https://docs.feed-the-beast.com/mod-docs/mods/suite/Teams/
