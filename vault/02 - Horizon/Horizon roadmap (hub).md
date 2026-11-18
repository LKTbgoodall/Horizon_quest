---
tags: [horizon, reference, roadmap]
source: https://playhorizon.dev/hub#roadmap (generated from roadmap/roadmap.toml, ROADMAP.md in horizon-rust/horizon)
verified_on: 2026-11-18
---
# Horizon roadmap (hub)

Back: [[Horizon - Overview]] · [[Modpacks on Horizon (hpack)]] · [[Publishing and mod policy]]

The project's public roadmap, read 2026-11-18. No dates: items are **Now**, **Next** or **Later** (97 items, in 13 themes). Pull requests carry a `roadmap:<id>` label. Below, only what touches Horizon Quests; the page has the rest (performance, rendering, platforms...).

## What touches us
| When | Item | What it means for us |
|---|---|---|
| Now | **Mods** (one line of work led by the modding team: native mods built by the launcher, mods on servers, mod screens and settings, tools, registry) | The mod API we are on (v0.22.0) is still moving: expect new hooks and fixes; ask for missing ones with an issue titled `mod API: ...` |
| Now | **Single player as a server: the last differences** | Our port already treats the single-player host as an ordinary player (one path for hand-in, place, levels); fixes there help us for free |
| Next | **One single-player path: remove the in-frame world** | Same: confirms we were right to drop the host workarounds |
| Next | Server console, bans, admin API (and MCP) | Nothing to do; our `/quests` commands should work from a console once it exists (test then) |
| Later | **Whole modpacks, such as ATM10, rewritten** (not a priority) | ATM10's progression is FTB Quests. A rewritten ATM-style pack needs a quest book: our roadmap's **FTB Quests import** (phase 5) would let such a pack bring its quests over. See [[Open questions]] |
| Later | **Mods with code: machines, magic, mobs** (machines first) | More item ids in packs; our item, craft and kill tasks take any id, nothing to change |
| Later | **Data packs in the mod browser** | Our quests can already come from data packs (`data/<ns>/horizon_quests/`): a quest pack could then be installed from the launcher |
| Later | **Scripts for mods** (KubeJS style, no compiler) | Pack makers script with KubeJS today; could one day drive quest rewards or tasks. Nothing to do now |
| Later | Native versions of popular client mods (block info, world map, inventory tweaks, schematics) | **No quest book in the list**: Horizon Quests has no official counterpart planned |
| Later | Port mods with your own AI agent | Our project is an example of that |
| Later | Server hosting (one-command install, Docker, panels) | Our roadmap's dedicated-server support (phase 4) |

## Not for us
Performance, rendering and shaders, platforms, launcher design, gameplay parity lists, video studio, AI teammate: no effect on the mod.
