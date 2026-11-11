# Horizon Quests

A quest book for [Horizon](https://playhorizon.dev), the Rust rewrite of Minecraft: Java Edition 26.3, in the spirit of FTB Quests: chapters of quests linked by dependencies, tasks to do, rewards to claim, and parties that share their progress. It is a native Horizon mod written from scratch in Rust: it contains no FTB Quests code, text or assets.

## What it does

- **A graphical quest book**, opened with a key (K by default) or by right-clicking the **Quest Book** every player gets on their first join. Chapters in a sidebar, quests as framed nodes on a canvas you can drag and zoom, dependency lines of small chevrons and beads pointing to what each quest opens (green once done). The pointer on a quest lights up what it needs (cyan) and what it opens (yellow).
- **A quest window** for each quest: its tasks and rewards as item icons (a check when done, a "!" when a reward waits), tooltips like Minecraft's, and its description. Click an icon to tick a checkmark, hand items in, claim a reward or pick one of several, or see the recipes that make an item.
- **9 task types**: hold items, hand items in, craft or smelt, kill mobs, tick a checkmark, reach an experience level, enter a dimension, reach a place, earn an advancement.
- **7 reward types**: items, experience points, levels, a console command, a toast, a random entry of a reward table, or one entry of a table chosen by the player. Rewards are claimed by each player, or once for the whole party.
- **Quests that repeat** after a cooldown, **optional** quests, tasks to do **in order**, four **dependency modes** and **visibility** rules (hidden until its dependencies show or are done, secret).
- **Parties**: `/quests team create`, `invite`, `join`, `leave`, `kick`. A party shares one progress; leaving gives you back your own, with what you claimed kept.
- **A tracked quest** pinned in the top-left corner of the screen while you play.
- **Packs** in the world's folder or in **data packs**, checked on load with a report of every problem, and reloaded with `/quests reload` or `/reload`.
- **Settings** for the server (`config/horizon_quests-server.toml`) and for each player (`config/horizon_quests-client.toml`).
- Solo and LAN. Dedicated servers are planned.

## Making quests

See **[docs/making-quests.md](docs/making-quests.md)**: where a pack goes, every field of the files, examples, and how to check a pack. The built-in example pack is in [`mod/mixins/horizon_quests/data/quests/`](mod/mixins/horizon_quests/data/quests/), and an example data pack in [`examples/quest-datapack/`](examples/quest-datapack/).

## Commands

| Command | Who | What it does |
|---|---|---|
| `/quests` | everyone | the help |
| `/quests progress` | everyone | your open quests and how far they are |
| `/quests claim` | everyone | claim every reward waiting for you |
| `/quests book` | everyone | get another Quest Book |
| `/quests check [task]`, `/quests submit [task]` | everyone | tick a checkmark, hand items in (alone: lists them) |
| `/quests team` ... `create <name>`, `invite <player>`, `join <name>`, `leave`, `kick <player>` | everyone | parties |
| `/quests reload`, `/quests validate` | op 2 | read the quest files again, list their problems |
| `/quests complete <player> <quest>`, `/quests reset <player> <quest>` | op 2 | complete or reset a quest for a player's team |
| `/quests reset_all <player>` | op 3 | reset all of a player's team's progress |

## Building from source

The mod lives in [`mod/`](mod/). Its pure logic is tested with `cargo test` (Rust stable, `RUSTFLAGS=-D warnings`), and `tools/check-glue.ps1` type-checks the parts that touch the game against the pinned Horizon. `tools/build-mod.ps1` (Windows) or `tools/build-mod.sh` (Linux) packs `build/horizon_quests.hmod` and has the Horizon launcher build it into your instance; that last step passes `--trust-native-code`, which only you can decide to run. Details: [`mod/README.md`](mod/README.md).

## Project notes

The design, decisions and journal are in [`vault/`](vault/00%20-%20Home.md), an Obsidian vault.

## License

[MIT](mod/LICENSE).
