# The registry entry for playhorizon.dev (draft for 1.0.0)

What to paste into the form at https://playhorizon.dev/mods/submit when 1.0.0 is ready. Bastien sends it with his GitHub account; the form opens a pull request on `horizon-rust/horizon-mods` with `mods/horizon_quests.toml` and `icons/horizon_quests.png`, and a maintainer reviews every line of the `.hnmod`.

## Form fields

| Field | Value |
|---|---|
| File | **`build/horizon_quests.hmod`** from `tools/build-mod.ps1`: content and code in one file ([ADR-014](../vault/05%20-%20Decisions/ADR-014%20Publish%20one%20hmod%20with%20content%20and%20code.md)) |
| Id | `horizon_quests` |
| Game version | `26.3` |
| Name | Horizon Quests |
| Authors | LKTbgoodall |
| License | MIT |
| Source | https://github.com/LKTbgoodall/Horizon_quest |
| Icon | `mod/icon.png` (256 x 256, 1.3 KB) |
| Tags | `quests`, `progression`, `questbook`, `adventure` |

## Summary (one line)

A quest book in the spirit of FTB Quests: chapters, quests, tasks, rewards and parties, with a graphical book.

## Description (Markdown)

```markdown
**Horizon Quests** adds a quest book to Horizon, in the spirit of FTB Quests, written from scratch for Horizon.

- **A graphical quest book**: open it with K or by right-clicking the Quest Book every player gets. Chapters, quests linked by dependencies, lines that show what each quest needs and opens.
- **9 task types**: items to hold or hand in, crafts, kills, checkmarks, experience levels, dimensions, places, advancements.
- **7 reward types**: items, experience, levels, commands, toasts, random rewards and rewards to choose.
- **Parties** that share their progress (`/quests team`), with rewards per player or for the whole party.
- **A tracked quest** on your screen, **recipes** of the items a quest asks for, **repeatable** and **optional** quests.
- **Your own quests**: JSON files in the world's folder or in a data pack, checked on load. Guide: https://github.com/LKTbgoodall/Horizon_quest/blob/main/docs/making-quests.md
- Comes with two example chapters, which a server can turn off.

Solo and LAN. Every player needs the mod. Dedicated servers are planned. Needs a launcher that reads native `.hmod` files (a recent one).
```

## Screenshots (taken 2026-11-11, in `docs/images/`, shown in the README)

1. `book-overview.png`: the book on "Getting started".
2. `book-hover.png`: a quest under the pointer (cyan and yellow lines, its title).
3. `quest-window.png`: a quest window with a task done and a reward waiting ("!").
4. `choice-window.png`: the choice window (taken before 0.10.7 made it opaque).
5. `chapter-list.png`: the chapter list, names green / blue / grey, a "!".
6. `tracked-quest.png`: the tracked quest in the top-left corner while playing.

For the form's description, link them from the GitHub repository (`https://raw.githubusercontent.com/LKTbgoodall/Horizon_quest/main/docs/images/<file>`) if it takes images.

## Open points before sending

- **Which file: settled** ([ADR-014](../vault/05%20-%20Decisions/ADR-014%20Publish%20one%20hmod%20with%20content%20and%20code.md)). Horizon `main` now says native registry entries may name a `.hmod` "optionally with content": the launcher installs it, snapshots the content under the build's `native-content/<id>/` and the game loads it. Ours already follows the file's rules (top level: `horizon-mod.toml`, `assets/`, `horizon-native.json`, `mixins/`; one id). If the form refuses it for a native mod, send `build/install/horizon_quests.hnmod` for that version (the mod works without its textures, plainer) and ask the maintainers.
- **Re-pin first**: native `.hmod` support is newer than our pin `fdc87ae`, and the registry compiles native mods on the newest Nightly. Update the launcher to the newest Nightly, check the glue against it, update the pinned revisions and `hooks`, test in game.
- **Run the registry's check**: `cargo install --git https://github.com/horizon-rust/horizon cargo-horizon-mod`, then `cargo horizon-mod check build/horizon_quests.hmod`.
- **Review**: every line is read. The code is clean (no dead-code allowances, no warnings from the game build), but a final read of the glue files (`hq_server_glue.rs`, `hq_client_glue.rs`) is worth it.
