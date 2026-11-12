// Edit mode's forms: a window over the book with rows of fields, for a
// quest (title, description, icon, shape, rules), a task or a reward (its
// type, then the fields that type has). Text fields are hq_text's; choices
// cycle on a click; flags toggle. Save turns a form into an `Edit` for the
// server. Pure: it lays itself out as hq_book draws and click zones.
use super::hq_book::{Draw, fit_text};
use super::hq_canvas as canvas;
use super::hq_edit::{ChapterInfo, Edit, PackSettings, QuestInfo};
use super::hq_model::{
    AutoClaim, Chapter, Group, ItemMatch, Progression, Quest, Reward, RewardKind, Settings, Shape, TableEntry, Task, TaskKind,
    resource_id,
};
#[cfg(test)]
use super::hq_model::{DependencyMode, Visibility};
use super::hq_text::{Key, TextField};
use std::collections::HashMap;

/// A row's value.
#[derive(Clone, Debug, PartialEq)]
enum Value {
    Text(TextField),
    /// The options (key, label) and the one chosen.
    Choice(Vec<(String, String)>, usize),
    Flag(bool),
}

/// One row of a form: a key the form reads it back by, a label, a value,
/// and whether typing in it suggests items.
#[derive(Clone, Debug, PartialEq)]
struct Row {
    key: &'static str,
    label: &'static str,
    value: Value,
    items: bool,
}

impl Row {
    fn text(key: &'static str, label: &'static str, value: &str, max: usize) -> Self {
        Row { key, label, value: Value::Text(TextField::new(value, max)), items: false }
    }

    fn lines(key: &'static str, label: &'static str, value: &str) -> Self {
        Row { key, label, value: Value::Text(TextField::lines(value, 2000)), items: false }
    }

    fn item(key: &'static str, label: &'static str, value: &str) -> Self {
        Row { items: true, ..Row::text(key, label, value, 100) }
    }

    fn number(key: &'static str, label: &'static str, value: Option<u32>) -> Self {
        Row::text(key, label, &value.map(|n| n.to_string()).unwrap_or_default(), 10)
    }

    fn choice(key: &'static str, label: &'static str, options: &[(&str, &str)], chosen: &str) -> Self {
        let options: Vec<(String, String)> = options.iter().map(|(k, l)| ((*k).to_owned(), (*l).to_owned())).collect();
        let at = options.iter().position(|(k, _)| k == chosen).unwrap_or(0);
        Row { key, label, value: Value::Choice(options, at), items: false }
    }

    fn flag(key: &'static str, label: &'static str, on: bool) -> Self {
        Row { key, label, value: Value::Flag(on), items: false }
    }

    fn height(&self) -> f32 {
        match &self.value {
            Value::Text(field) if field.multiline => DESCRIPTION_LINES as f32 * LINE + 5.0,
            _ => ROW,
        }
    }
}

/// What a form edits.
#[derive(Clone, Debug, PartialEq)]
pub enum Target {
    /// A quest (full id).
    Quest(String),
    /// A task of a quest: the one with id `old`, or a new one.
    Task { quest: String, old: Option<String> },
    Reward { quest: String, old: Option<String> },
    /// A chapter (its id).
    Chapter(String),
    Group(String),
    /// The pack's settings (quests.json).
    Settings,
    /// Entry `index` of a reward table, or a new one.
    Entry { table: String, index: Option<usize> },
}

/// What a click on the form does.
#[derive(Clone, Debug, PartialEq)]
pub enum FormAction {
    /// Put the cursor in row `0`, on line `1`, `2` pixels into it.
    Focus(usize, usize, f32),
    Cycle(usize),
    Toggle(usize),
    /// Put this item in row `0`.
    Pick(usize, String),
    Save,
    Cancel,
    /// Remove the task or reward the form edits.
    Delete,
    /// A click on the form that does nothing.
    Stay,
}

const SHAPES: [(&str, &str); 5] =
    [("", "the chapter's"), ("square", "square"), ("circle", "circle"), ("diamond", "diamond"), ("hexagon", "hexagon")];
const VISIBILITIES: [(&str, &str); 4] = [
    ("always", "always"),
    ("until_deps_visible", "once a quest it needs shows"),
    ("until_deps_complete", "once the quests it needs are done"),
    ("secret", "secret (once done)"),
];
const MODES: [(&str, &str); 4] = [
    ("all_completed", "every quest done"),
    ("one_completed", "one quest done"),
    ("all_started", "every quest started"),
    ("one_started", "one quest started"),
];
const TASK_TYPES: [(&str, &str); 8] = [
    ("item", "have an item"),
    ("checkmark", "tick a box"),
    ("kill", "kill"),
    ("craft", "craft"),
    ("xp_levels", "reach a level"),
    ("dimension", "visit a dimension"),
    ("location", "reach a place"),
    ("advancement", "get an advancement"),
];
const REWARD_TYPES: [(&str, &str); 7] = [
    ("item", "an item"),
    ("xp", "experience points"),
    ("xp_levels", "experience levels"),
    ("command", "a command"),
    ("toast", "a message on screen"),
    ("random", "a random draw from a table"),
    ("choice", "a choice from a table"),
];
const PROGRESSIONS: [(&str, &str); 2] = [("linear", "linear: quests open in order"), ("flexible", "flexible: tasks in any order")];
const AUTO_CLAIMS: [(&str, &str); 3] = [("default", "as the pack says"), ("disabled", "never"), ("enabled", "always")];

/// The form's size and look.
const WIDTH: f32 = 360.0;
const BAR: f32 = 14.0;
const ROW: f32 = 15.0;
const LINE: f32 = 10.0;
const LABEL_W: f32 = 104.0;
const PAD: f32 = 4.0;
const DESCRIPTION_LINES: usize = 4;
const SUGGESTIONS: usize = 6;
const BACK: u32 = 0xFF1C_2230;
const EDGE: u32 = 0xFF3A_4458;
const BAR_BACK: u32 = 0xFF2A_3044;
const BOX: u32 = 0xFF0C_0E14;
const BOX_EDGE: u32 = 0xFF3A_4458;
const BOX_FOCUS: u32 = 0xFF80_90C0;
const BUTTON: u32 = 0xFF3A_5A8A;
const BUTTON_HOVER: u32 = 0xFF4A_74B0;
const DANGER: u32 = 0xFF8A_3A3A;
const DANGER_HOVER: u32 = 0xFFB0_4A4A;
const HOVER: u32 = 0xFF2A_2A3A;
const INK: u32 = 0xFF_FFFF;
const DIM_INK: u32 = 0xA0_A0B0;
const LABEL_INK: u32 = 0xC8_C8D0;
const ERROR_INK: u32 = 0xFF_6A5A;
const CURSOR: u32 = 0xFFE0_E0E8;
const ON: u32 = 0xFF5C_C83C;

#[derive(Clone, Debug, PartialEq)]
pub struct Form {
    pub target: Target,
    title: String,
    rows: Vec<Row>,
    /// The row with the cursor.
    pub focus: Option<usize>,
    pub scroll: f32,
    /// Why the last Save did not go.
    pub error: String,
    /// The pack's reward tables, for a reward's table choice.
    tables: Vec<String>,
    /// Rows of the types tried before: switching back and forth between
    /// types keeps what was typed.
    memory: Vec<Row>,
}

/// `text` trimmed, None when empty.
fn optional(text: &str) -> Option<String> {
    Some(text.trim().to_owned()).filter(|t| !t.is_empty())
}

/// How a plain enum value is written (`square`, `one_completed`...).
fn enum_key(value: &impl serde::Serialize) -> String {
    serde_json::to_value(value).ok().and_then(|v| v.as_str().map(str::to_owned)).unwrap_or_default()
}

/// `[x, y, z]` as `x y z`.
fn place_text(at: [i32; 3]) -> String {
    format!("{} {} {}", at[0], at[1], at[2])
}

impl Form {
    fn new(target: Target, title: &str, rows: Vec<Row>) -> Self {
        let focus = rows.iter().position(|row| matches!(row.value, Value::Text(_)));
        Self { target, title: title.to_owned(), rows, focus, scroll: 0.0, error: String::new(), tables: Vec::new(), memory: Vec::new() }
    }

    /// The form for quest `id`.
    pub fn quest(id: &str, quest: &Quest) -> Self {
        let info = QuestInfo::of(quest);
        let rows = vec![
            Row::text("title", "Title", &info.title, 100),
            Row::text("subtitle", "Subtitle", info.subtitle.as_deref().unwrap_or(""), 100),
            Row::lines("description", "Description", &info.description.join("\n")),
            Row::item("icon", "Icon (item id)", info.icon.as_deref().unwrap_or("")),
            Row::choice("shape", "Shape", &SHAPES, &info.shape.map(|s| enum_key(&s)).unwrap_or_default()),
            Row::text("size", "Size (1 = normal)", &info.size.map(|s| s.to_string()).unwrap_or_default(), 10),
            Row::choice("visibility", "Shown", &VISIBILITIES, &enum_key(&info.visibility)),
            Row::choice("dependency_mode", "Needs", &MODES, &enum_key(&info.dependency_mode)),
            Row::number("min_required", "How many needed", info.min_required),
            Row::flag("optional", "Optional", info.optional),
            Row::flag("sequential_tasks", "Tasks in order", info.sequential_tasks),
            Row::flag("repeatable", "Repeatable", info.repeatable),
            Row::number("repeat_cooldown", "Cooldown (seconds)", Some(info.repeat_cooldown).filter(|&c| c > 0)),
        ];
        Self::new(Target::Quest(id.to_owned()), "Edit quest", rows)
    }

    /// The form for a task of quest `quest`: `task`, or a new one.
    pub fn task(quest: &str, task: Option<&Task>) -> Self {
        let kind = task.map_or(TaskKind::Item { item: ItemMatch::Id("minecraft:oak_log".into()), count: 1, consume: false }, |t| t.kind.clone());
        let title = if task.is_some() { "Edit task" } else { "New task" };
        let target = Target::Task { quest: quest.to_owned(), old: task.map(|t| t.id.clone()) };
        let mut form = Self::new(target, title, Vec::new());
        form.rows = task_rows(&kind, task.and_then(|t| t.title.as_deref()));
        form.focus = form.rows.iter().position(|row| matches!(row.value, Value::Text(_)));
        form
    }

    /// The form for a reward of quest `quest`: `reward`, or a new one.
    /// `tables` are the pack's reward tables.
    pub fn reward(quest: &str, reward: Option<&Reward>, tables: Vec<String>) -> Self {
        let kind = reward.map_or(RewardKind::Item { item: "minecraft:bread".into(), count: 1, max_count: None }, |r| r.kind.clone());
        let title = if reward.is_some() { "Edit reward" } else { "New reward" };
        let target = Target::Reward { quest: quest.to_owned(), old: reward.map(|r| r.id.clone()) };
        let mut form = Self::new(target, title, Vec::new());
        form.tables = tables;
        let (auto, team, name) = reward.map_or((AutoClaim::Default, false, None), |r| (r.auto_claim, r.team, r.title.as_deref()));
        form.rows = reward_rows(&kind, name, auto, team, &form.tables);
        form.focus = form.rows.iter().position(|row| matches!(row.value, Value::Text(_)));
        form
    }

    /// The form for chapter `chapter`; `groups` are the pack's.
    pub fn chapter(chapter: &Chapter, groups: &[Group]) -> Self {
        let info = ChapterInfo::of(chapter);
        let mut group_options = vec![("", "none")];
        group_options.extend(groups.iter().map(|g| (g.id.as_str(), if g.title.is_empty() { g.id.as_str() } else { g.title.as_str() })));
        let mut progressions = vec![("", "the pack's")];
        progressions.extend(PROGRESSIONS);
        let mut shapes = SHAPES;
        shapes[0].1 = "the pack's";
        let rows = vec![
            Row::text("title", "Title", &info.title, 100),
            Row::text("subtitle", "Subtitle", info.subtitle.as_deref().unwrap_or(""), 100),
            Row::item("icon", "Icon (item id)", info.icon.as_deref().unwrap_or("")),
            Row::choice("group", "Group", &group_options, info.group.as_deref().unwrap_or("")),
            Row::flag("hidden", "Hidden from the list", info.hidden),
            Row::choice("progression", "Progression", &progressions, &info.progression.map(|p| enum_key(&p)).unwrap_or_default()),
            Row::choice("default_shape", "Quest shape", &shapes, &info.default_shape.map(|s| enum_key(&s)).unwrap_or_default()),
            Row::text("default_size", "Quest size", &info.default_size.map(|s| s.to_string()).unwrap_or_default(), 10),
        ];
        Self::new(Target::Chapter(chapter.id.clone()), "Edit chapter", rows)
    }

    /// The form to rename group `group`.
    pub fn group(group: &Group) -> Self {
        Self::new(Target::Group(group.id.clone()), "Rename group", vec![Row::text("title", "Title", &group.title, 100)])
    }

    /// The form for the pack's settings.
    pub fn settings(settings: &Settings) -> Self {
        let now = PackSettings::of(settings);
        let rows = vec![
            Row::choice("progression", "Progression", &PROGRESSIONS, &enum_key(&now.progression)),
            Row::choice("default_shape", "Quest shape", &SHAPES[1..], &enum_key(&now.default_shape)),
            Row::text("default_size", "Quest size", &now.default_size.to_string(), 10),
            Row::flag("auto_claim_rewards", "Rewards claimed by themselves", now.auto_claim_rewards),
            Row::number("detection_delay", "Look at inventories every (ticks)", Some(now.detection_delay)),
        ];
        Self::new(Target::Settings, "Pack settings", rows)
    }

    /// The form for entry `index` of reward table `table`: `entry`, or a
    /// new one.
    pub fn entry(table: &str, index: Option<usize>, entry: Option<&TableEntry>) -> Self {
        let kind = entry.map_or(RewardKind::Item { item: "minecraft:bread".into(), count: 1, max_count: None }, |e| e.kind.clone());
        let title = if entry.is_some() { "Edit table entry" } else { "New table entry" };
        let mut form = Self::new(Target::Entry { table: table.to_owned(), index }, title, Vec::new());
        form.rows = entry_rows(&kind, entry.and_then(|e| e.title.as_deref()), entry.map_or(1, |e| e.weight));
        form.focus = form.rows.iter().position(|row| matches!(row.value, Value::Text(_)));
        form
    }

    /// Whether the form edits something that is there (it can be deleted).
    fn deletable(&self) -> bool {
        matches!(
            &self.target,
            Target::Task { old: Some(_), .. } | Target::Reward { old: Some(_), .. } | Target::Entry { index: Some(_), .. }
        )
    }

    fn find(&self, key: &str) -> Option<&Value> {
        self.rows.iter().find(|row| row.key == key).map(|row| &row.value)
    }

    fn text(&self, key: &str) -> String {
        match self.find(key) {
            Some(Value::Text(field)) => field.value.trim().to_owned(),
            _ => String::new(),
        }
    }

    fn chosen(&self, key: &str) -> String {
        match self.find(key) {
            Some(Value::Choice(options, at)) => options.get(*at).map(|(k, _)| k.clone()).unwrap_or_default(),
            _ => String::new(),
        }
    }

    fn flag(&self, key: &str) -> bool {
        matches!(self.find(key), Some(Value::Flag(true)))
    }

    /// Row `key` as a whole number: None when empty.
    fn number(&self, key: &str, label: &str) -> Result<Option<u32>, String> {
        optional(&self.text(key)).map(|t| t.parse::<u32>().map_err(|_| format!("{label}: a whole number."))).transpose()
    }

    /// Row `key` as a number of at least 1, 1 when empty.
    fn count(&self, key: &str, label: &str) -> Result<u32, String> {
        match self.number(key, label)? {
            Some(0) => Err(format!("{label}: at least 1.")),
            n => Ok(n.unwrap_or(1)),
        }
    }

    /// Row `key` as an id (`minecraft:` when no namespace).
    fn id(&self, key: &str, label: &str) -> Result<String, String> {
        let text = self.text(key);
        resource_id(&text).ok_or_else(|| format!("{label}: `{text}` is not an id."))
    }

    /// Row `key` as three whole numbers.
    fn place(&self, key: &str, label: &str) -> Result<[i32; 3], String> {
        let parts: Vec<i32> = self.text(key).split_whitespace().filter_map(|p| p.parse().ok()).collect();
        <[i32; 3]>::try_from(parts).map_err(|_| format!("{label}: three whole numbers, x y z."))
    }

    fn enum_of<T: serde::de::DeserializeOwned>(&self, key: &str) -> Option<T> {
        serde_json::from_value(serde_json::Value::String(self.chosen(key))).ok()
    }

    /// The quest form's fields as the quest's new info, or what is wrong.
    pub fn quest_info(&self) -> Result<QuestInfo, String> {
        let size = optional(&self.text("size"))
            .map(|t| t.parse::<f32>().ok().filter(|s| (0.25..=4.0).contains(s)).ok_or("Size: a number from 0.25 to 4, or nothing."))
            .transpose()?;
        let icon = optional(&self.text("icon")).map(|_| self.id("icon", "Icon")).transpose()?;
        let description = match self.find("description") {
            Some(Value::Text(field)) => {
                let mut lines: Vec<String> = field.value.split('\n').map(|line| line.trim_end().to_owned()).collect();
                while lines.last().is_some_and(String::is_empty) {
                    lines.pop();
                }
                lines
            }
            _ => Vec::new(),
        };
        Ok(QuestInfo {
            title: self.text("title"),
            subtitle: optional(&self.text("subtitle")),
            description,
            icon,
            shape: self.enum_of::<Shape>("shape"),
            size,
            visibility: self.enum_of("visibility").unwrap_or_default(),
            dependency_mode: self.enum_of("dependency_mode").unwrap_or_default(),
            min_required: self.number("min_required", "How many needed")?.filter(|&n| n > 0),
            optional: self.flag("optional"),
            sequential_tasks: self.flag("sequential_tasks"),
            repeatable: self.flag("repeatable"),
            repeat_cooldown: self.number("repeat_cooldown", "Cooldown")?.unwrap_or(0),
        })
    }

    /// The task form's fields as a task (its id is the server's to give
    /// a new one), or what is wrong.
    pub fn task_value(&self) -> Result<Task, String> {
        let item = || -> Result<ItemMatch, String> {
            let text = self.text("item");
            ItemMatch::try_from(text.clone()).map_err(|_| format!("Item: `{text}` is not an item id or a #tag."))
        };
        let kind = match self.chosen("type").as_str() {
            "checkmark" => TaskKind::Checkmark,
            "kill" => TaskKind::Kill { entity: self.id("entity", "Entity")?, count: self.count("count", "How many")? },
            "craft" => TaskKind::Craft { item: item()?, count: self.count("count", "How many")? },
            "xp_levels" => TaskKind::XpLevels { levels: self.count("levels", "Level")? },
            "dimension" => TaskKind::Dimension { dimension: self.id("dimension", "Dimension")? },
            "location" => TaskKind::Location {
                dimension: optional(&self.text("dimension")).map(|_| self.id("dimension", "Dimension")).transpose()?,
                from: self.place("from", "From")?,
                to: self.place("to", "To")?,
            },
            "advancement" => TaskKind::Advancement { advancement: self.id("advancement", "Advancement")? },
            _ => TaskKind::Item { item: item()?, count: self.count("count", "How many")?, consume: self.flag("consume") },
        };
        let id = match &self.target {
            Target::Task { old: Some(old), .. } => old.clone(),
            _ => String::new(),
        };
        Ok(Task { id, title: optional(&self.text("title")), kind })
    }

    /// The reward form's fields as a reward, or what is wrong.
    pub fn reward_value(&self) -> Result<Reward, String> {
        let kind = self.reward_kind()?;
        let id = match &self.target {
            Target::Reward { old: Some(old), .. } => old.clone(),
            _ => String::new(),
        };
        let auto_claim = self.enum_of("auto_claim").unwrap_or_default();
        Ok(Reward { id, title: optional(&self.text("title")), auto_claim, team: self.flag("team"), kind })
    }

    /// The entry form's fields as a table entry.
    pub fn entry_value(&self) -> Result<TableEntry, String> {
        let weight = self.number("weight", "Weight")?.unwrap_or(1);
        Ok(TableEntry { weight, title: optional(&self.text("title")), kind: self.reward_kind()? })
    }

    /// The chapter form's fields.
    pub fn chapter_info(&self) -> Result<ChapterInfo, String> {
        let size = optional(&self.text("default_size"))
            .map(|t| t.parse::<f32>().ok().filter(|s| (0.25..=4.0).contains(s)).ok_or("Quest size: a number from 0.25 to 4, or nothing."))
            .transpose()?;
        Ok(ChapterInfo {
            title: self.text("title"),
            subtitle: optional(&self.text("subtitle")),
            icon: optional(&self.text("icon")).map(|_| self.id("icon", "Icon")).transpose()?,
            group: optional(&self.chosen("group")),
            hidden: self.flag("hidden"),
            progression: self.enum_of::<Progression>("progression"),
            default_shape: self.enum_of::<Shape>("default_shape"),
            default_size: size,
        })
    }

    /// The settings form's fields.
    pub fn pack_settings(&self) -> Result<PackSettings, String> {
        let size = self.text("default_size").parse::<f32>().ok().filter(|s| (0.25..=4.0).contains(s)).ok_or("Quest size: a number from 0.25 to 4.")?;
        Ok(PackSettings {
            progression: self.enum_of("progression").unwrap_or_default(),
            default_shape: self.enum_of("default_shape").unwrap_or_default(),
            default_size: size,
            auto_claim_rewards: self.flag("auto_claim_rewards"),
            detection_delay: self.count("detection_delay", "Look at inventories every")?,
        })
    }

    /// What the reward or entry form gives.
    fn reward_kind(&self) -> Result<RewardKind, String> {
        let table = || -> Result<String, String> {
            Some(self.chosen("table")).filter(|t| !t.is_empty()).ok_or_else(|| "This pack has no reward table yet.".to_owned())
        };
        let kind = match self.chosen("type").as_str() {
            "xp" => RewardKind::Xp { points: self.count("points", "Points")? },
            "xp_levels" => RewardKind::XpLevels { levels: self.count("levels", "Levels")? },
            "command" => {
                let command = self.text("command");
                if command.is_empty() {
                    return Err("Command: write the command to run.".to_owned());
                }
                RewardKind::Command { command: command.trim_start_matches('/').to_owned() }
            }
            "toast" => RewardKind::Toast { text: self.text("text") },
            "random" => RewardKind::Random { table: table()? },
            "choice" => RewardKind::Choice { table: table()? },
            _ => {
                let count = self.count("count", "How many")?;
                let max_count = self.number("max_count", "At most")?;
                if max_count.is_some_and(|max| max < count) {
                    return Err("At most: not below How many.".to_owned());
                }
                RewardKind::Item { item: self.id("item", "Item")?, count, max_count }
            }
        };
        Ok(kind)
    }

    /// The edit Save asks for, or why it cannot go.
    pub fn save(&self) -> Result<Edit, String> {
        Ok(match &self.target {
            Target::Quest(quest) => Edit::SetQuestInfo { quest: quest.clone(), info: self.quest_info()? },
            Target::Task { quest, old } => Edit::SaveTask { quest: quest.clone(), old: old.clone(), task: self.task_value()? },
            Target::Reward { quest, old } => Edit::SaveReward { quest: quest.clone(), old: old.clone(), reward: self.reward_value()? },
            Target::Chapter(chapter) => Edit::SetChapter { chapter: chapter.clone(), info: self.chapter_info()? },
            Target::Group(group) => Edit::SetGroup { group: group.clone(), title: self.text("title") },
            Target::Settings => Edit::SetSettings { settings: self.pack_settings()? },
            Target::Entry { table, index } => Edit::SaveEntry { table: table.clone(), index: *index, entry: self.entry_value()? },
        })
    }

    /// The edit Delete asks for.
    pub fn delete(&self) -> Option<Edit> {
        match &self.target {
            Target::Task { quest, old: Some(task) } => Some(Edit::RemoveTask { quest: quest.clone(), task: task.clone() }),
            Target::Reward { quest, old: Some(reward) } => Some(Edit::RemoveReward { quest: quest.clone(), reward: reward.clone() }),
            Target::Entry { table, index: Some(index) } => Some(Edit::RemoveEntry { table: table.clone(), index: *index }),
            _ => None,
        }
    }

    /// A task or reward form whose type changed: its rows again, keeping
    /// what rows of the same key held.
    fn retype(&mut self) {
        let old = std::mem::take(&mut self.rows);
        for row in &old {
            match self.memory.iter_mut().find(|kept| kept.key == row.key) {
                Some(kept) => *kept = row.clone(),
                None => self.memory.push(row.clone()),
            }
        }
        let memory = &self.memory;
        let kept = |key: &str| memory.iter().find(|row| row.key == key).map(|row| row.value.clone());
        let fresh = match &self.target {
            Target::Task { .. } => {
                let kind = blank_task(&match kept("type") {
                    Some(Value::Choice(options, at)) => options[at].0.clone(),
                    _ => String::new(),
                });
                task_rows(&kind, None)
            }
            Target::Reward { .. } => {
                let kind = blank_reward(
                    &match kept("type") {
                        Some(Value::Choice(options, at)) => options[at].0.clone(),
                        _ => String::new(),
                    },
                    &self.tables,
                );
                reward_rows(&kind, None, AutoClaim::Default, false, &self.tables)
            }
            Target::Entry { .. } => {
                let kind = blank_reward(
                    &match kept("type") {
                        Some(Value::Choice(options, at)) => options[at].0.clone(),
                        _ => String::new(),
                    },
                    &[],
                );
                entry_rows(&kind, None, 1)
            }
            Target::Quest(_) | Target::Chapter(_) | Target::Group(_) | Target::Settings => old.clone(),
        };
        let rows = fresh
            .into_iter()
            .map(|mut row| {
                if let Some(value) = kept(row.key)
                    && std::mem::discriminant(&value) == std::mem::discriminant(&row.value)
                {
                    row.value = value;
                }
                row
            })
            .collect();
        self.rows = rows;
        self.focus = None;
    }

    /// A click on the form. Save, Cancel and Delete are the book's.
    pub fn click(&mut self, action: &FormAction, measure: &dyn Fn(&str) -> f32) {
        match action {
            FormAction::Focus(r, line, x) => {
                if let Some(Value::Text(field)) = self.rows.get_mut(*r).map(|row| &mut row.value) {
                    let scroll = field.shown(*line, f32::MAX, measure).1;
                    field.click(*line, *x, scroll, measure);
                    self.focus = Some(*r);
                }
            }
            FormAction::Cycle(r) => {
                if let Some(row) = self.rows.get_mut(*r)
                    && let Value::Choice(options, at) = &mut row.value
                {
                    *at = (*at + 1) % options.len().max(1);
                    if row.key == "type" {
                        self.retype();
                    }
                }
            }
            FormAction::Toggle(r) => {
                if let Some(Value::Flag(on)) = self.rows.get_mut(*r).map(|row| &mut row.value) {
                    *on = !*on;
                }
            }
            FormAction::Pick(r, item) => {
                if let Some(Value::Text(field)) = self.rows.get_mut(*r).map(|row| &mut row.value) {
                    *field = TextField::new(item, field.max_length);
                    self.focus = None;
                }
            }
            FormAction::Save | FormAction::Cancel | FormAction::Delete | FormAction::Stay => {}
        }
    }

    /// A character typed: into the row with the cursor.
    pub fn typed(&mut self, c: char) {
        if let Some(Value::Text(field)) = self.focus.and_then(|r| self.rows.get_mut(r)).map(|row| &mut row.value) {
            field.type_char(c);
        }
    }

    /// A key pressed while the form is up. Escape leaves the field first,
    /// then the form (`false`: close it). Tab goes to the next text field.
    pub fn key(&mut self, name: &str) -> bool {
        let texts: Vec<usize> = (0..self.rows.len()).filter(|&r| matches!(self.rows[r].value, Value::Text(_))).collect();
        match name {
            "key.keyboard.escape" => return self.focus.take().is_some(),
            "key.keyboard.tab" if !texts.is_empty() => {
                let next = self.focus.and_then(|f| texts.iter().position(|&r| r == f)).map_or(0, |i| (i + 1) % texts.len());
                self.focus = Some(texts[next]);
            }
            _ => {
                if let (Some(r), Some(key)) = (self.focus, Key::named(name))
                    && let Some(Value::Text(field)) = self.rows.get_mut(r).map(|row| &mut row.value)
                    && !field.key(key)
                {
                    self.focus = None;
                }
            }
        }
        true
    }

    /// The item suggestions for the row with the cursor: items whose id or
    /// name holds what is typed, those starting with it first.
    fn suggestions(&self, names: &HashMap<String, String>) -> Vec<(String, String)> {
        let Some(Row { value: Value::Text(field), items: true, .. }) = self.focus.and_then(|r| self.rows.get(r)) else {
            return Vec::new();
        };
        let typed = field.value.trim().to_lowercase();
        if typed.len() < 2 || typed.starts_with('#') {
            return Vec::new();
        }
        let mut found: Vec<(bool, &String, &String)> = names
            .iter()
            .filter(|(id, name)| id.contains(&typed) || name.to_lowercase().contains(&typed))
            .map(|(id, name)| {
                let path = id.split_once(':').map_or(id.as_str(), |(_, p)| p);
                (!(path.starts_with(&typed) || name.to_lowercase().starts_with(&typed)), id, name)
            })
            .collect();
        found.sort_by(|a, b| (a.0, a.1.len(), a.1).cmp(&(b.0, b.1.len(), b.1)));
        found.into_iter().take(SUGGESTIONS).map(|(_, id, name)| (id.clone(), name.clone())).collect()
    }

    fn content_h(&self) -> f32 {
        self.rows.iter().map(Row::height).sum::<f32>() + 2.0 * PAD
    }

    /// Where the form goes over `area` (the canvas).
    pub fn rect(&self, area: [f32; 4]) -> [f32; 4] {
        let w = WIDTH.min(area[2] - 8.0).max(0.0);
        let h = (self.content_h() + 2.0 * BAR).min(area[3] - 8.0).max(0.0);
        let [cx, cy] = canvas::centre(area);
        [cx - w / 2.0, cy - h / 2.0, w, h]
    }

    /// How far the rows can scroll in `area`.
    pub fn max_scroll(&self, area: [f32; 4]) -> f32 {
        (self.content_h() - (self.rect(area)[3] - 2.0 * BAR)).max(0.0)
    }

    /// The form over `area`: what to draw and where clicks land (later
    /// zones on top). `clock` blinks the cursor.
    pub fn layout(
        &self,
        area: [f32; 4],
        mouse: [f32; 2],
        clock: u32,
        names: &HashMap<String, String>,
        measure: &dyn Fn(&str) -> f32,
    ) -> (Vec<Draw>, Vec<([f32; 4], FormAction)>) {
        let mut draws = Vec::new();
        let mut zones = Vec::new();
        let fill = |draws: &mut Vec<Draw>, rect: [f32; 4], colour: u32| draws.push(Draw::Fill { rect, colour });
        let text = |draws: &mut Vec<Draw>, text: String, at: [f32; 2], colour: u32| {
            if !text.is_empty() {
                draws.push(Draw::Text { text, at, colour, shadow: false });
            }
        };
        let frame = |draws: &mut Vec<Draw>, [x, y, w, h]: [f32; 4], colour: u32| {
            for edge in [[x, y, w, 1.0], [x, y + h - 1.0, w, 1.0], [x, y, 1.0, h], [x + w - 1.0, y, 1.0, h]] {
                draws.push(Draw::Fill { rect: edge, colour });
            }
        };
        let rect @ [x, y, w, h] = self.rect(area);
        fill(&mut draws, rect, BACK);
        frame(&mut draws, rect, EDGE);
        zones.push((rect, FormAction::Stay));
        fill(&mut draws, [x + 1.0, y + 1.0, w - 2.0, BAR - 1.0], BAR_BACK);
        text(&mut draws, fit_text(&self.title, w - 2.0 * PAD, measure), [x + PAD, y + 3.0], INK);

        let body = [x, y + BAR, w, h - 2.0 * BAR];
        let field_x = x + PAD + LABEL_W;
        let field_w = (x + w - PAD - field_x).max(0.0);
        let mut row_y = body[1] + PAD - self.scroll;
        let mut item_box = None;
        if self.rows.is_empty() {
            text(&mut draws, "Nothing to set.".to_owned(), [x + PAD, row_y + 3.0], DIM_INK);
        }
        for (r, row) in self.rows.iter().enumerate() {
            let row_h = row.height();
            if row_y >= body[1] && row_y + row_h <= body[1] + body[3] {
                text(&mut draws, fit_text(row.label, LABEL_W - PAD, measure), [x + PAD, row_y + 3.0], LABEL_INK);
                let control = [field_x, row_y + 1.0, field_w, row_h - 2.0];
                match &row.value {
                    Value::Text(value) => {
                        let focused = self.focus == Some(r);
                        // An item field leaves room for the item itself.
                        let boxed = if row.items { [control[0], control[1], control[2] - 16.0, control[3]] } else { control };
                        fill(&mut draws, boxed, BOX);
                        frame(&mut draws, boxed, if focused { BOX_FOCUS } else { BOX_EDGE });
                        let inner = boxed[2] - 6.0;
                        let lines = if value.multiline { DESCRIPTION_LINES } else { 1 };
                        if value.multiline {
                            // Under the lines (they are on top): the end of the text.
                            zones.push((boxed, FormAction::Focus(r, value.line_count() - 1, f32::MAX)));
                        }
                        // A long text scrolls to keep the cursor's line in sight.
                        let first = if focused { value.cursor_line().saturating_sub(lines - 1) } else { 0 };
                        for k in 0..lines {
                            let line = first + k;
                            if line >= value.line_count() {
                                break;
                            }
                            let (part, _, cursor_x) = value.shown(line, inner, measure);
                            let at = [boxed[0] + 3.0, boxed[1] + 2.5 + k as f32 * LINE];
                            text(&mut draws, part, at, INK);
                            if let Some(cx) = cursor_x.filter(|_| focused && (clock / 10) % 2 == 0) {
                                fill(&mut draws, [at[0] + cx, at[1] - 1.0, 1.0, 9.0], CURSOR);
                            }
                            let zone = [boxed[0], boxed[1] + k as f32 * LINE, boxed[2], LINE + 1.0];
                            zones.push((zone, FormAction::Focus(r, line, mouse[0] - at[0])));
                        }
                        if row.items {
                            if focused {
                                item_box = Some((r, boxed));
                            }
                            let typed = value.value.trim();
                            let shown = typed.strip_prefix('#').map_or_else(|| resource_id(typed), |tag| resource_id(tag).map(|t| format!("#{t}")));
                            if let Some(item) = shown {
                                draws.push(Draw::Item { item, at: [boxed[0] + boxed[2] + 2.0, row_y - 0.5], size: 14.0 });
                            }
                        }
                    }
                    Value::Choice(options, at) => {
                        let label = options.get(*at).map_or("(none)", |(_, label)| label.as_str());
                        let hovered = canvas::inside(control, mouse);
                        fill(&mut draws, control, if hovered { BUTTON_HOVER } else { BUTTON });
                        text(&mut draws, fit_text(&format!("< {label} >"), control[2] - 6.0, measure), [control[0] + 3.0, control[1] + 3.0], INK);
                        zones.push((control, FormAction::Cycle(r)));
                    }
                    Value::Flag(on) => {
                        let square = [control[0], control[1] + 1.0, 10.0, 10.0];
                        fill(&mut draws, square, BOX);
                        frame(&mut draws, square, if canvas::inside(square, mouse) { BOX_FOCUS } else { BOX_EDGE });
                        if *on {
                            fill(&mut draws, [square[0] + 2.0, square[1] + 2.0, 6.0, 6.0], ON);
                        }
                        text(&mut draws, (if *on { "yes" } else { "no" }).to_owned(), [square[0] + 14.0, control[1] + 2.0], if *on { INK } else { DIM_INK });
                        zones.push(([control[0], control[1], 40.0, control[3]], FormAction::Toggle(r)));
                    }
                }
            }
            row_y += row_h;
        }

        // Bottom bar: the hint or the last error, then Delete, Cancel, Save.
        let bar_y = y + h - BAR;
        fill(&mut draws, [x + 1.0, bar_y, w - 2.0, BAR - 1.0], BAR_BACK);
        let mut right = x + w - PAD;
        let mut buttons = vec![("Save", FormAction::Save, BUTTON, BUTTON_HOVER), ("Cancel", FormAction::Cancel, BUTTON, BUTTON_HOVER)];
        if self.deletable() {
            buttons.push(("Delete", FormAction::Delete, DANGER, DANGER_HOVER));
        }
        for (label, action, colour, hover) in buttons {
            let bw = measure(label) + 8.0;
            let button = [right - bw, bar_y + 2.0, bw, BAR - 4.0];
            fill(&mut draws, button, if canvas::inside(button, mouse) { hover } else { colour });
            text(&mut draws, label.to_owned(), [button[0] + 4.0, button[1] + 1.0], INK);
            zones.push((button, action));
            right = button[0] - PAD;
        }
        let hint = if self.error.is_empty() { "Tab: next field | Esc: back" } else { &self.error };
        let ink = if self.error.is_empty() { DIM_INK } else { ERROR_INK };
        text(&mut draws, fit_text(hint, right - x - 2.0 * PAD, measure), [x + PAD, bar_y + 3.0], ink);

        // Item suggestions drop down over the rows below the field.
        if let Some((r, field)) = item_box {
            let found = self.suggestions(names);
            let list_y = field[1] + field[3];
            for (k, (id, name)) in found.iter().enumerate() {
                let line = [field[0], list_y + k as f32 * 13.0, field[2], 13.0];
                fill(&mut draws, line, if canvas::inside(line, mouse) { HOVER } else { BOX });
                draws.push(Draw::Item { item: id.clone(), at: [line[0] + 1.0, line[1] + 0.5], size: 12.0 });
                text(&mut draws, fit_text(&format!("{name}  {id}"), line[2] - 18.0, measure), [line[0] + 16.0, line[1] + 2.5], INK);
                zones.push((line, FormAction::Pick(r, id.clone())));
            }
            if !found.is_empty() {
                frame(&mut draws, [field[0], list_y, field[2], found.len() as f32 * 13.0], BOX_EDGE);
            }
        }
        (draws, zones)
    }
}

/// A task of type `key` with its defaults.
fn blank_task(key: &str) -> TaskKind {
    match key {
        "checkmark" => TaskKind::Checkmark,
        "kill" => TaskKind::Kill { entity: "minecraft:zombie".into(), count: 1 },
        "craft" => TaskKind::Craft { item: ItemMatch::Id("minecraft:crafting_table".into()), count: 1 },
        "xp_levels" => TaskKind::XpLevels { levels: 5 },
        "dimension" => TaskKind::Dimension { dimension: "minecraft:the_nether".into() },
        "location" => TaskKind::Location { dimension: None, from: [0, 0, 0], to: [10, 255, 10] },
        "advancement" => TaskKind::Advancement { advancement: "minecraft:story/mine_stone".into() },
        _ => TaskKind::Item { item: ItemMatch::Id("minecraft:oak_log".into()), count: 1, consume: false },
    }
}

/// A reward of type `key` with its defaults.
fn blank_reward(key: &str, tables: &[String]) -> RewardKind {
    let table = tables.first().cloned().unwrap_or_default();
    match key {
        "xp" => RewardKind::Xp { points: 10 },
        "xp_levels" => RewardKind::XpLevels { levels: 1 },
        "command" => RewardKind::Command { command: "say {player} did it!".into() },
        "toast" => RewardKind::Toast { text: String::new() },
        "random" => RewardKind::Random { table },
        "choice" => RewardKind::Choice { table },
        _ => RewardKind::Item { item: "minecraft:bread".into(), count: 1, max_count: None },
    }
}

/// A task form's rows for `kind`.
fn task_rows(kind: &TaskKind, title: Option<&str>) -> Vec<Row> {
    let key = serde_json::to_value(kind).ok().and_then(|v| v.get("type").and_then(|t| t.as_str()).map(str::to_owned)).unwrap_or_default();
    let mut rows = vec![Row::choice("type", "Type", &TASK_TYPES, &key)];
    match kind {
        TaskKind::Item { item, count, consume } => {
            rows.push(Row::item("item", "Item (or #tag)", &String::from(item.clone())));
            rows.push(Row::number("count", "How many", Some(*count)));
            rows.push(Row::flag("consume", "Handed in", *consume));
        }
        TaskKind::Checkmark => {}
        TaskKind::Kill { entity, count } => {
            rows.push(Row::text("entity", "Entity", entity, 100));
            rows.push(Row::number("count", "How many", Some(*count)));
        }
        TaskKind::Craft { item, count } => {
            rows.push(Row::item("item", "Item (or #tag)", &String::from(item.clone())));
            rows.push(Row::number("count", "How many", Some(*count)));
        }
        TaskKind::XpLevels { levels } => rows.push(Row::number("levels", "Level", Some(*levels))),
        TaskKind::Dimension { dimension } => rows.push(Row::text("dimension", "Dimension", dimension, 100)),
        TaskKind::Location { dimension, from, to } => {
            rows.push(Row::text("dimension", "Dimension (or any)", dimension.as_deref().unwrap_or(""), 100));
            rows.push(Row::text("from", "From (x y z)", &place_text(*from), 40));
            rows.push(Row::text("to", "To (x y z)", &place_text(*to), 40));
        }
        TaskKind::Advancement { advancement } => rows.push(Row::text("advancement", "Advancement", advancement, 100)),
    }
    rows.push(Row::text("title", "Title (optional)", title.unwrap_or(""), 100));
    rows
}

/// A reward form's rows for `kind`.
fn reward_rows(kind: &RewardKind, title: Option<&str>, auto: AutoClaim, team: bool, tables: &[String]) -> Vec<Row> {
    let mut rows = kind_rows(kind, &REWARD_TYPES, tables);
    rows.push(Row::text("title", "Title (optional)", title.unwrap_or(""), 100));
    let auto_key = serde_json::to_value(auto).ok().and_then(|v| v.as_str().map(str::to_owned)).unwrap_or_default();
    rows.push(Row::choice("auto_claim", "Claimed by itself", &AUTO_CLAIMS, &auto_key));
    rows.push(Row::flag("team", "For the whole party", team));
    rows
}

/// A table entry form's rows for `kind`: no table in a table.
fn entry_rows(kind: &RewardKind, title: Option<&str>, weight: u32) -> Vec<Row> {
    let mut rows = kind_rows(kind, &REWARD_TYPES[..5], &[]);
    rows.push(Row::text("title", "Title (optional)", title.unwrap_or(""), 100));
    rows.push(Row::number("weight", "Weight (random draws)", Some(weight)));
    rows
}

/// The type row and the rows of what a reward of `kind` gives.
fn kind_rows(kind: &RewardKind, types: &[(&str, &str)], tables: &[String]) -> Vec<Row> {
    let key = serde_json::to_value(kind).ok().and_then(|v| v.get("type").and_then(|t| t.as_str()).map(str::to_owned)).unwrap_or_default();
    let mut rows = vec![Row::choice("type", "Type", types, &key)];
    match kind {
        RewardKind::Item { item, count, max_count } => {
            rows.push(Row::item("item", "Item", item));
            rows.push(Row::number("count", "How many", Some(*count)));
            rows.push(Row::number("max_count", "At most (random)", *max_count));
        }
        RewardKind::Xp { points } => rows.push(Row::number("points", "Points", Some(*points))),
        RewardKind::XpLevels { levels } => rows.push(Row::number("levels", "Levels", Some(*levels))),
        RewardKind::Command { command } => rows.push(Row::text("command", "Command", command, 300)),
        RewardKind::Toast { text } => rows.push(Row::text("text", "Message", text, 200)),
        RewardKind::Random { table } | RewardKind::Choice { table } => {
            let options: Vec<(&str, &str)> = tables.iter().map(|t| (t.as_str(), t.as_str())).collect();
            rows.push(Row::choice("table", "Table", &options, table));
        }
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    fn measure(text: &str) -> f32 {
        text.chars().count() as f32 * 6.0
    }

    fn quest() -> Quest {
        Quest {
            id: "q".into(),
            title: "Old".into(),
            description: vec!["Line one".into(), "Line two".into()],
            dependency_mode: DependencyMode::OneCompleted,
            visibility: Visibility::Secret,
            ..Quest::default()
        }
    }

    fn row(form: &Form, key: &str) -> usize {
        form.rows.iter().position(|r| r.key == key).unwrap_or_else(|| panic!("no row {key}"))
    }

    fn type_in(form: &mut Form, key: &str, text: &str) {
        let r = row(form, key);
        form.click(&FormAction::Focus(r, 0, f32::MAX), &measure);
        for _ in 0..100 {
            form.key("key.keyboard.backspace");
        }
        text.chars().for_each(|c| form.typed(c));
    }

    #[test]
    fn an_untouched_form_gives_the_quest_back() {
        let q = quest();
        assert_eq!(Form::quest("a/q", &q).quest_info(), Ok(QuestInfo::of(&q)));
    }

    #[test]
    fn the_quest_form_reads_every_field() {
        let mut form = Form::quest("a/q", &quest());
        assert_eq!(form.focus, Some(0));
        type_in(&mut form, "title", "New");
        form.key("key.keyboard.tab");
        "Sub".chars().for_each(|c| form.typed(c));
        form.key("key.keyboard.tab");
        form.key("key.keyboard.enter");
        form.key("key.keyboard.enter");
        type_in(&mut form, "icon", "stone");
        form.click(&FormAction::Cycle(row(&form, "shape")), &measure);
        form.click(&FormAction::Cycle(row(&form, "dependency_mode")), &measure);
        form.click(&FormAction::Toggle(row(&form, "repeatable")), &measure);
        let info = form.quest_info().unwrap();
        assert_eq!((info.title.as_str(), info.subtitle.as_deref()), ("New", Some("Sub")));
        // Two new empty lines at the end are dropped.
        assert_eq!(info.description, vec!["Line one", "Line two"]);
        assert_eq!(info.icon.as_deref(), Some("minecraft:stone"));
        assert_eq!(info.shape, Some(Shape::Square));
        assert_eq!(info.dependency_mode, DependencyMode::AllStarted);
        assert_eq!(info.visibility, Visibility::Secret);
        assert!(info.repeatable);
        assert!(matches!(form.save(), Ok(Edit::SetQuestInfo { .. })));
    }

    #[test]
    fn wrong_values_are_named() {
        let mut form = Form::quest("a/q", &quest());
        type_in(&mut form, "size", "9");
        assert!(form.quest_info().unwrap_err().starts_with("Size"));
        type_in(&mut form, "size", "");
        type_in(&mut form, "icon", "Bad Item");
        assert!(form.quest_info().unwrap_err().starts_with("Icon"));
    }

    #[test]
    fn escape_leaves_the_field_then_the_form() {
        let mut form = Form::quest("a/q", &quest());
        assert!(form.key("key.keyboard.escape"));
        assert!(!form.key("key.keyboard.escape"));
    }

    #[test]
    fn a_new_task_changes_type_and_keeps_the_count() {
        let mut form = Form::task("a/q", None);
        assert!(!form.deletable());
        type_in(&mut form, "item", "#minecraft:logs");
        type_in(&mut form, "count", "16");
        form.click(&FormAction::Toggle(row(&form, "consume")), &measure);
        let task = form.task_value().unwrap();
        assert_eq!(task.kind, TaskKind::Item { item: ItemMatch::Tag("minecraft:logs".into()), count: 16, consume: true });
        assert_eq!(task.id, "");
        // item -> checkmark -> kill: the count comes back with the kill.
        let t = row(&form, "type");
        form.click(&FormAction::Cycle(t), &measure);
        assert_eq!(form.task_value().unwrap().kind, TaskKind::Checkmark);
        form.click(&FormAction::Cycle(t), &measure);
        assert_eq!(form.task_value().unwrap().kind, TaskKind::Kill { entity: "minecraft:zombie".into(), count: 16 });
        type_in(&mut form, "count", "0");
        assert!(form.task_value().is_err());
        assert!(matches!(form.save(), Err(_)));
    }

    #[test]
    fn a_task_form_round_trips_every_type() {
        for (key, _) in TASK_TYPES {
            let task = Task { id: "t".into(), title: Some("Named".into()), kind: blank_task(key) };
            let form = Form::task("a/q", Some(&task));
            assert!(form.deletable());
            assert_eq!(form.task_value(), Ok(task.clone()), "{key}");
            assert_eq!(form.delete(), Some(Edit::RemoveTask { quest: "a/q".into(), task: "t".into() }));
        }
    }

    #[test]
    fn a_reward_form_round_trips_every_type_and_needs_a_table() {
        let tables = vec!["loot".to_owned()];
        for (key, _) in REWARD_TYPES {
            let reward = Reward { id: "r".into(), title: None, auto_claim: AutoClaim::Disabled, team: true, kind: blank_reward(key, &tables) };
            let form = Form::reward("a/q", Some(&reward), tables.clone());
            assert_eq!(form.reward_value(), Ok(reward.clone()), "{key}");
        }
        let mut form = Form::reward("a/q", None, Vec::new());
        let t = row(&form, "type");
        for _ in 0..5 {
            form.click(&FormAction::Cycle(t), &measure);
        }
        assert_eq!(form.chosen("type"), "random");
        assert!(form.reward_value().unwrap_err().contains("no reward table"));
    }

    #[test]
    fn chapter_group_settings_and_entry_forms_round_trip() {
        let groups = vec![Group { id: "main".into(), title: "Main story".into() }];
        let chapter = Chapter {
            id: "c".into(),
            title: "Chapter".into(),
            group: Some("main".into()),
            progression: Some(Progression::Flexible),
            default_size: Some(1.5),
            ..Chapter::default()
        };
        let mut form = Form::chapter(&chapter, &groups);
        assert_eq!(form.chapter_info(), Ok(ChapterInfo::of(&chapter)));
        form.click(&FormAction::Cycle(row(&form, "group")), &measure);
        assert_eq!(form.chapter_info().unwrap().group, None);

        let mut form = Form::group(&groups[0]);
        type_in(&mut form, "title", "Story");
        assert_eq!(form.save(), Ok(Edit::SetGroup { group: "main".into(), title: "Story".into() }));

        let settings = Settings::default();
        let mut form = Form::settings(&settings);
        assert_eq!(form.pack_settings(), Ok(PackSettings::of(&settings)));
        type_in(&mut form, "detection_delay", "0");
        assert!(form.save().is_err());

        let entry = TableEntry { weight: 4, title: Some("Snack".into()), kind: RewardKind::Xp { points: 3 } };
        let form = Form::entry("loot", Some(2), Some(&entry));
        assert_eq!(form.entry_value(), Ok(entry));
        assert_eq!(form.delete(), Some(Edit::RemoveEntry { table: "loot".into(), index: 2 }));
        // No table inside a table: the types stop at the message.
        let mut form = Form::entry("loot", None, None);
        let t = row(&form, "type");
        for _ in 0..5 {
            form.click(&FormAction::Cycle(t), &measure);
        }
        assert_eq!(form.chosen("type"), "item");
    }

    #[test]
    fn item_suggestions_put_names_that_start_with_the_text_first() {
        let names: HashMap<String, String> =
            [("minecraft:oak_log", "Oak Log"), ("minecraft:stripped_oak_log", "Stripped Oak Log"), ("minecraft:stone", "Stone")]
                .into_iter()
                .map(|(a, b)| (a.to_owned(), b.to_owned()))
                .collect();
        let mut form = Form::quest("a/q", &quest());
        type_in(&mut form, "icon", "oak");
        let ids: Vec<String> = form.suggestions(&names).into_iter().map(|(id, _)| id).collect();
        assert_eq!(ids, ["minecraft:oak_log", "minecraft:stripped_oak_log"]);
        let (_, zones) = form.layout([0.0, 0.0, 480.0, 400.0], [-1.0, -1.0], 0, &names, &measure);
        let icon = row(&form, "icon");
        assert!(zones.iter().any(|(_, a)| *a == FormAction::Pick(icon, "minecraft:oak_log".into())));
        form.click(&FormAction::Pick(icon, "minecraft:oak_log".into()), &measure);
        assert_eq!(form.quest_info().unwrap().icon.as_deref(), Some("minecraft:oak_log"));
    }
}
