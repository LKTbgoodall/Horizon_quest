---
tags: [horizon, reference]
source: playhorizon.dev (/, /docs, /mods, /docs/make-a-mod, /docs/mod-code, /docs/apis, /docs/publish-a-mod, /docs/api/server), github.com/horizon-rust/horizon/docs
verified_on: 2026-11-10
---
# Horizon: overview

> « Horizon is Minecraft: Java Edition 26.3 rewritten in Rust. » It is installed through a small launcher and runs on the game files of **your own** copy of Minecraft.

## What it is (and what it is not)
- It is a **clean-room reimplementation** in Rust of the client, the game logic and a dedicated server (`horizon-server`). Parity with vanilla is checked by 281 "harnesses".
- It is **NOT a modloader**: Java jars (Forge, Fabric, NeoForge) do not load and Java Mixins do not apply. **FTB Quests therefore cannot run** on it, a rewrite is needed.
- Game version: **Java Edition 26.3 only**. Worlds from 1.8 to 26.2 are upgraded.
- Platforms: Linux x86-64 (Vulkan), macOS 12+ (Apple Silicon or Intel), **Windows x64 or ARM64** (Vulkan or DX12).
- Horizon license: **MIT**. It contains no Minecraft code or assets. It is not an official product.
- Launcher version seen on 2026-11-10: 0.2.58.

## Mod types
| Type | File | Content | For us |
|---|---|---|---|
| **Content mod** | `.hmod` | data-only: blocks, items, recipes, loot tables, textures, lang (`horizon-mod.toml`) | useful for **assets** (icons, book textures, "quest book" item) |
| **Native mod** | `.hnmod` | **Rust code compiled into the game** (`horizon-mixin.toml`), not sandboxed | ✅ **the core of Horizon Quests** |
| Combined | `.hmod` with `horizon-native.json` + `mixins/<id>/` + `assets/` | code and assets | 🎯 the final target (code + textures + lang) |
| Shader, resource pack | — | — | — |

Details: [[Native mods - structure and manifest]]

## Client and server sides
- A single binary contains several `Game`s: the window, the singleplayer integrated server and the dedicated server. Each hook declares its side (`server`, `client` or `both`).
- **Singleplayer / LAN host**: mods run in the host's game **and** in its integrated server.
- **Joining a server**: only client hooks act.
- **Dedicated server**: an *engine export* compiled with `--server-output` is required (see [[Build, test and tools]]). ⚠️ The public docs still list it as "Not there yet". There is **no compatibility handshake yet** (tracked in #6304).
- Native mods are **not** part of the mod check at connection: our mod will need to do its **own handshake** via mod messages.

Details: [[Horizon hooks and API]] · [[Persistence and sync]]

## Stability
| API | Level |
|---|---|
| Native mod API | **experimental** |
| `/api/v1` server control API | stable |
| Shaders | planned |

**Hook points** are only ever added and their contracts are frozen (a change produces a `_v2`). **Injections on raw functions** can break on every commit. → We use **hook points only** whenever possible ([[ADR-003 Hook points over raw injections]]).

## Ecosystem
- The catalog https://playhorizon.dev/mods has 158 projects (133 native mods) as of 2026-11-10. **No quest mod.**
- Notable ported mods: JEI, Create, AE2, Jade… There is a `native_show_recipes` to open the installed recipe viewer from **our** book.
- Reference server mod: `smp` → [[Example - smp mod (patterns to reuse)]].

Links: [[Links and resources]]
