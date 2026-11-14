# Showcase post for the Horizon Discord (#showcase)

The #showcase channel is a forum. Its pinned rules (read 2026-11-14): one post per creation (later updates go in the same post), at most one new post an hour, a screenshot, a video or a link is required, pick the tags that fit, and fill in the template below. A bot (Kira) checks each new post; if something is missing, edit the first message within an hour or the post is closed.

Bastien posts it himself (Claude does not post on Discord).

## Post title

```
Horizon Quests: an FTB Quests-style quest book, as a native Rust mod
```

## Tags

Whatever fits among the channel's tags: a "mod" or "native mod" tag if there is one, plus "WIP" / "in progress" if offered (1.0.0 is not on the registry yet).

## First message (paste as is, under 2000 characters)

```
What it is: **Horizon Quests**, a quest book for Horizon in the spirit of FTB Quests, written from scratch as a native Rust mod (MIT, no FTB code or assets).

- A graphical book (K, or the Quest Book every player gets): chapters, quests as nodes on a canvas you drag and zoom, animated dependency lines (cyan = what a quest needs, yellow = what it opens)
- A quest window with tasks and rewards as item icons, Minecraft-style tooltips, recipes of the items asked for
- 9 task types (items, hand-ins, crafts, kills, checkmarks, XP levels, dimensions, places, advancements) and 7 reward types (items, XP, levels, commands, toasts, random and choice reward tables)
- Repeatable, optional and secret quests, tasks in order
- Parties that share their progress (`/quests team`), rewards per player or per party
- **An in-game editor** for operators: drag quests, link them by right-dragging, edit tasks and rewards in place with an item / mob picker, reorder chapters and groups with the mouse
- Packs in the world, the modpack config or data packs, checked on load
- Ships with a 9-chapter vanilla progression pack (81 quests, to the end of the game and beyond)

Solo and LAN for now, dedicated servers planned. 1.0.0 goes to the registry as soon as the launcher installs a combined `.hmod`'s content (horizon#15933).

Horizon version: nightly-2026-11-13-9f42064 (launcher 0.2.65)
Source and guide to write your own quests: https://github.com/LKTbgoodall/Horizon_quest
```

## Files to attach

The six JPEGs in `release/screenshots/` (each under 1 MiB), in this order:

1. `1-book-overview.jpg`
2. `2-book-hover.jpg`
3. `3-quest-window.jpg`
4. `5-chapter-list.jpg`
5. `6-tracked-quest.jpg`
6. `4-choice-window.jpg` (taken before 0.10.7 made the window opaque; skip it if a newer one exists)

None shows the editor or the right-hand rail (v0.19.0): a screenshot or a short clip of edit mode (dragging a quest, linking two quests, dragging a chapter) would be the strongest addition. A clip also counts as the required media.
