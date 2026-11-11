# The registry entry for playhorizon.dev (draft for 1.0.0)

What to fill in the form at https://playhorizon.dev/mods/submit when 1.0.0 is ready. Bastien sends it with his GitHub account; the form opens a pull request on `horizon-rust/horizon-mods`, and a maintainer reviews every line of the code.

## The form, section by section (as seen 2026-11-11)

### The file
| Field | Value |
|---|---|
| Kind | **Native mod (.hnmod)** |
| File | **`build/horizon_quests.hmod`** from `tools/build-mod.ps1` (content and code in one file, [ADR-014](../vault/05%20-%20Decisions/ADR-014%20Publish%20one%20hmod%20with%20content%20and%20code.md)). The form's help lists `.hmod` among the files it takes; if it refuses it for a native mod, send `build/install/horizon_quests.hnmod` (code only: the book falls back to plain shapes) |
| Id | `horizon_quests` |
| Version | `1.0.0` |
| For Minecraft | `26.3` |
| What changed | `First public release.` |

### Its page
| Field | Value |
|---|---|
| Name | `Horizon Quests` |
| Licence | `MIT` |
| Summary | see "Summary" below |
| Description | see "Description" below (Markdown, 50 to 10,000 characters) |
| Categories (1 to 3) | **Adventure**, **Game mechanics** |
| Tags (up to 10, lowercase, digits, `-`) | `quests, quest-book, progression, ftb-quests, parties, rewards` |
| Authors | `LKTbgoodall` |
| Source code | `https://github.com/LKTbgoodall/Horizon_quest` (reviewers build the file from it: send a file built from the tagged commit) |
| Icon | `mod/icon.png` (256 x 256, 1.3 KB; the form takes a square PNG of 64 to 1024 pixels, at most 256 KiB) |
| Screenshots (up to 8, PNG or JPEG, at most 1 MiB each, 320x180 to 3840x2160) | the six JPEGs in `release/screenshots/`, in their order (`1-book-overview.jpg` to `6-tracked-quest.jpg`, 2560 x 1440, 0.3 to 0.75 MB each; made from `docs/images/`, whose PNGs are over 1 MiB) |

### Signature (optional, recommended)
With a minisign key on the project, every later version must be signed with it, so nobody else can publish an update under the mod's name. Bastien makes and keeps the key (Claude never holds it):

1. Install minisign: `winget install jedisct1.minisign`.
2. Once: `minisign -G -p minisign.pub -s %USERPROFILE%\.minisign\horizon_quests.key` (asks for a password). Keep the secret key and its password safe and out of the repository: losing it means a pull request by hand to change keys.
3. For each release: `minisign -S -s %USERPROFILE%\.minisign\horizon_quests.key -m build\horizon_quests.hmod` writes `build\horizon_quests.hmod.minisig`.
4. In the form: **Public key** = the `RW...` line of `minisign.pub`; **Signature** = the `.minisig` file.

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

## Screenshots (taken 2026-11-11)

PNGs in `docs/images/` (shown in the README), JPEG copies under 1 MiB in `release/screenshots/` for the form:

1. `book-overview`: the book on "Getting started".
2. `book-hover`: a quest under the pointer (cyan and yellow lines, its title).
3. `quest-window`: a quest window with a task done and a reward waiting ("!").
4. `choice-window`: the choice window (taken before 0.10.7 made it opaque).
5. `chapter-list`: the chapter list, names green / blue / grey, a "!".
6. `tracked-quest`: the tracked quest in the top-left corner while playing.

## Open points before sending

- **Which file: settled** ([ADR-014](../vault/05%20-%20Decisions/ADR-014%20Publish%20one%20hmod%20with%20content%20and%20code.md)). Horizon `main` now says native registry entries may name a `.hmod` "optionally with content": the launcher installs it, snapshots the content under the build's `native-content/<id>/` and the game loads it. Ours already follows the file's rules (top level: `horizon-mod.toml`, `assets/`, `horizon-native.json`, `mixins/`; one id). If the form refuses it for a native mod, send `build/install/horizon_quests.hnmod` for that version (the mod works without its textures, plainer) and ask the maintainers.
- **Re-pin first**: native `.hmod` support is newer than our pin `fdc87ae`, and the registry compiles native mods on the newest Nightly. Update the launcher to the newest Nightly, check the glue against it, update the pinned revisions and `hooks`, test in game.
- **Run the registry's check**: `cargo install --git https://github.com/horizon-rust/horizon cargo-horizon-mod`, then `cargo horizon-mod check build/horizon_quests.hmod`.
- **Review**: every line is read. The code is clean (no dead-code allowances, no warnings from the game build), but a final read of the glue files (`hq_server_glue.rs`, `hq_client_glue.rs`) is worth it.
