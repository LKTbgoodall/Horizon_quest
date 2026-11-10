---
tags: [conception, data-model]
statut: brouillon
date: 2026-11-10
---
# Modèle de données Horizon Quest

Inspiration : [[Modèle de données FTB]]. Format sur disque : [[Format des fichiers de quêtes]].

> [!note] Brouillon v0
> Ce sont des structures Rust indicatives. À affiner au moment de coder `hq_model`. Les champs marqués `// V1` ou `// V2` ne sont pas dans le MVP.

## Identifiants
- `QuestId`, `ChapterId`, `TaskId`, `RewardId`, `TableId` : des chaînes **lisibles** (`"getting_started/first_wood"`) quand on écrit à la main, ou un hexa aléatoire de 16 caractères quand l'éditeur génère.
- L'id est **stable** : la progression le référence, donc on ne le renomme jamais à la légère (prévoir une table d'alias plus tard).

## Définitions
```rust
struct QuestFile {
    version: u32,                       // migrations
    settings: FileSettings,             // défauts globaux
    groups: Vec<ChapterGroup>,
    chapters: Vec<Chapter>,             // chargés depuis chapters/*.json
    reward_tables: Vec<RewardTable>,    // V1
}

struct Chapter {
    id: ChapterId, title: Text, subtitle: Option<Text>,
    icon: ItemIcon, group: Option<String>, order: i32,
    progression: ProgressionMode,       // Linear (défaut) | Flexible
    default_shape: Shape, default_size: f32,
    hidden: bool,
    quests: Vec<Quest>,
    links: Vec<QuestLink>,              // V2
    images: Vec<ChapterImage>,          // V2
}

struct Quest {
    id: QuestId, title: Text, subtitle: Option<Text>, description: Vec<Text>,
    icon: Option<ItemIcon>,             // défaut : icône de la 1re tâche
    x: f32, y: f32, shape: Option<Shape>, size: Option<f32>,
    dependencies: Vec<QuestId>,
    dependency_mode: DependencyMode,    // AllCompleted | OneCompleted | AllStarted | OneStarted
    min_required: Option<u32>,          // prime sur dependency_mode
    visibility: Visibility,             // Always | UntilDepsVisible | UntilDepsComplete | Secret
    optional: bool,
    repeatable: Option<Repeat>,         // V1 : { cooldown_s }
    sequential_tasks: bool,             // V1
    tasks: Vec<Task>,
    rewards: Vec<Reward>,
}

enum Task {                              // + id, title optionnel
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
    Custom { key: String },                             // V2 (API autres mods)
}

enum ItemMatch { Id(String), Tag(String), Stack { snbt: String } /* V2 : composants */ }

enum Reward {                            // + id, team: Option<bool>, auto_claim: AutoClaim
    Item { stack: ItemSpec },
    Xp { points: u32 },
    XpLevels { levels: u32 },
    Command { command: String, as_player: bool },       // V1 ; {player} remplacé
    Choice { table: TableId },                          // V1
    Random { table: TableId },                          // V1
    Loot { table: TableId },                            // V2
    AllTable { table: TableId },                        // V2
    Toast { title: Text, description: Text },           // V1
    Currency { amount: i64 },                           // V2 (pont économie ?)
}

enum AutoClaim { Default, Disabled, Enabled, NoToast, Invisible }
```

## Progression (par équipe)
```rust
struct TeamProgress {
    team: TeamId,                       // = UUID du joueur pour une équipe perso
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
Les règles de fusion et de copie de progression sont reprises de [[Équipes et progression FTB]].

## États dérivés (calculés, jamais stockés)
`Locked` → `Visible` → `Startable` → `InProgress` → `Completed` → `RewardsClaimed`
Calculés par `hq_logic` à partir de la définition et de `TeamProgress`.

## Questions
- Les textes en `Text` (composant JSON) ou en clés de traduction ? → [[Questions ouvertes]]
