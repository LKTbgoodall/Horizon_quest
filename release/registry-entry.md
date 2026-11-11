# The registry entry for playhorizon.dev (draft for 1.0.0)

What to paste into the form at https://playhorizon.dev/mods/submit when 1.0.0 is ready. Bastien sends it with his GitHub account; the form opens a pull request on `horizon-rust/horizon-mods` with `mods/horizon_quests.toml` and `icons/horizon_quests.png`, and a maintainer reviews every line of the `.hnmod`.

## Form fields

| Field | Value |
|---|---|
| File | `build/install/horizon_quests.hnmod` from `tools/build-mod.ps1` (the native half; see "Open points") |
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

Solo and LAN. Every player needs the mod. Dedicated servers are planned.
```

## Screenshots to take in game (for the description and the README)

1. The book on "Getting started", a quest under the pointer (cyan and yellow lines).
2. A quest window with tasks done (checks) and a reward waiting ("!").
3. The choice window.
4. The tracked quest in the top-left corner while playing.
5. A party: two players' books with "Party ..." in the header.

## Open points before sending

- **Which file**: the registry takes `.hnmod` files "until launchers and the game read native `.hmod`s", and a `.hnmod` holds code only. Our Quest Book texture and node frames are in the content half. Ask the Horizon maintainers (Discord) whether the registry can take our one `horizon_quests.hmod`, or a native `.hnmod` plus a content entry with the same id. Without the content, the book falls back to flat shapes and the Quest Book looks like a vanilla book: the mod works, but plainer.
- **Hooks number**: `mod/compatibility.json` says hooks 400 on Horizon `fdc87ae`. Check the Horizon version the registry builds against at the time.
- **Review**: every line is read. The code is clean (no dead-code allowances, no warnings from the game build), but a final read of the glue files (`hq_server_glue.rs`, `hq_client_glue.rs`) is worth it.
