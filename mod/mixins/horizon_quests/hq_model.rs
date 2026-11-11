// The quest book's data: what a quest pack's files hold (settings, chapters,
// quests, tasks, rewards) and what a progress file holds. Pure: no game
// types. Loading and checking are in hq_validate, the rules in hq_logic and
// hq_progress.
//
// Ids: a chapter, quest, task or reward has a short id (`first_wood`). The
// full id of a quest is `chapter/quest`, of a task or reward
// `chapter/quest/task`. Progress refers to full ids, so ids must stay stable.
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// The version of the quest files and progress files this code reads and
/// writes.
pub const FORMAT_VERSION: u32 = 1;

fn one() -> u32 {
    1
}

/// Whether tasks count before a quest's dependencies are met.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Progression {
    /// Tasks only count once the dependencies are met.
    #[default]
    Linear,
    /// Tasks always count; the quest completes once its dependencies are met
    /// too.
    Flexible,
}

/// A quest node's outline in the book.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Shape {
    #[default]
    Square,
    Circle,
    Diamond,
    Hexagon,
    Octagon,
}

/// What a quest's dependencies must be for it to open.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DependencyMode {
    #[default]
    AllCompleted,
    OneCompleted,
    AllStarted,
    OneStarted,
}

/// When a quest shows in the book. A completed quest always shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Visibility {
    #[default]
    Always,
    /// Hidden until every dependency shows.
    UntilDepsVisible,
    /// Hidden until the dependencies are met.
    UntilDepsComplete,
    /// Hidden until completed.
    Secret,
}

/// Whether a reward is given as soon as its quest completes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AutoClaim {
    /// As `auto_claim_rewards` in quests.json says.
    #[default]
    Default,
    Disabled,
    Enabled,
}

/// `quests.json`: the pack's version and defaults.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub version: u32,
    pub progression: Progression,
    pub default_shape: Shape,
    /// A quest node's size, 1 being the standard node.
    pub default_size: f32,
    pub auto_claim_rewards: bool,
    /// Ticks between two looks at the players' inventories for item tasks.
    pub detection_delay: u32,
    pub groups: Vec<Group>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            version: FORMAT_VERSION,
            progression: Progression::default(),
            default_shape: Shape::default(),
            default_size: 1.0,
            auto_claim_rewards: false,
            detection_delay: 20,
            groups: Vec::new(),
        }
    }
}

/// A heading chapters gather under in the book's chapter list.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Group {
    pub id: String,
    pub title: String,
}

/// One file of `chapters/`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Chapter {
    pub id: String,
    pub title: String,
    pub subtitle: Option<String>,
    /// An item id.
    pub icon: Option<String>,
    pub group: Option<String>,
    /// Chapters list by `order`, then by file name.
    pub order: i32,
    pub progression: Option<Progression>,
    pub default_shape: Option<Shape>,
    pub default_size: Option<f32>,
    /// Never listed, and its quests never show.
    pub hidden: bool,
    pub quests: Vec<Quest>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Quest {
    pub id: String,
    pub title: String,
    pub subtitle: Option<String>,
    /// One paragraph per entry.
    pub description: Vec<String>,
    /// An item id. The book falls back to the first task's item.
    pub icon: Option<String>,
    /// The node's grid cell on the chapter's canvas.
    pub x: f32,
    pub y: f32,
    pub shape: Option<Shape>,
    pub size: Option<f32>,
    /// `quest` for one of the same chapter, `chapter/quest` for another's.
    pub dependencies: Vec<String>,
    pub dependency_mode: DependencyMode,
    /// "At least this many dependencies completed", instead of
    /// `dependency_mode`.
    pub min_required: Option<u32>,
    pub visibility: Visibility,
    /// Not counted in the chapter's completion.
    pub optional: bool,
    /// Tasks are done in order: one only counts once those before it are
    /// done.
    pub sequential_tasks: bool,
    /// Can be done again: once completed, its rewards claimed and its
    /// cooldown over, it starts over.
    pub repeatable: bool,
    /// Seconds a repeatable quest stays completed before it starts over.
    pub repeat_cooldown: u32,
    /// All must be done. A quest without tasks completes as soon as its
    /// dependencies are met.
    pub tasks: Vec<Task>,
    pub rewards: Vec<Reward>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Task {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(flatten)]
    pub kind: TaskKind,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TaskKind {
    /// Have `count` of the item. With `consume`, hand them in instead.
    Item {
        item: ItemMatch,
        #[serde(default = "one")]
        count: u32,
        #[serde(default)]
        consume: bool,
    },
    /// Tick a box in the book.
    Checkmark,
    /// Kill `count` entities of this type (an entity id, written in full
    /// once loaded: `minecraft:zombie`).
    Kill {
        entity: String,
        #[serde(default = "one")]
        count: u32,
    },
    /// Craft `count` of the item: results taken from a crafting grid or a
    /// furnace, counted while the quest is open.
    Craft {
        item: ItemMatch,
        #[serde(default = "one")]
        count: u32,
    },
    /// Reach experience level `levels`.
    XpLevels { levels: u32 },
    /// Be in this dimension (`minecraft:the_nether`).
    Dimension { dimension: String },
    /// Stand in the box between the blocks `from` and `to` (both included),
    /// in `dimension` when it is given.
    Location {
        #[serde(default)]
        dimension: Option<String>,
        from: [i32; 3],
        to: [i32; 3],
    },
    /// Have this advancement done (`minecraft:story/mine_stone`).
    Advancement { advancement: String },
}

impl TaskKind {
    /// The progress at which the task is done.
    pub fn goal(&self) -> u64 {
        match self {
            TaskKind::Item { count, .. } | TaskKind::Kill { count, .. } | TaskKind::Craft { count, .. } => {
                u64::from(*count)
            }
            TaskKind::XpLevels { levels } => u64::from(*levels),
            TaskKind::Checkmark
            | TaskKind::Dimension { .. }
            | TaskKind::Location { .. }
            | TaskKind::Advancement { .. } => 1,
        }
    }
}

/// What an item task accepts: `minecraft:oak_log` for one item,
/// `#minecraft:logs` for any item of a tag.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub enum ItemMatch {
    Id(String),
    Tag(String),
}

impl TryFrom<String> for ItemMatch {
    type Error = String;

    fn try_from(text: String) -> Result<Self, String> {
        let (tag, name) = match text.strip_prefix('#') {
            Some(name) => (true, name),
            None => (false, text.as_str()),
        };
        let id = resource_id(name).ok_or_else(|| format!("`{text}` is not an item id or a #tag"))?;
        Ok(if tag { ItemMatch::Tag(id) } else { ItemMatch::Id(id) })
    }
}

impl From<ItemMatch> for String {
    fn from(item: ItemMatch) -> String {
        match item {
            ItemMatch::Id(id) => id,
            ItemMatch::Tag(tag) => format!("#{tag}"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Reward {
    #[serde(default)]
    pub id: String,
    /// What the book calls it, instead of a name made from its type.
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub auto_claim: AutoClaim,
    /// A team reward: one member claims it for the whole party (a lone
    /// player is a party of one). False: each player claims their own.
    #[serde(default)]
    pub team: bool,
    #[serde(flatten)]
    pub kind: RewardKind,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RewardKind {
    /// `count` of an item (its id, written in full once loaded).
    Item {
        item: String,
        #[serde(default = "one")]
        count: u32,
    },
    /// Experience points.
    Xp { points: u32 },
    XpLevels { levels: u32 },
    /// A console command, run with op level 4. `{player}` is the player's
    /// name and `{uuid}` their UUID.
    Command { command: String },
    /// A toast on the player's screen (a chat line for a client without the
    /// mod): the reward's `title`, with `text` under it.
    Toast {
        #[serde(default)]
        text: String,
    },
    /// One entry of a reward table, drawn by weight.
    Random { table: String },
    /// One entry of a reward table, picked by the player in the book.
    Choice { table: String },
}

impl RewardKind {
    /// The table a random or choice reward takes its entry from.
    pub fn table(&self) -> Option<&str> {
        match self {
            RewardKind::Random { table } | RewardKind::Choice { table } => Some(table),
            _ => None,
        }
    }
}

/// One file of `reward_tables/`: what random and choice rewards give.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RewardTable {
    pub id: String,
    pub entries: Vec<TableEntry>,
}

/// One thing a reward table can give. Its kind is never another table.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TableEntry {
    /// How often a random reward draws it, against the other entries.
    #[serde(default = "one")]
    pub weight: u32,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(flatten)]
    pub kind: RewardKind,
}

/// A whole quest pack: quests.json, its chapters in book order, and its
/// reward tables.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct QuestFile {
    pub settings: Settings,
    pub chapters: Vec<Chapter>,
    pub reward_tables: Vec<RewardTable>,
}

/// What one team has done (a lone player is their own team). Times are Unix
/// seconds. Ids no pack defines any more are kept: the pack may bring them
/// back.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Progress {
    pub version: u32,
    /// Quest → when its first task progress came.
    pub started: BTreeMap<String, u64>,
    /// Quest → when it completed.
    pub completed: BTreeMap<String, u64>,
    /// Task → how far it is (its goal when done). Absent means 0.
    pub tasks: BTreeMap<String, u64>,
    /// Reward → the players who claimed it.
    pub claimed: BTreeMap<String, BTreeSet<String>>,
    /// Repeatable quest → how many times it was completed and started over.
    pub repeats: BTreeMap<String, u32>,
    /// The advancements the pack's tasks name that a member was seen to
    /// complete: a quest opening later still finds them.
    pub advancements: BTreeSet<String>,
    /// The quest book item was given to the player on their first join.
    pub book_given: bool,
}

impl Default for Progress {
    fn default() -> Self {
        Self {
            version: FORMAT_VERSION,
            started: BTreeMap::new(),
            completed: BTreeMap::new(),
            tasks: BTreeMap::new(),
            claimed: BTreeMap::new(),
            repeats: BTreeMap::new(),
            advancements: BTreeSet::new(),
            book_given: false,
        }
    }
}

impl Progress {
    pub fn is_completed(&self, quest: &str) -> bool {
        self.completed.contains_key(quest)
    }

    /// Completed now, or before it started over (a repeatable quest): what
    /// dependencies and chapter tallies count.
    pub fn ever_completed(&self, quest: &str) -> bool {
        self.is_completed(quest) || self.repeats.get(quest).is_some_and(|times| *times > 0)
    }

    /// Started, which a quest completed at least once is too.
    pub fn is_started(&self, quest: &str) -> bool {
        self.started.contains_key(quest) || self.ever_completed(quest)
    }

    pub fn task(&self, task: &str) -> u64 {
        self.tasks.get(task).copied().unwrap_or(0)
    }

    pub fn has_claimed(&self, reward: &str, player: &str) -> bool {
        self.claimed.get(reward).is_some_and(|players| players.contains(player))
    }

    /// Whether reward `reward` is claimed as far as `player` goes: by them,
    /// or by anyone of the team for a team reward.
    pub fn claimed_for(&self, reward: &str, player: &str, team: bool) -> bool {
        if team { self.claimed.get(reward).is_some_and(|players| !players.is_empty()) } else { self.has_claimed(reward, player) }
    }
}

/// Whether `id` can name a chapter, quest, task or reward: lowercase
/// letters, digits, `_`, `-` and `.`.
pub fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || "_-.".contains(c))
}

/// `apple` or `minecraft:apple` → `minecraft:apple`. None when `text` is not
/// a registry id.
pub fn resource_id(text: &str) -> Option<String> {
    let (namespace, path) = text.split_once(':').unwrap_or(("minecraft", text));
    let ok = |part: &str, slash: bool| {
        !part.is_empty()
            && part
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || "_-.".contains(c) || (slash && c == '/'))
    };
    (ok(namespace, false) && ok(path, true)).then(|| format!("{namespace}:{path}"))
}

/// A quest's full id.
pub fn quest_id(chapter: &str, quest: &str) -> String {
    format!("{chapter}/{quest}")
}

/// A task's or reward's full id, from its quest's full id.
pub fn child_id(quest: &str, child: &str) -> String {
    format!("{quest}/{child}")
}

/// The full id of the quest a dependency names, written in `chapter`.
pub fn dependency_id(chapter: &str, dependency: &str) -> String {
    if dependency.contains('/') {
        dependency.to_owned()
    } else {
        quest_id(chapter, dependency)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_chapter_reads_with_defaults() {
        let chapter: Chapter = serde_json::from_str(
            r##"{
              "id": "getting_started",
              "title": "Getting started",
              "icon": "minecraft:oak_sapling",
              "quests": [
                { "id": "first_wood", "title": "Wood!", "x": 0, "y": 0,
                  "description": ["Punch a tree."],
                  "tasks": [ { "id": "t1", "type": "item", "item": "#minecraft:logs", "count": 8 } ],
                  "rewards": [ { "id": "r1", "type": "item", "item": "minecraft:apple", "count": 2 },
                               { "id": "r2", "type": "xp", "points": 20 } ] },
                { "id": "table", "title": "Crafting table", "x": 2, "y": -1.5,
                  "dependencies": ["first_wood"], "visibility": "until_deps_complete",
                  "tasks": [ { "id": "t1", "type": "item", "item": "crafting_table", "consume": true },
                             { "id": "t2", "type": "checkmark", "title": "Ready?" },
                             { "id": "t3", "type": "kill", "entity": "minecraft:zombie", "count": 3 } ],
                  "rewards": [ { "id": "r1", "type": "xp_levels", "levels": 1, "auto_claim": "enabled" } ] }
              ]
            }"##,
        )
        .unwrap();
        assert_eq!(chapter.progression, None);
        assert!(!chapter.hidden);
        let wood = &chapter.quests[0];
        assert_eq!(wood.dependency_mode, DependencyMode::AllCompleted);
        assert_eq!(wood.visibility, Visibility::Always);
        assert_eq!(
            wood.tasks[0].kind,
            TaskKind::Item { item: ItemMatch::Tag("minecraft:logs".into()), count: 8, consume: false }
        );
        assert_eq!(wood.rewards[0].kind, RewardKind::Item { item: "minecraft:apple".into(), count: 2 });
        assert_eq!(wood.rewards[0].auto_claim, AutoClaim::Default);
        let table = &chapter.quests[1];
        assert_eq!((table.x, table.y), (2.0, -1.5));
        assert_eq!(table.visibility, Visibility::UntilDepsComplete);
        assert_eq!(
            table.tasks[0].kind,
            TaskKind::Item { item: ItemMatch::Id("minecraft:crafting_table".into()), count: 1, consume: true }
        );
        assert_eq!(table.tasks[1].kind, TaskKind::Checkmark);
        assert_eq!(table.tasks[1].title.as_deref(), Some("Ready?"));
        assert_eq!(table.tasks[2].kind.goal(), 3);
        assert_eq!(table.rewards[0].auto_claim, AutoClaim::Enabled);
    }

    #[test]
    fn a_chapter_round_trips() {
        let chapter = Chapter {
            id: "c".into(),
            quests: vec![Quest {
                id: "q".into(),
                tasks: vec![Task {
                    id: "t".into(),
                    title: None,
                    kind: TaskKind::Item { item: ItemMatch::Tag("minecraft:logs".into()), count: 4, consume: true },
                }],
                rewards: vec![Reward {
                    id: "r".into(),
                    title: None,
                    auto_claim: AutoClaim::Disabled,
                    team: true,
                    kind: RewardKind::Xp { points: 5 },
                }],
                ..Quest::default()
            }],
            ..Chapter::default()
        };
        let text = serde_json::to_string(&chapter).unwrap();
        assert!(text.contains(r##""item":"#minecraft:logs""##), "{text}");
        assert_eq!(serde_json::from_str::<Chapter>(&text).unwrap(), chapter);
    }

    #[test]
    fn bad_tasks_and_items_are_refused() {
        for bad in [
            r#"{ "id": "t", "type": "dance" }"#,
            r#"{ "id": "t" }"#,
            r#"{ "id": "t", "type": "item" }"#,
            r#"{ "id": "t", "type": "item", "item": "Oak Log" }"#,
            r##"{ "id": "t", "type": "item", "item": "#" }"##,
            r#"{ "id": "t", "type": "item", "item": "minecraft:apple", "count": -1 }"#,
        ] {
            assert!(serde_json::from_str::<Task>(bad).is_err(), "{bad}");
        }
        let err = serde_json::from_str::<Task>(r#"{ "type": "item", "item": "Oak Log" }"#).unwrap_err();
        assert!(err.to_string().contains("`Oak Log` is not an item id"), "{err}");
    }

    #[test]
    fn settings_default_when_fields_are_missing() {
        assert_eq!(serde_json::from_str::<Settings>("{}").unwrap(), Settings::default());
        let settings: Settings = serde_json::from_str(
            r#"{ "progression": "flexible", "auto_claim_rewards": true,
                 "groups": [ { "id": "main", "title": "Main story" } ] }"#,
        )
        .unwrap();
        assert_eq!(settings.version, FORMAT_VERSION);
        assert_eq!(settings.progression, Progression::Flexible);
        assert_eq!(settings.detection_delay, 20);
        assert_eq!(settings.groups[0].title, "Main story");
    }

    #[test]
    fn progress_round_trips_and_answers() {
        let mut progress = Progress::default();
        progress.started.insert("c/q".into(), 10);
        progress.tasks.insert("c/q/t".into(), 3);
        progress.completed.insert("c/done".into(), 20);
        progress.claimed.entry("c/done/r".into()).or_default().insert("steve".into());
        let text = serde_json::to_string(&progress).unwrap();
        assert_eq!(serde_json::from_str::<Progress>(&text).unwrap(), progress);
        assert_eq!(serde_json::from_str::<Progress>("{}").unwrap(), Progress::default());

        assert!(progress.is_started("c/q") && !progress.is_completed("c/q"));
        assert!(progress.is_started("c/done") && progress.is_completed("c/done"));
        assert_eq!((progress.task("c/q/t"), progress.task("c/q/other")), (3, 0));
        assert!(progress.has_claimed("c/done/r", "steve"));
        assert!(!progress.has_claimed("c/done/r", "alex"));
    }

    #[test]
    fn the_later_task_and_reward_kinds_read() {
        let quest: Quest = serde_json::from_str(
            r##"{ "id": "q", "sequential_tasks": true, "repeatable": true, "repeat_cooldown": 300,
                "tasks": [ { "id": "a", "type": "craft", "item": "#minecraft:planks", "count": 8 },
                           { "id": "b", "type": "xp_levels", "levels": 5 },
                           { "id": "c", "type": "dimension", "dimension": "minecraft:the_nether" },
                           { "id": "d", "type": "location", "from": [0, 60, 0], "to": [16, 80, 16] },
                           { "id": "e", "type": "advancement", "advancement": "minecraft:story/mine_stone" } ],
                "rewards": [ { "id": "r", "type": "command", "command": "say hi {player}" },
                             { "id": "s", "type": "toast", "title": "Well done" },
                             { "id": "t", "type": "random", "table": "loot" },
                             { "id": "u", "type": "choice", "table": "picks" } ] }"##,
        )
        .unwrap();
        assert!(quest.sequential_tasks && quest.repeatable);
        assert_eq!(quest.repeat_cooldown, 300);
        let goals: Vec<u64> = quest.tasks.iter().map(|task| task.kind.goal()).collect();
        assert_eq!(goals, [8, 5, 1, 1, 1]);
        assert_eq!(quest.tasks[0].kind, TaskKind::Craft { item: ItemMatch::Tag("minecraft:planks".into()), count: 8 });
        assert_eq!(quest.tasks[3].kind, TaskKind::Location { dimension: None, from: [0, 60, 0], to: [16, 80, 16] });
        assert_eq!(quest.rewards[1].kind, RewardKind::Toast { text: String::new() });
        assert_eq!(quest.rewards[1].title.as_deref(), Some("Well done"));
        let tables: Vec<Option<&str>> = quest.rewards.iter().map(|reward| reward.kind.table()).collect();
        assert_eq!(tables, [None, None, Some("loot"), Some("picks")]);
        // A quest that says nothing of it is done once, in any order.
        let plain = Quest::default();
        assert!(!plain.sequential_tasks && !plain.repeatable && plain.repeat_cooldown == 0);
        // A task or a reward missing what its type needs is refused.
        for bad in [r#"{ "type": "xp_levels" }"#, r#"{ "type": "location", "from": [0, 0, 0] }"#, r#"{ "type": "dimension" }"#] {
            assert!(serde_json::from_str::<Task>(bad).is_err(), "{bad}");
        }
        for bad in [r#"{ "type": "command" }"#, r#"{ "type": "random" }"#, r#"{ "type": "choice", "table": 3 }"#] {
            assert!(serde_json::from_str::<Reward>(bad).is_err(), "{bad}");
        }

        let table: RewardTable = serde_json::from_str(
            r#"{ "id": "loot", "entries": [ { "type": "xp", "points": 5 },
                 { "weight": 3, "title": "Shiny", "type": "item", "item": "minecraft:diamond" } ] }"#,
        )
        .unwrap();
        assert_eq!((table.entries[0].weight, table.entries[1].weight), (1, 3));
        assert_eq!(table.entries[1].title.as_deref(), Some("Shiny"));
        assert_eq!(table.entries[1].kind, RewardKind::Item { item: "minecraft:diamond".into(), count: 1 });
    }

    #[test]
    fn a_quest_that_started_over_still_counts_as_completed_once() {
        // A progress file written before repeats and advancements existed.
        let mut progress: Progress = serde_json::from_str(r#"{ "completed": { "c/q": 5 } }"#).unwrap();
        assert!(progress.repeats.is_empty() && progress.advancements.is_empty());
        assert!(progress.ever_completed("c/q") && !progress.ever_completed("c/other"));
        progress.completed.clear();
        assert!(!progress.ever_completed("c/q") && !progress.is_started("c/q"));
        progress.repeats.insert("c/q".into(), 2);
        progress.advancements.insert("minecraft:story/mine_stone".into());
        assert!(progress.ever_completed("c/q") && progress.is_started("c/q") && !progress.is_completed("c/q"));
        let text = serde_json::to_string(&progress).unwrap();
        assert_eq!(serde_json::from_str::<Progress>(&text).unwrap(), progress);
    }

    #[test]
    fn ids() {
        for good in ["a", "first_wood", "01-intro", "v1.2", "0123456789abcdef"] {
            assert!(valid_id(good), "{good}");
        }
        for bad in ["", "First", "a b", "a/b", "é", "a:b"] {
            assert!(!valid_id(bad), "{bad}");
        }
        assert_eq!(resource_id("apple").as_deref(), Some("minecraft:apple"));
        assert_eq!(resource_id("create:gears/big").as_deref(), Some("create:gears/big"));
        for bad in ["", ":", "a:", ":b", "a:b:c", "Apple", "a b", "a/b:c"] {
            assert_eq!(resource_id(bad), None, "{bad}");
        }
        assert_eq!(quest_id("c", "q"), "c/q");
        assert_eq!(child_id("c/q", "t1"), "c/q/t1");
        assert_eq!(dependency_id("c", "q"), "c/q");
        assert_eq!(dependency_id("c", "other/q"), "other/q");
    }
}
