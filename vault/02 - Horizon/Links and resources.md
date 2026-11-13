---
tags: [horizon, links, reference]
verified_on: 2026-11-10
---
# Links and resources

## Project
- Repository: https://github.com/LKTbgoodall/Horizon_quest
- Sister project (same author, mob-powered tech mod): `D:\dev\mod_minecraft\cagebound` (Cagebound, ex-Organa)

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
- Registry (the submit form opens a pull request here): https://github.com/horizon-rust/horizon-mods
- **Applied Energistics 2 for Horizon** (a big native mod on the registry that ships its content inside its `.hnmod`): https://github.com/Joeypc6/AE2-Horizon. Model for [[ADR-015 Embed the content pack in the native mod]] (`me_embedded_content.rs`, `tools/make_content_mod.py`). Read to understand; check its license before reusing anything.

## Local copies (`ext/`, gitignored)
Since 2026-11-10: `ext/horizon` (sparse, partial clone at `1453e0e`: `docs/`, `native-hooks.json`, `Cargo.lock`, `crates/horizon-client/src/game/native_*`, `mod_api*`, `hud*`, `extras_hud.rs`) and `ext/horizon-mods` (`--depth 1`). Update with `git -C ext/horizon pull` (set `MSYS_NO_PATHCONV=1` before `git sparse-checkout` in Git Bash, or the paths get mangled). Other files: `git -C ext/horizon show HEAD:<path>`.

## FTB Quests (model)
- CurseForge: https://www.curseforge.com/minecraft/mc-mods/ftb-quests-forge
- Docs: https://docs.feed-the-beast.com/mod-docs/mods/suite/Quests/
- Source: https://github.com/FTBTeam/FTB-Quests (All Rights Reserved: **read to understand, never copy**)
- FTB Teams: https://docs.feed-the-beast.com/mod-docs/mods/suite/Teams/
