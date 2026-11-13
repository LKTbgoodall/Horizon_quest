---
tags: [horizon, publication]
source: playhorizon.dev/docs/publish-a-mod, horizon-mods/MOD-POLICY.md, README.md, mods/*.toml, native/tools/native.py
verified_on: 2026-11-13
---
# Publishing and mod policy

Back: [[Horizon - Overview]]

## Process
1. The source code must be in a **public repository** under a free license (SPDX name: MIT, Apache-2.0, GPL…). For us: https://github.com/LKTbgoodall/Horizon_quest. License to be chosen, see [[Open questions]].
2. `python3 native/tools/native.py package <folder>` produces `native/target/<id>-<version>.hnmod`, then prints the size and the sha256.
3. Form at https://playhorizon.dev/mods/submit (GitHub login): file, id, game version (26.3), Markdown description, authors, license, source URL, PNG icon ≤ 256 KiB (ours: `mod/icon.png`, 256 x 256, 1.3 KB, since v0.6.0).
4. A **PR** is opened automatically on `horizon-rust/horizon-mods`, with `mods/<id>.toml` and `icons/<id>.png`.
5. **Human review**: a maintainer reads **every line** of the `.hnmod`, and **every update** of a native mod is reviewed.
6. Each published version requires a new `version`. « A published file never changes. »

## Our release files (2026-11-11)
- `release/v1-checklist.md`: what remains before 1.0.0.
- `release/registry-entry.md`: the form's fields, summary and description, screenshots, open points.
- **How the content is shipped** (2026-11-11, Horizon `main`): a native registry entry may name a `.hmod` with content; the launcher snapshots the content under the build's `native-content/<id>/`. We publish `build/horizon_quests.hmod` → [[ADR-014 Publish one hmod with content and code]]. The registry compiles native mods on the newest Nightly: re-pin before sending. Its own check: `cargo horizon-mod check <file>`.

## ✅ Native publishing is open (read 2026-11-13)
The submit page itself lists "a native mod" among what it takes (form, checks, pull request, human review); two third-party native entries are already merged (Bioneisme's `big_nukes_native`, `deepbore_quarry_native`). The horizon-mods README sentence saying native publishing waits for `cargo-horizon-mod` and launcher changes is stale, and ewanc26/atmospheric-horizon#28 rests on it. What still holds 1.0.0 back for us is our own horizon-rust/horizon#15933 (a combined `.hmod`'s content not installed).

## Contents of a `.hnmod`
A deterministic zip containing `horizon-native.json` (`{schema:1, id, version, game, horizon, mixins, hooks}`), the `.rs`, `.toml` and `data/` files under `mixins/<id>/`, and the `LICENSE`.

## Registry entry `mods/<id>.toml`
```toml
id = "horizon_quests"        # a-z, 0-9, _
kind = "native-mod"
name = "Horizon Quests"
summary = "…"
description = """markdown"""
authors = ["…"]
owners = ["<github login>"]
owner_ids = [123]
license = "MIT"
source = "https://github.com/LKTbgoodall/Horizon_quest"
icon = "icons/horizon_quests.png"
categories = ["…"]
tags = ["quests", "progression"]
# key = "<minisign pubkey>"   (optional ; once set, all versions must be signed)

[[versions]]
version = "0.1.0"
published = "YYYY-MM-DD"
game = "26.3"
file = "horizon_quests-0.1.0.hnmod"
sha256 = "…"
changelog = "…"
```

## Policy (MOD-POLICY.md), summary
By submitting, you confirm:
1. having the right to publish under the stated license;
2. for a **port**, respecting the original license or having written permission from the authors. ⚠️ FTB Quests is *All Rights Reserved*: we are doing an **inspired reimplementation**, not a port. No FTB code, asset or text may be reused;
3. providing **no Mojang code, assets or data**. The game's data tables (recipes, biomes) are read at runtime from the player's files;
4. that there is nothing harmful (malware, data theft, hidden network access);
5. that the `source` link matches the compiled code exactly.

No cheat clients. Removals go through `revoked.toml`.

## Player-side security
Native mods are **not sandboxed**: the player accepts a red warning and the launcher compiles the game with `--trust-native-code`. A native mod is **compiled per instance**.

## Signing (optional)
```bash
minisign -G -p horizon-mods.pub -s horizon-mods.key
minisign -S -s horizon-mods.key -m native/target/<id>-<version>.hnmod
```

See also: [[Build, test and tools]]
