// Edit mode's changes to a quest pack: an operator's book asks for one
// `Edit` at a time, the server applies it to the pack in use, checks the
// result as the loader would (hq_validate) and saves the whole pack. The
// book applies the same edit to its own copy at once, so a moved quest does
// not jump back while the server answers; the server's pack then replaces
// it. Pure: no game types.
//
// Ids stay as they are once made: progress files refer to full ids. A new
// quest gets a fresh id from its title (`new_quest`, `new_quest_2`...).
use super::hq_model::{
    AutoClaim, Chapter, DependencyMode, Group, Progression, Quest, QuestFile, Reward, RewardKind, RewardTable, Settings, Shape, TableEntry, Task,
    Visibility, child_id, dependency_id, quest_id,
};
use super::hq_validate::{self as validate, Severity};
use serde::{Deserialize, Serialize};

/// Where the pack in use comes from (`ToClient::Editor`): the world's own
/// folder, the modpack's (`config/horizon_quests/quests/`), the data packs,
/// the pack built into the mod, or nowhere.
pub const SOURCE_WORLD: &str = "world";
pub const SOURCE_MODPACK: &str = "modpack";
pub const SOURCE_DATA_PACKS: &str = "data_packs";
pub const SOURCE_BUILT_IN: &str = "built_in";
pub const SOURCE_NONE: &str = "none";

/// The op level that may edit the quests, as `/quests reload` needs.
pub const EDIT_LEVEL: u8 = 2;

/// The title of a quest made in the book.
pub const NEW_QUEST: &str = "New quest";
/// How far from the canvas's middle a quest may be put, in grid cells.
const FAR: f32 = 10_000.0;

/// One change to the pack. Quests are named by full id (`chapter/quest`),
/// places in grid cells.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Edit {
    /// A new quest in chapter `chapter` (its id) at (`x`, `y`), empty.
    NewQuest { chapter: String, x: f32, y: f32 },
    MoveQuest { quest: String, x: f32, y: f32 },
    /// A copy of the quest, with its tasks, rewards and dependencies, at
    /// (`x`, `y`) in the same chapter.
    DuplicateQuest { quest: String, x: f32, y: f32 },
    /// Removes the quest, and every other quest's dependency on it.
    DeleteQuest { quest: String },
    /// Makes `quest` need `needs` (`on`), or stop needing it.
    Link { quest: String, needs: String, on: bool },
    /// The quest form's fields: everything but the place, the links, the
    /// tasks and the rewards.
    SetQuestInfo { quest: String, info: QuestInfo },
    /// Replaces the quest's task `old` (keeping its id), or adds `task`
    /// with a fresh id from its type when `old` is None.
    SaveTask { quest: String, old: Option<String>, task: Task },
    RemoveTask { quest: String, task: String },
    SaveReward { quest: String, old: Option<String>, reward: Reward },
    /// A random (or, with `choice`, a chosen) reward drawn from a new table
    /// of its own, whose first entry is one `item`.
    NewTableReward { quest: String, choice: bool, item: String },
    RemoveReward { quest: String, reward: String },
    /// A new empty chapter at the end, its id from its title.
    NewChapter { title: String },
    SetChapter { chapter: String, info: ChapterInfo },
    /// Moves the chapter before (`by` < 0) or after the next chapter of
    /// its group in the list.
    MoveChapter { chapter: String, by: i32 },
    /// Removes the chapter, its quests, and other chapters' links to them.
    DeleteChapter { chapter: String },
    NewGroup { title: String },
    SetGroup { group: String, title: String },
    /// Removes the group; its chapters stay, without a group.
    DeleteGroup { group: String },
    SetSettings { settings: PackSettings },
    /// A new reward table with one entry (a table cannot be empty).
    NewTable,
    /// Replaces entry `index` of the table, or adds `entry` at the end.
    SaveEntry { table: String, index: Option<usize>, entry: TableEntry },
    RemoveEntry { table: String, index: usize },
    /// Gives entry `index` of a random table a `percent` chance: the
    /// weights become percentages, the other entries keeping their shares
    /// of the rest.
    SetChance { table: String, index: usize, percent: u32 },
    DeleteTable { table: String },
}

/// What the chapter form edits.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ChapterInfo {
    pub title: String,
    pub subtitle: Option<String>,
    pub icon: Option<String>,
    pub group: Option<String>,
    pub hidden: bool,
    pub progression: Option<Progression>,
    pub default_shape: Option<Shape>,
    pub default_size: Option<f32>,
}

impl ChapterInfo {
    pub fn of(chapter: &Chapter) -> Self {
        Self {
            title: chapter.title.clone(),
            subtitle: chapter.subtitle.clone(),
            icon: chapter.icon.clone(),
            group: chapter.group.clone(),
            hidden: chapter.hidden,
            progression: chapter.progression,
            default_shape: chapter.default_shape,
            default_size: chapter.default_size,
        }
    }

    fn write(&self, chapter: &mut Chapter) {
        chapter.title = self.title.clone();
        chapter.subtitle = self.subtitle.clone();
        chapter.icon = self.icon.clone();
        chapter.group = self.group.clone();
        chapter.hidden = self.hidden;
        chapter.progression = self.progression;
        chapter.default_shape = self.default_shape;
        chapter.default_size = self.default_size;
    }
}

/// What the pack settings form edits (quests.json, its groups aside).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PackSettings {
    pub progression: Progression,
    pub default_shape: Shape,
    pub default_size: f32,
    pub auto_claim_rewards: bool,
    pub detection_delay: u32,
}

impl PackSettings {
    pub fn of(settings: &Settings) -> Self {
        Self {
            progression: settings.progression,
            default_shape: settings.default_shape,
            default_size: settings.default_size,
            auto_claim_rewards: settings.auto_claim_rewards,
            detection_delay: settings.detection_delay,
        }
    }
}

/// What the quest form edits.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct QuestInfo {
    pub title: String,
    pub subtitle: Option<String>,
    pub description: Vec<String>,
    pub icon: Option<String>,
    pub shape: Option<Shape>,
    pub size: Option<f32>,
    pub visibility: Visibility,
    pub dependency_mode: DependencyMode,
    pub min_required: Option<u32>,
    pub optional: bool,
    pub sequential_tasks: bool,
    pub repeatable: bool,
    pub repeat_cooldown: u32,
}

impl QuestInfo {
    pub fn of(quest: &Quest) -> Self {
        Self {
            title: quest.title.clone(),
            subtitle: quest.subtitle.clone(),
            description: quest.description.clone(),
            icon: quest.icon.clone(),
            shape: quest.shape,
            size: quest.size,
            visibility: quest.visibility,
            dependency_mode: quest.dependency_mode,
            min_required: quest.min_required,
            optional: quest.optional,
            sequential_tasks: quest.sequential_tasks,
            repeatable: quest.repeatable,
            repeat_cooldown: quest.repeat_cooldown,
        }
    }

    fn write(&self, quest: &mut Quest) {
        quest.title = self.title.clone();
        quest.subtitle = self.subtitle.clone();
        quest.description = self.description.clone();
        quest.icon = self.icon.clone();
        quest.shape = self.shape;
        quest.size = self.size;
        quest.visibility = self.visibility;
        quest.dependency_mode = self.dependency_mode;
        quest.min_required = self.min_required;
        quest.optional = self.optional;
        quest.sequential_tasks = self.sequential_tasks;
        quest.repeatable = self.repeatable;
        quest.repeat_cooldown = self.repeat_cooldown;
    }
}

/// The chapter and place of quest `id` (a full id).
fn locate(file: &QuestFile, id: &str) -> Option<(usize, usize)> {
    let (chapter, quest) = id.split_once('/')?;
    let c = file.chapters.iter().position(|ch| ch.id == chapter)?;
    let q = file.chapters[c].quests.iter().position(|qu| qu.id == quest)?;
    Some((c, q))
}

fn chapter_at(file: &QuestFile, chapter: &str) -> Result<usize, String> {
    file.chapters.iter().position(|c| c.id == chapter).ok_or_else(|| format!("There is no chapter {chapter} any more."))
}

fn table_at(file: &QuestFile, table: &str) -> Result<usize, String> {
    file.reward_tables.iter().position(|t| t.id == table).ok_or_else(|| format!("There is no reward table {table} any more."))
}

fn place(x: f32, y: f32) -> Result<(f32, f32), String> {
    if x.is_finite() && y.is_finite() && x.abs() <= FAR && y.abs() <= FAR {
        Ok((x, y))
    } else {
        Err("That place is off the canvas.".to_owned())
    }
}

/// `base` in lower case with `_` for anything that cannot be in an id.
pub fn slug(base: &str) -> String {
    let mut id = String::new();
    for c in base.chars().flat_map(char::to_lowercase) {
        if c.is_ascii_lowercase() || c.is_ascii_digit() {
            id.push(c);
        } else if !id.ends_with('_') && !id.is_empty() {
            id.push('_');
        }
    }
    let id = id.trim_end_matches('_');
    if id.is_empty() { "quest".to_owned() } else { id.chars().take(40).collect() }
}

/// An id made from `base` that `taken` says nothing has yet.
fn fresh(base: &str, taken: impl Fn(&str) -> bool) -> String {
    let base = slug(base);
    if !taken(&base) {
        return base;
    }
    (2..).map(|k| format!("{base}_{k}")).find(|id| !taken(id)).unwrap_or(base)
}

/// An id made from `base` that no quest of chapter `c` has yet.
fn fresh_id(file: &QuestFile, c: usize, base: &str) -> String {
    fresh(base, |id| file.chapters[c].quests.iter().any(|q| q.id == id))
}

/// The `type` a task or reward is written with (`item`, `xp_levels`...).
fn type_name(value: &impl Serialize) -> String {
    serde_json::to_value(value).ok().and_then(|v| v.get("type")?.as_str().map(str::to_owned)).unwrap_or_default()
}

/// Puts `value` among `list` in place of the one with id `old`, or at the
/// end with a fresh id from its type. `id` reads and writes an entry's id.
/// Returns the new entry's id, when it is new.
fn save_child<T: Serialize>(list: &mut Vec<T>, old: &Option<String>, mut value: T, id: fn(&mut T) -> &mut String, what: &str) -> Result<Option<String>, String> {
    match old {
        Some(old) => {
            let at = list.iter_mut().position(|entry| id(entry) == old).ok_or_else(|| format!("There is no {what} {old} any more."))?;
            *id(&mut value) = old.clone();
            list[at] = value;
            Ok(None)
        }
        None => {
            let ids: Vec<String> = list.iter_mut().map(|entry| id(entry).clone()).collect();
            let made = fresh(&type_name(&value), |taken| ids.iter().any(|i| i == taken));
            *id(&mut value) = made.clone();
            list.push(value);
            Ok(Some(made))
        }
    }
}

/// Removes the entry of `list` with id `old`.
fn remove_child<T>(list: &mut Vec<T>, old: &str, id: fn(&mut T) -> &mut String, what: &str) -> Result<(), String> {
    let at = list.iter_mut().position(|entry| id(entry) == old).ok_or_else(|| format!("There is no {what} {old} any more."))?;
    list.remove(at);
    Ok(())
}

/// Applies `edit` to `file`. Returns the full id of the quest it made, if
/// any. The result is not checked yet: see `check`.
pub fn apply(file: &mut QuestFile, edit: &Edit) -> Result<Option<String>, String> {
    // A table no reward uses any more after a reward, quest or chapter goes
    // (or a reward changes table) goes with it: each drawn reward has its own.
    let prunes = matches!(
        edit,
        Edit::RemoveReward { .. } | Edit::SaveReward { .. } | Edit::DeleteQuest { .. } | Edit::DeleteChapter { .. }
    );
    let before = if prunes { used_tables(file) } else { Vec::new() };
    let made = apply_one(file, edit)?;
    if prunes {
        let after = used_tables(file);
        file.reward_tables.retain(|t| !before.contains(&t.id) || after.contains(&t.id));
    }
    Ok(made)
}

/// The tables the pack's rewards draw from.
fn used_tables(file: &QuestFile) -> Vec<String> {
    file.chapters
        .iter()
        .flat_map(|c| &c.quests)
        .flat_map(|q| &q.rewards)
        .filter_map(|r| match &r.kind {
            RewardKind::Random { table } | RewardKind::Choice { table } => Some(table.clone()),
            _ => None,
        })
        .collect()
}

/// Weights giving entry `index` `percent` of 100, the others sharing the
/// rest as their `weights` did (equally when they were all 0).
pub fn chances(weights: &[u32], index: usize, percent: u32) -> Vec<u32> {
    if weights.len() <= 1 {
        return vec![100; weights.len()];
    }
    let percent = percent.clamp(1, 99);
    let rest = 100 - percent;
    let others: u32 = weights.iter().enumerate().filter(|(i, _)| *i != index).map(|(_, w)| *w).sum();
    let count = weights.len() as u32 - 1;
    let mut out: Vec<u32> = weights
        .iter()
        .enumerate()
        .map(|(i, w)| {
            if i == index {
                percent
            } else if others == 0 {
                rest / count
            } else {
                ((u64::from(*w) * u64::from(rest) + u64::from(others) / 2) / u64::from(others)) as u32
            }
        })
        .collect();
    // Rounding: the largest other entry takes what is missing or over.
    let sum: u32 = out.iter().sum();
    if let Some(big) = (0..out.len()).filter(|&i| i != index).max_by_key(|&i| out[i]) {
        out[big] = (out[big] + 100).saturating_sub(sum).max(if sum > 100 { 0 } else { 1 });
    }
    out
}

fn apply_one(file: &mut QuestFile, edit: &Edit) -> Result<Option<String>, String> {
    let missing = |id: &str| format!("There is no quest {id} any more.");
    match edit {
        Edit::NewQuest { chapter, x, y } => {
            let (x, y) = place(*x, *y)?;
            let c = chapter_at(file, chapter)?;
            let id = fresh_id(file, c, NEW_QUEST);
            file.chapters[c].quests.push(Quest { id: id.clone(), title: NEW_QUEST.to_owned(), x, y, ..Quest::default() });
            Ok(Some(quest_id(chapter, &id)))
        }
        Edit::MoveQuest { quest, x, y } => {
            let (x, y) = place(*x, *y)?;
            let (c, q) = locate(file, quest).ok_or_else(|| missing(quest))?;
            let moved = &mut file.chapters[c].quests[q];
            (moved.x, moved.y) = (x, y);
            Ok(None)
        }
        Edit::DuplicateQuest { quest, x, y } => {
            let (x, y) = place(*x, *y)?;
            let (c, q) = locate(file, quest).ok_or_else(|| missing(quest))?;
            let mut copy = file.chapters[c].quests[q].clone();
            copy.id = fresh_id(file, c, &format!("{}_copy", copy.id));
            (copy.x, copy.y) = (x, y);
            let id = quest_id(&file.chapters[c].id, &copy.id);
            file.chapters[c].quests.insert(q + 1, copy);
            Ok(Some(id))
        }
        Edit::DeleteQuest { quest } => {
            let (c, q) = locate(file, quest).ok_or_else(|| missing(quest))?;
            file.chapters[c].quests.remove(q);
            for chapter in &mut file.chapters {
                let here = chapter.id.clone();
                for other in &mut chapter.quests {
                    other.dependencies.retain(|d| dependency_id(&here, d) != *quest);
                }
            }
            Ok(None)
        }
        Edit::Link { quest, needs, on } => {
            if quest == needs {
                return Err("A quest cannot need itself.".to_owned());
            }
            locate(file, needs).ok_or_else(|| missing(needs))?;
            let (c, q) = locate(file, quest).ok_or_else(|| missing(quest))?;
            let here = file.chapters[c].id.clone();
            let deps = &mut file.chapters[c].quests[q].dependencies;
            deps.retain(|d| dependency_id(&here, d) != *needs);
            if *on {
                // Same chapter: the short id, as pack authors write it.
                let short = needs.strip_prefix(&format!("{here}/")).unwrap_or(needs);
                deps.push(short.to_owned());
            }
            Ok(None)
        }
        Edit::SetQuestInfo { quest, info } => {
            if info.size.is_some_and(|size| !(0.25..=4.0).contains(&size)) {
                return Err("A quest's size goes from 0.25 to 4.".to_owned());
            }
            let (c, q) = locate(file, quest).ok_or_else(|| missing(quest))?;
            info.write(&mut file.chapters[c].quests[q]);
            Ok(None)
        }
        Edit::SaveTask { quest, old, task } => {
            let (c, q) = locate(file, quest).ok_or_else(|| missing(quest))?;
            let made = save_child(&mut file.chapters[c].quests[q].tasks, old, task.clone(), |t| &mut t.id, "task")?;
            Ok(made.map(|id| child_id(quest, &id)))
        }
        Edit::RemoveTask { quest, task } => {
            let (c, q) = locate(file, quest).ok_or_else(|| missing(quest))?;
            remove_child(&mut file.chapters[c].quests[q].tasks, task, |t| &mut t.id, "task")?;
            Ok(None)
        }
        Edit::SaveReward { quest, old, reward } => {
            let (c, q) = locate(file, quest).ok_or_else(|| missing(quest))?;
            let made = save_child(&mut file.chapters[c].quests[q].rewards, old, reward.clone(), |r| &mut r.id, "reward")?;
            Ok(made.map(|id| child_id(quest, &id)))
        }
        Edit::NewTableReward { quest, choice, item } => {
            let (c, q) = locate(file, quest).ok_or_else(|| missing(quest))?;
            let base = format!("{}_{}", file.chapters[c].quests[q].id, if *choice { "choice" } else { "loot" });
            let table = fresh(&base, |id| file.reward_tables.iter().any(|t| t.id == id));
            let first = RewardKind::Item { item: item.clone(), count: 1, max_count: None };
            file.reward_tables.push(RewardTable { id: table.clone(), entries: vec![TableEntry { weight: 1, title: None, kind: first }] });
            let kind = if *choice { RewardKind::Choice { table } } else { RewardKind::Random { table } };
            let reward = Reward { id: String::new(), title: None, auto_claim: AutoClaim::Default, team: false, kind };
            let made = save_child(&mut file.chapters[c].quests[q].rewards, &None, reward, |r| &mut r.id, "reward")?;
            Ok(made.map(|id| child_id(quest, &id)))
        }
        Edit::RemoveReward { quest, reward } => {
            let (c, q) = locate(file, quest).ok_or_else(|| missing(quest))?;
            remove_child(&mut file.chapters[c].quests[q].rewards, reward, |r| &mut r.id, "reward")?;
            Ok(None)
        }
        Edit::NewChapter { title } => {
            let id = fresh(title, |id| file.chapters.iter().any(|c| c.id == id));
            let order = file.chapters.iter().map(|c| c.order).max().unwrap_or(0);
            file.chapters.push(Chapter { id: id.clone(), title: title.clone(), order, ..Chapter::default() });
            Ok(Some(id))
        }
        Edit::SetChapter { chapter, info } => {
            if info.default_size.is_some_and(|size| !(0.25..=4.0).contains(&size)) {
                return Err("A size goes from 0.25 to 4.".to_owned());
            }
            let c = chapter_at(file, chapter)?;
            info.write(&mut file.chapters[c]);
            Ok(None)
        }
        Edit::MoveChapter { chapter, by } => {
            let c = chapter_at(file, chapter)?;
            let group = file.chapters[c].group.clone();
            let same: Vec<usize> = (0..file.chapters.len()).filter(|&i| file.chapters[i].group == group).collect();
            let at = same.iter().position(|&i| i == c).unwrap_or(0) as i32;
            let Some(&other) = usize::try_from(at + by.signum()).ok().and_then(|to| same.get(to)) else {
                return Ok(None);
            };
            file.chapters.swap(c, other);
            // The loader orders chapters by `order`: number them as they stand.
            for (i, chapter) in file.chapters.iter_mut().enumerate() {
                chapter.order = i as i32;
            }
            Ok(None)
        }
        Edit::DeleteChapter { chapter } => {
            if file.chapters.len() <= 1 {
                return Err("A pack keeps at least one chapter.".to_owned());
            }
            let c = chapter_at(file, chapter)?;
            file.chapters.remove(c);
            let gone = format!("{chapter}/");
            for other in &mut file.chapters {
                let here = other.id.clone();
                for quest in &mut other.quests {
                    quest.dependencies.retain(|d| !dependency_id(&here, d).starts_with(&gone));
                }
            }
            Ok(None)
        }
        Edit::NewGroup { title } => {
            let groups = &mut file.settings.groups;
            let id = fresh(title, |id| groups.iter().any(|g| g.id == id));
            groups.push(Group { id, title: title.clone() });
            Ok(None)
        }
        Edit::SetGroup { group, title } => {
            let found = file.settings.groups.iter_mut().find(|g| g.id == *group).ok_or_else(|| format!("There is no group {group} any more."))?;
            found.title = title.clone();
            Ok(None)
        }
        Edit::DeleteGroup { group } => {
            file.settings.groups.retain(|g| g.id != *group);
            for chapter in &mut file.chapters {
                if chapter.group.as_deref() == Some(group) {
                    chapter.group = None;
                }
            }
            Ok(None)
        }
        Edit::SetSettings { settings } => {
            if !(0.25..=4.0).contains(&settings.default_size) || settings.detection_delay == 0 {
                return Err("The size goes from 0.25 to 4, the delay is at least 1 tick.".to_owned());
            }
            let s = &mut file.settings;
            s.progression = settings.progression;
            s.default_shape = settings.default_shape;
            s.default_size = settings.default_size;
            s.auto_claim_rewards = settings.auto_claim_rewards;
            s.detection_delay = settings.detection_delay;
            Ok(None)
        }
        Edit::NewTable => {
            let id = fresh("table", |id| file.reward_tables.iter().any(|t| t.id == id));
            let bread = RewardKind::Item { item: "minecraft:bread".into(), count: 1, max_count: None };
            file.reward_tables.push(RewardTable { id, entries: vec![TableEntry { weight: 1, title: None, kind: bread }] });
            Ok(None)
        }
        Edit::SaveEntry { table, index, entry } => {
            let t = table_at(file, table)?;
            let entries = &mut file.reward_tables[t].entries;
            match index {
                Some(i) => *entries.get_mut(*i).ok_or_else(|| "That entry is not there any more.".to_owned())? = entry.clone(),
                None => entries.push(entry.clone()),
            }
            Ok(None)
        }
        Edit::RemoveEntry { table, index } => {
            let t = table_at(file, table)?;
            let entries = &mut file.reward_tables[t].entries;
            if entries.len() <= 1 {
                return Err("A table keeps at least one entry: delete the table instead.".to_owned());
            }
            if *index >= entries.len() {
                return Err("That entry is not there any more.".to_owned());
            }
            entries.remove(*index);
            Ok(None)
        }
        Edit::SetChance { table, index, percent } => {
            let t = table_at(file, table)?;
            let entries = &mut file.reward_tables[t].entries;
            if *index >= entries.len() {
                return Err("That entry is not there any more.".to_owned());
            }
            let weights: Vec<u32> = entries.iter().map(|e| e.weight).collect();
            for (entry, weight) in entries.iter_mut().zip(chances(&weights, *index, *percent)) {
                entry.weight = weight;
            }
            Ok(None)
        }
        Edit::DeleteTable { table } => {
            let t = table_at(file, table)?;
            file.reward_tables.remove(t);
            Ok(None)
        }
    }
}

/// The data packs' folder of advancements (vanilla's, mods', the world's).
pub const ADVANCEMENT_FOLDER: &str = "advancement";

/// The advancements a task may name, from their files as the data load reads
/// them (`namespace:path`, JSON): (id, icon item), sorted by id. Those with
/// no `display` (recipe unlocks, hidden helpers) are left out.
pub fn advancements(files: &[(String, serde_json::Value)]) -> Vec<(String, String)> {
    let mut found: Vec<(String, String)> = files
        .iter()
        .filter_map(|(id, json)| {
            let display = json.get("display")?.as_object()?;
            let icon = display.get("icon");
            let item = icon
                .and_then(|i| i.get("id").or_else(|| i.get("item")))
                .and_then(|i| i.as_str())
                .or_else(|| icon.and_then(|i| i.as_str()))
                .unwrap_or("minecraft:knowledge_book");
            let item = if item.contains(':') { item.to_owned() } else { format!("minecraft:{item}") };
            Some((id.clone(), item))
        })
        .collect();
    found.sort();
    found
}

/// `file` as the loader reads it back, when that leaves nothing out and
/// finds no error; else why not, in a line for the book.
pub fn check(file: &QuestFile) -> Result<QuestFile, String> {
    fn json<T: Serialize>(value: &T) -> String {
        serde_json::to_string(value).unwrap_or_default()
    }
    // Numbered so the loader keeps the chapters' order.
    let chapters: Vec<(String, String)> =
        file.chapters.iter().enumerate().map(|(i, chapter)| (format!("{i:03}.json"), json(chapter))).collect();
    let tables: Vec<(String, String)> = file.reward_tables.iter().map(|table| (format!("{}.json", table.id), json(table))).collect();
    fn as_refs(files: &[(String, String)]) -> Vec<(&str, &str)> {
        files.iter().map(|(name, text)| (name.as_str(), text.as_str())).collect()
    }
    let loaded = validate::load_pack(Some(&json(&file.settings)), &as_refs(&chapters), &as_refs(&tables));
    if let Some(problem) = loaded.problems.iter().find(|p| p.severity == Severity::Error) {
        return Err(format!("Not saved: {} ({})", problem.message, problem.place));
    }
    if loaded.file.chapters.len() != file.chapters.len() || loaded.file.reward_tables.len() != file.reward_tables.len() {
        return Err("Not saved: the quests would not load back whole.".to_owned());
    }
    Ok(loaded.file)
}

/// Where a quest dropped at grid place `at` goes: the nearest half cell.
pub fn snap(at: [f32; 2]) -> [f32; 2] {
    [(at[0] * 2.0).round() / 2.0, (at[1] * 2.0).round() / 2.0]
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::hq_model::{Chapter, TaskKind};

    fn pack() -> QuestFile {
        let quest = |id: &str, deps: &[&str]| Quest {
            id: id.to_owned(),
            title: id.to_owned(),
            dependencies: deps.iter().map(|d| (*d).to_owned()).collect(),
            tasks: vec![Task { id: "t".to_owned(), title: None, kind: TaskKind::Checkmark }],
            ..Quest::default()
        };
        let one = Chapter { id: "one".to_owned(), title: "One".to_owned(), quests: vec![quest("a", &[]), quest("b", &["a"])], ..Chapter::default() };
        let two = Chapter { id: "two".to_owned(), title: "Two".to_owned(), order: 1, quests: vec![quest("c", &["one/b"])], ..Chapter::default() };
        QuestFile { chapters: vec![one, two], ..QuestFile::default() }
    }

    #[test]
    fn chances_are_percentages_and_unused_tables_go() {
        assert_eq!(chances(&[1, 1, 2], 0, 50), [50, 17, 33]);
        assert_eq!(chances(&[1, 1, 2], 0, 50).iter().sum::<u32>(), 100);
        assert_eq!(chances(&[5], 0, 30), [100]);
        assert_eq!(chances(&[0, 0, 0], 2, 40), [30, 30, 40]);
        assert_eq!(chances(&[3, 1], 1, 100), [1, 99]);
        let mut file = pack();
        apply(&mut file, &Edit::NewTableReward { quest: "one/a".into(), choice: false, item: "minecraft:apple".into() }).unwrap();
        let entry = TableEntry { weight: 1, title: None, kind: RewardKind::Xp { points: 5 } };
        apply(&mut file, &Edit::SaveEntry { table: "a_loot".into(), index: None, entry }).unwrap();
        apply(&mut file, &Edit::SetChance { table: "a_loot".into(), index: 1, percent: 25 }).unwrap();
        let weights: Vec<u32> = file.reward_tables.iter().find(|t| t.id == "a_loot").unwrap().entries.iter().map(|e| e.weight).collect();
        assert_eq!(weights, [75, 25]);
        // The reward goes: its table, used by nothing else, goes too.
        apply(&mut file, &Edit::RemoveReward { quest: "one/a".into(), reward: "random".into() }).unwrap();
        assert!(file.reward_tables.iter().all(|t| t.id != "a_loot"));
    }

    #[test]
    fn advancements_are_read_from_their_files() {
        let files: Vec<(String, serde_json::Value)> = vec![
            ("minecraft:story/mine_stone".into(), serde_json::json!({ "display": { "icon": { "id": "minecraft:wooden_pickaxe" } } })),
            ("minecraft:recipes/misc/bread".into(), serde_json::json!({ "criteria": {} })),
            ("mymod:root".into(), serde_json::json!({ "display": { "icon": { "item": "stone" } } })),
            ("minecraft:adventure/root".into(), serde_json::json!({ "display": {} })),
        ];
        assert_eq!(
            advancements(&files),
            [
                ("minecraft:adventure/root".to_owned(), "minecraft:knowledge_book".to_owned()),
                ("minecraft:story/mine_stone".to_owned(), "minecraft:wooden_pickaxe".to_owned()),
                ("mymod:root".to_owned(), "minecraft:stone".to_owned()),
            ]
        );
    }

    #[test]
    fn a_new_quest_gets_a_fresh_id_and_nothing_else() {
        let mut file = pack();
        let first = apply(&mut file, &Edit::NewQuest { chapter: "one".into(), x: 2.0, y: 1.5 }).unwrap();
        let second = apply(&mut file, &Edit::NewQuest { chapter: "one".into(), x: 3.0, y: 1.5 }).unwrap();
        assert_eq!(first.as_deref(), Some("one/new_quest"));
        assert_eq!(second.as_deref(), Some("one/new_quest_2"));
        let made = &file.chapters[0].quests[2];
        assert_eq!((made.x, made.y, made.title.as_str()), (2.0, 1.5, NEW_QUEST));
        assert!(made.tasks.is_empty() && made.rewards.is_empty());
        check(&file).unwrap();
    }

    #[test]
    fn moving_changes_only_the_place() {
        let mut file = pack();
        apply(&mut file, &Edit::MoveQuest { quest: "one/b".into(), x: -4.5, y: 2.0 }).unwrap();
        let b = &file.chapters[0].quests[1];
        assert_eq!((b.x, b.y), (-4.5, 2.0));
        assert!(apply(&mut file, &Edit::MoveQuest { quest: "one/zz".into(), x: 0.0, y: 0.0 }).is_err());
        assert!(apply(&mut file, &Edit::MoveQuest { quest: "one/a".into(), x: f32::NAN, y: 0.0 }).is_err());
    }

    #[test]
    fn a_copy_keeps_tasks_and_dependencies_under_a_new_id() {
        let mut file = pack();
        let id = apply(&mut file, &Edit::DuplicateQuest { quest: "one/b".into(), x: 5.0, y: 0.0 }).unwrap();
        assert_eq!(id.as_deref(), Some("one/b_copy"));
        let copy = &file.chapters[0].quests[2];
        assert_eq!(copy.dependencies, vec!["a".to_owned()]);
        assert_eq!(copy.tasks.len(), 1);
        check(&file).unwrap();
    }

    #[test]
    fn deleting_a_quest_drops_the_dependencies_on_it_in_every_chapter() {
        let mut file = pack();
        apply(&mut file, &Edit::DeleteQuest { quest: "one/b".into() }).unwrap();
        assert_eq!(file.chapters[0].quests.len(), 1);
        assert!(file.chapters[1].quests[0].dependencies.is_empty());
        check(&file).unwrap();
    }

    #[test]
    fn links_are_short_in_the_same_chapter_and_full_across() {
        let mut file = pack();
        apply(&mut file, &Edit::Link { quest: "one/a".into(), needs: "two/c".into(), on: true }).unwrap();
        assert_eq!(file.chapters[0].quests[0].dependencies, vec!["two/c".to_owned()]);
        // a needs c, c needs b, b needs a: a cycle the check refuses.
        assert!(check(&file).is_err());
        apply(&mut file, &Edit::Link { quest: "one/a".into(), needs: "two/c".into(), on: false }).unwrap();
        assert!(file.chapters[0].quests[0].dependencies.is_empty());
        apply(&mut file, &Edit::Link { quest: "two/c".into(), needs: "one/a".into(), on: true }).unwrap();
        assert_eq!(file.chapters[1].quests[0].dependencies, vec!["one/b".to_owned(), "one/a".to_owned()]);
        assert!(apply(&mut file, &Edit::Link { quest: "one/a".into(), needs: "one/a".into(), on: true }).is_err());
        check(&file).unwrap();
    }

    #[test]
    fn the_form_changes_the_info_and_keeps_tasks_links_and_place() {
        let mut file = pack();
        let mut info = QuestInfo::of(&file.chapters[0].quests[1]);
        info.title = "Bee".into();
        info.description = vec!["One".into(), "".into(), "Two".into()];
        info.shape = Some(Shape::Diamond);
        info.icon = Some("minecraft:stone".into());
        apply(&mut file, &Edit::SetQuestInfo { quest: "one/b".into(), info: info.clone() }).unwrap();
        let b = &file.chapters[0].quests[1];
        assert_eq!(QuestInfo::of(b), info);
        assert_eq!((b.dependencies.len(), b.tasks.len()), (1, 1));
        info.size = Some(9.0);
        assert!(apply(&mut file, &Edit::SetQuestInfo { quest: "one/b".into(), info }).is_err());
        check(&file).unwrap();
    }

    #[test]
    fn tasks_and_rewards_are_added_replaced_and_removed() {
        use super::super::hq_model::{AutoClaim, ItemMatch, RewardKind};
        let mut file = pack();
        let item = Task { id: String::new(), title: None, kind: TaskKind::Item { item: ItemMatch::Id("minecraft:stone".into()), count: 3, consume: false } };
        let edit = |old: Option<&str>, task: &Task| Edit::SaveTask { quest: "one/a".into(), old: old.map(str::to_owned), task: task.clone() };
        assert_eq!(apply(&mut file, &edit(None, &item)).unwrap().as_deref(), Some("one/a/item"));
        apply(&mut file, &edit(None, &item)).unwrap();
        let ids: Vec<&str> = file.chapters[0].quests[0].tasks.iter().map(|t| t.id.as_str()).collect();
        assert_eq!(ids, ["t", "item", "item_2"]);
        // Replaced in place: the id stays, whatever the form sent.
        let kill = Task { id: "ignored".into(), title: Some("Zombies".into()), kind: TaskKind::Kill { entity: "minecraft:zombie".into(), count: 2 } };
        apply(&mut file, &edit(Some("item"), &kill)).unwrap();
        let replaced = &file.chapters[0].quests[0].tasks[1];
        assert_eq!((replaced.id.as_str(), replaced.title.as_deref()), ("item", Some("Zombies")));
        apply(&mut file, &Edit::RemoveTask { quest: "one/a".into(), task: "item_2".into() }).unwrap();
        assert_eq!(file.chapters[0].quests[0].tasks.len(), 2);
        assert!(apply(&mut file, &Edit::RemoveTask { quest: "one/a".into(), task: "item_2".into() }).is_err());
        assert!(apply(&mut file, &edit(Some("gone"), &kill)).is_err());

        let reward = Reward { id: String::new(), title: None, auto_claim: AutoClaim::Default, team: false, kind: RewardKind::XpLevels { levels: 3 } };
        apply(&mut file, &Edit::SaveReward { quest: "one/a".into(), old: None, reward }).unwrap();
        assert_eq!(file.chapters[0].quests[0].rewards[0].id, "xp_levels");
        check(&file).unwrap();
        apply(&mut file, &Edit::RemoveReward { quest: "one/a".into(), reward: "xp_levels".into() }).unwrap();
        assert!(file.chapters[0].quests[0].rewards.is_empty());
        // A drawn reward gets a table of its own, its first entry the item picked.
        for (choice, table, made) in [(false, "a_loot", "one/a/random"), (true, "a_choice", "one/a/choice")] {
            let id = apply(&mut file, &Edit::NewTableReward { quest: "one/a".into(), choice, item: "minecraft:apple".into() }).unwrap();
            assert_eq!(id.as_deref(), Some(made));
            let t = file.reward_tables.iter().find(|t| t.id == table).expect("its table");
            assert_eq!(t.entries.len(), 1);
            check(&file).unwrap();
        }
    }

    #[test]
    fn chapters_are_made_moved_within_their_group_and_deleted_with_their_links() {
        let mut file = pack();
        let made = apply(&mut file, &Edit::NewChapter { title: "Three".into() }).unwrap();
        assert_eq!(made.as_deref(), Some("three"));
        check(&file).unwrap();
        let ids = |file: &QuestFile| file.chapters.iter().map(|c| c.id.clone()).collect::<Vec<_>>();
        apply(&mut file, &Edit::MoveChapter { chapter: "three".into(), by: -1 }).unwrap();
        assert_eq!(ids(&file), ["one", "three", "two"]);
        // A chapter in a group moves among that group's chapters only.
        apply(&mut file, &Edit::NewGroup { title: "Side paths".into() }).unwrap();
        let mut info = ChapterInfo::of(&file.chapters[2]);
        info.group = Some("side_paths".into());
        apply(&mut file, &Edit::SetChapter { chapter: "two".into(), info }).unwrap();
        apply(&mut file, &Edit::MoveChapter { chapter: "two".into(), by: -1 }).unwrap();
        assert_eq!(ids(&file), ["one", "three", "two"]);
        check(&file).unwrap();
        apply(&mut file, &Edit::DeleteGroup { group: "side_paths".into() }).unwrap();
        assert_eq!(file.chapters[2].group, None);
        // Deleting "one" drops "two/c"'s link to "one/b".
        apply(&mut file, &Edit::DeleteChapter { chapter: "one".into() }).unwrap();
        assert!(file.chapters[1].quests[0].dependencies.is_empty());
        apply(&mut file, &Edit::DeleteChapter { chapter: "three".into() }).unwrap();
        assert!(apply(&mut file, &Edit::DeleteChapter { chapter: "two".into() }).is_err());
        check(&file).unwrap();
    }

    #[test]
    fn tables_keep_one_entry_and_a_used_table_cannot_go() {
        use super::super::hq_model::AutoClaim;
        let mut file = pack();
        apply(&mut file, &Edit::NewTable).unwrap();
        assert_eq!(file.reward_tables[0].id, "table");
        let xp = TableEntry { weight: 3, title: None, kind: RewardKind::Xp { points: 5 } };
        apply(&mut file, &Edit::SaveEntry { table: "table".into(), index: None, entry: xp.clone() }).unwrap();
        apply(&mut file, &Edit::SaveEntry { table: "table".into(), index: Some(0), entry: xp.clone() }).unwrap();
        assert_eq!(file.reward_tables[0].entries, vec![xp.clone(), xp]);
        apply(&mut file, &Edit::RemoveEntry { table: "table".into(), index: 1 }).unwrap();
        assert!(apply(&mut file, &Edit::RemoveEntry { table: "table".into(), index: 0 }).is_err());
        let draw = Reward { id: String::new(), title: None, auto_claim: AutoClaim::Default, team: false, kind: RewardKind::Random { table: "table".into() } };
        apply(&mut file, &Edit::SaveReward { quest: "one/a".into(), old: None, reward: draw }).unwrap();
        check(&file).unwrap();
        apply(&mut file, &Edit::DeleteTable { table: "table".into() }).unwrap();
        assert!(check(&file).is_err());
    }

    #[test]
    fn pack_settings_are_checked() {
        let mut file = pack();
        let mut settings = PackSettings::of(&file.settings);
        settings.progression = Progression::Flexible;
        apply(&mut file, &Edit::SetSettings { settings: settings.clone() }).unwrap();
        assert_eq!(PackSettings::of(&file.settings), settings);
        settings.detection_delay = 0;
        assert!(apply(&mut file, &Edit::SetSettings { settings }).is_err());
    }

    #[test]
    fn ids_come_from_titles() {
        assert_eq!(slug("New quest"), "new_quest");
        assert_eq!(slug("  Iron -- Age! "), "iron_age");
        assert_eq!(slug("!!!"), "quest");
        assert_eq!(snap([1.26, -0.74]), [1.5, -0.5]);
    }
}
