---
tags: [horizon, reference, modpack]
source: github.com/horizon-rust/horizon-mods (README, mods/atomic_industry.toml, index.json of 2026-11-17); github.com/Bioneisme/atomic-industry (horizon-modpack.toml); horizon docs/modpacks.md and docs/mod-browser.md on main
verified_on: 2026-11-17
---
# Modpacks on Horizon (`.hpack`)

Back: [[Horizon - Overview]] · [[Publishing and mod policy]]

Two different things are called "modpack" in Horizon:
1. **Importing a Java pack** (`.mrpack`, CurseForge zip): the launcher's `horizon-port-pack` carries the data and assets of the mods it can port. Java code does not come along, so most mods end up "partial" or "not yet" (ATM10: 42 of 496 files ported partly). Not what we would make.
2. **A Horizon pack (`.hpack`, registry kind `modpack`)**: a list of **registry mods** pinned by version and SHA-256, plus `overrides/`. The pack holds no mod file. The registry takes "modpacks of registry mods" only. ✅ This is the one to make.

## The `.hpack` source (from Atomic Industry, the only pack on the registry)
- `horizon-modpack.toml`: `format = 1`, `id`, `name`, `version`, `description`, `authors`, `game = "26.3"`, then one `[[mods]]` per mod with `id`, `version`, `sha256` (of the registry file).
- `overrides/`: files copied into the instance, e.g. `overrides/horizon-mods/.horizon-mod-data/<mod>/...` for mod settings. For us, a pack's quests would go in `overrides/config/horizon_quests/quests/`, the modpack folder of [[ADR-016 Edit mode in the book]] (not yet checked that overrides reach `config/`).
- `icon.png`, `README.md`, `tools/build.py` (builds the `.hpack` zip).
- Registry entry `mods/<id>.toml` with `kind = "modpack"`, categories such as `kitchen-sink`, screenshots, versions with `file = "<id>-<version>.hpack"`.
- The player installs the pack, presses Build and confirms the native-code warning.

## What a pack can contain (registry, 2026-11-17)
- 137 native mods, 12 content mods, 13 shaders, 1 modpack. Most native mods are client tweaks (HUD, zoom, maps, inventory).
- Gameplay content not marked broken on the newest Nightly: Applied Energistics 2, ProjectE (`projecte` + `projecte_core`), Silent's Gems, Voltz (`voltz` + `voltz_core`), Deepbore Quarry, Big Nukes, Wayfarer Stones, Inferium Lab, Tectonic, Veinminer, Salvage, Additional Lights, Factory Blocks, Macaw's Trapdoors, Nether Star recipe, Flesh to Leather. Utilities: JEI / REI / Recipes, Jade, AppleSkin, maps, Mouse Tweaks.
- Marked broken (`broken_on`): Create, Orbital, Atlas, Treefall, Chop, Carry Block, WorldEdit, Distant Horizons, Shulker Box Tooltip, Inventory Wizard and others.

## Caveats
- Every native mod listed is in the **old** format. Once the first Nightly with the mod API ships ([[ADR-018 Port to the mod API]]), each one needs its author's port before a pack can pin it on that Nightly.
- A pack can only list published mods, so a pack built around Horizon Quests waits for our 1.0.0.
