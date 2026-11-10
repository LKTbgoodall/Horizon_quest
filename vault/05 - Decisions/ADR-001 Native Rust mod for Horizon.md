---
tags: [adr]
status: accepted
date: 2026-11-10
---
# ADR-001: Native Rust mod for Horizon

## Context
We want an FTB Quests on Horizon. But Horizon is a rewrite of Minecraft in Rust: Java jars and Mixins do not work. Content mods (`.hmod`) are data-only.

## Options
1. Port FTB Quests → impossible: no JVM, and the license is All Rights Reserved.
2. Content-only mod (advancements, datapacks) → far too limited (no GUI, no team progress).
3. **Native Rust mod** (`.hnmod`, then a combined `.hmod` for the assets).

## Decision
Option 3: a native mod in Rust, reimplemented **from scratch** and inspired by the concepts of FTB Quests, with no FTB code or assets.

## Consequences
- + Everything has to be written, but we control the design.
- − The native API is **experimental**: we'll need to follow the Nightlies (see [[ADR-003 Hook points over raw injections]]).
- − The code is reviewed line by line by the maintainers for every published version.

[[Decision index]]
