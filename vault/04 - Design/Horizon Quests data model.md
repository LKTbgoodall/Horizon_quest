---
tags: [design, data-model]
status: draft
date: 2026-11-10
---
# Horizon Quests data model

Inspiration: [[FTB data model]]. On-disk format: [[Quest file format]].

> [!important] Implemented (format version 1, 2026-11-10)
> The MVP part is coded in `mod/mixins/horizon_quests/hq_model.rs`, **which is the reference**. The structures below are the original draft: fields marked `// V1` or `// V2` are **not in the code yet**, and the MVP fields differ in small ways listed here.
>
> What the code has, where it differs from the draft:
> - **Ids**: short ids in the files (`[a-z0-9_.-]+`). Full ids are built: quest `chapter/quest`, task or reward `chapter/quest/child`. A dependency is `quest` (same chapter) or `chapter/quest`. Task ids and reward ids are unique within their quest.
> - **Text** is plain `String` (see [[Open questions]]).
> - `Settings` (`quests.json`): `version`, `progression`, `default_shape`, `default_size`, `auto_claim_rewards`, `detection_delay`, `groups`.
> - `Chapter`: `progression`, `default_shape`, `default_size` are optional and fall back to the settings. No `links` or `images`.
> - `Quest`: `sequential_tasks` (bool), `repeatable` (bool) and `repeat_cooldown` (seconds) since v0.0.6, instead of the draft's `Option<Repeat>`. A quest **without tasks** completes as soon as its dependencies are met.
> - `Visibility`: `always`, `until_deps_visible` (every dependency shows), `until_deps_complete` (dependencies met), `secret` (until completed). A completed quest always shows; a `hidden` chapter's quests never do.
> - `TaskKind`: `item` (`item`, `count`, `consume`), `checkmark`, `kill` (`entity`, `count`); since v0.0.6 `craft` (`item`, `count`), `xp_levels` (`levels`, never taken), `dimension` (`dimension`), `location` (`dimension?`, `from`, `to`: a box of blocks, corners included), `advancement` (`advancement`, no criterion). `ItemMatch` is `Id` or `Tag` (`#ns:tag`).
> - `RewardKind`: `item`, `xp`, `xp_levels`; since v0.0.6 `command` (`command`, always as the console), `toast` (`text`), `random` (`table`), `choice` (`table`). A `Reward` has an optional `title`. `auto_claim`: `default | disabled | enabled`. `team` (v0.5.0): claimed once for the whole team (`Progress::claimed_for`: anyone of the team claimed it).
> - `RewardTable { id, entries }`, `TableEntry { weight (1), title, kind }` in `QuestFile.reward_tables` (v0.0.6). An entry's kind is never another table.
> - `Progress` (the draft's `TeamProgress`): `version`, `started`, `completed`, `tasks`, `claimed` (reward → set of player UUID strings); since v0.0.6 `repeats` (quest → times it started over, instead of the draft's `repeat_ready_at`) and `advancements` (those seen done that a task names). `ever_completed` = completed now or started over at least once. Unknown ids are kept.
> - Since v0.2.0, `hq_teams::Teams { version, parties }` in `teams.json`; `Party { name, owner, members (player id → name), invites (player id → name), created }`. A player's team is their party's id, else their own player id; `Progress` is unchanged and kept per team ([[ADR-012 Parties share one progress]]).
> - Derived states (`hq_logic::QuestState`): `Locked`, `Available`, `InProgress`, `RewardsWaiting`, `Done`, plus `visible`.

## Identifiers
- `QuestId`, `ChapterId`, `TaskId`, `RewardId`, `TableId`: **readable** strings (`"getting_started/first_wood"`) when written by hand, or a random 16-character hex when the editor generates them.
- The id is **stable**: progress references it, so never rename it lightly (plan an alias table later).

## Definitions
```rust
struct QuestFile {
    version: u32,                       // migrations
    settings: FileSettings,             // global defaults
    groups: Vec<ChapterGroup>,
    chapters: Vec<Chapter>,             // loaded from chapters/*.json
    reward_tables: Vec<RewardTable>,    // V1
}

struct Chapter {
    id: ChapterId, title: Text, subtitle: Option<Text>,
    icon: ItemIcon, group: Option<String>, order: i32,
    progression: ProgressionMode,       // Linear (default) | Flexible
    default_shape: Shape, default_size: f32,
    hidden: bool,
    quests: Vec<Quest>,
    links: Vec<QuestLink>,              // V2
    images: Vec<ChapterImage>,          // V2
}

struct Quest {
    id: QuestId, title: Text, subtitle: Option<Text>, description: Vec<Text>,
    icon: Option<ItemIcon>,             // default: icon of the 1st task
    x: f32, y: f32, shape: Option<Shape>, size: Option<f32>,
    dependencies: Vec<QuestId>,
    dependency_mode: DependencyMode,    // AllCompleted | OneCompleted | AllStarted | OneStarted
    min_required: Option<u32>,          // overrides dependency_mode
    visibility: Visibility,             // Always | UntilDepsVisible | UntilDepsComplete | Secret
    optional: bool,
    repeatable: Option<Repeat>,         // V1 : { cooldown_s }
    sequential_tasks: bool,             // V1
    tasks: Vec<Task>,
    rewards: Vec<Reward>,
}

enum Task {                              // + id, optional title
    Item { item: ItemMatch, count: u32, consume: bool },
    Checkmark,
    Kill { entity: String, count: u32 },
    XpLevels { levels: u32, consume: bool },           // V1
    Dimension { dimension: String },                    // V1
    Location { dimension: String, min: [i32;3], max: [i32;3] }, // V1
    Advancement { advancement: String, criterion: Option<String> }, // V1
    Craft { item: ItemMatch, count: u32 },              // V1 (via item_crafted)
    Stat { stat_type: String, stat: String, value: i64 }, // V2
    Biome { biome: String },                            // V2 (⏳ hook)
    Custom { key: String },                             // V2 (API for other mods)
}

enum ItemMatch { Id(String), Tag(String), Stack { snbt: String } /* V2 : components */ }

enum Reward {                            // + id, team: Option<bool>, auto_claim: AutoClaim
    Item { stack: ItemSpec },
    Xp { points: u32 },
    XpLevels { levels: u32 },
    Command { command: String, as_player: bool },       // V1 ; {player} replaced
    Choice { table: TableId },                          // V1
    Random { table: TableId },                          // V1
    Loot { table: TableId },                            // V2
    AllTable { table: TableId },                        // V2
    Toast { title: Text, description: Text },           // V1
    Currency { amount: i64 },                           // V2 (economy bridge?)
}

enum AutoClaim { Default, Disabled, Enabled, NoToast, Invisible }
```

## Progress (per team)
```rust
struct TeamProgress {
    team: TeamId,                       // = the player's UUID for a personal team
    started: BTreeMap<QuestId, Timestamp>,
    completed: BTreeMap<QuestId, Timestamp>,
    task_progress: BTreeMap<TaskId, u64>,
    claimed: BTreeMap<RewardId, ClaimState>, // ClaimState::Team | ::Players(BTreeSet<Uuid>)
    repeat_ready_at: BTreeMap<QuestId, Timestamp>, // V1
}

struct Teams {                           // hq_teams
    player_team: BTreeMap<Uuid, TeamId>,
    teams: BTreeMap<TeamId, Team>,      // { name, owner, members, invites, kind: Personal|Party }
}
```
The rules for merging and copying progress are taken from [[FTB teams and progress]].

## Derived states (computed, never stored)
`Locked` → `Visible` → `Startable` → `InProgress` → `Completed` → `RewardsClaimed`
Computed by `hq_logic` from the definition and `TeamProgress`.

## Questions
- Texts as `Text` (JSON component) or as translation keys? → [[Open questions]]
