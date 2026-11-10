---
tags: [project, glossary]
---
# Glossary

| Term | Meaning |
|---|---|
| **Horizon** | Minecraft Java 26.3 rewritten in Rust (client + `horizon-server`). It is not a modloader. → [[Horizon - Overview]] |
| **Native mod** | Rust code **compiled into the game** by the launcher, shipped as `.hnmod`, not sandboxed. → [[Native mods - structure and manifest]] |
| **Content mod** | Data-only mod (blocks, items, recipes, textures) shipped as `.hmod`. |
| **Hook point** | Stable extension point exposed by Horizon (`native_tick`, `native_hud`…). It survives Nightlies. |
| **Injection** | Code inserted into a game function (`HEAD`, `RETURN`, `BEFORE_CALL`, `MODIFY_ARG`, `OVERWRITE`), like Mixins. It forces you to pin `horizon_revision`. |
| **Typed API** | `horizon_client::mod_api`: `Hooks`, `Host`, `Flow` trait. Cleaner than raw hooks. |
| **`horizon-mixin.toml`** | The manifest of a native mod. |
| **`compatibility.json`** | Declares the game version, the hooks API version and the tested commits. |
| **`native.py`** | The `horizon-mods` repo tool to `check`, `build` and `package` a native mod. |
| **Nightly** | Daily Horizon build. The API can change between two Nightlies. |
| **Quest** | A node of the book. It contains tasks and rewards. |
| **Task** | The condition to fulfil (get an item, kill a mob…). |
| **Reward** | What you receive when claiming (items, XP, command…). |
| **Chapter** | A page of the book, with its quest canvas. |
| **Dependency** | A quest A must be completed (or started) before B. |
| **Claim** | Claiming a reward. |
| **Team** | Group of players that shares the same progress. |
| **ADR** | *Architecture Decision Record*: the record of a technical decision. → [[Decision index]] |
