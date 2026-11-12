// Edit mode in place: the small panels and the item picker the book shows
// beside a quest's window, instead of a big form. A panel is a column of
// controls built from what it edits each frame (a quest's settings, a task,
// a reward, a chapter, a table entry, the pack settings): segmented
// buttons, switches, − / + numbers, the node shapes as they are drawn, an
// item slot that opens the picker, a line to type in, a red button. Each
// click changes the thing at once (the book sends the edit). The picker is a
// grid of every item, or every mob (by spawn egg), with a search line.
// Pure: it lays itself out as hq_book draws and click zones.
use super::hq_book::{Draw, NODE_SPRITE, entry_label, fit_text, node_cell, pretty, reward_item};
use super::hq_canvas as canvas;
use super::hq_edit::{ChapterInfo, Edit, PackSettings, QuestInfo};
use super::hq_logic::{Book, QuestState};
use super::hq_model::{
    AutoClaim, DependencyMode, Group, ItemMatch, Progression, Reward, RewardKind, Shape, TableEntry, Task, TaskKind, Visibility,
    resource_id,
};
use super::hq_text::TextField;
use std::collections::HashMap;

/// One control of a panel. `key` names it in the actions it gives.
#[derive(Clone, Debug, PartialEq)]
pub enum Control {
    /// A line to type in; `hint` shows while it is empty.
    Text { key: &'static str, label: &'static str, value: String, hint: &'static str },
    /// A number with − and + (and the wheel), typed after a click on it;
    /// `zero` names the lowest value ("auto", "none", "exactly") when it
    /// means something else.
    Number { key: &'static str, label: &'static str, value: f32, step: f32, min: f32, max: f32, zero: Option<&'static str> },
    Toggle { key: &'static str, label: &'static str, on: bool },
    /// Buttons side by side, one of them chosen.
    Segments { key: &'static str, label: &'static str, options: Vec<String>, at: Option<usize> },
    /// The node shapes, drawn; with a "Default" button first when `auto`
    /// is some (the shape it then has).
    Shapes { key: &'static str, label: &'static str, at: Option<Shape>, auto: Option<Shape> },
    /// An item slot: a click opens the picker. `mob` shows an entity by its egg.
    Item { key: &'static str, label: &'static str, item: String, mob: bool },
    /// Something chosen from a list (an advancement), shown by `icon` and
    /// `name`; `value` is its id. A click opens the picker.
    Choose { key: &'static str, label: &'static str, value: String, icon: String, name: String },
    /// A list across the panel under its label: one row per entry, (icon,
    /// text, chance in percent), a click edits that entry; a chance has
    /// - / + and is typed after a click. A last row adds one (and, with
    /// chances, evens them). A reward table's entries in a reward's panel.
    /// `step`: what - / + and the wheel move a chance by (the book keeps it).
    Rows { key: &'static str, label: &'static str, rows: Vec<(String, String, Option<f32>)>, add: &'static str, step: f32 },
    /// A red button across the panel.
    Danger { key: &'static str, label: &'static str },
}

/// What a click on a panel does.
#[derive(Clone, Debug, PartialEq)]
pub enum PanelAction {
    /// Start typing in a text control.
    Type(&'static str),
    Step(&'static str, f32),
    /// A number typed: its new value.
    Set(&'static str, f32),
    Toggle(&'static str),
    Segment(&'static str, usize),
    Shape(&'static str, Option<Shape>),
    /// Open the picker for an item control.
    Pick(&'static str),
    /// A red button.
    Press(&'static str),
    /// Row `1` of list `0`, and its "add" row.
    Row(&'static str, usize),
    AddRow(&'static str),
    /// Row `1`'s chance moved by `2` steps (-1 or 1); 0: typed after a click.
    RowStep(&'static str, usize, f32),
    /// What a step of the list's chances is, in percent.
    ChanceStep(&'static str, f32),
    /// Every row the same chance.
    Even(&'static str),
    Close,
    Stay,
}

/// What a click on the picker does.
#[derive(Clone, Debug, PartialEq)]
pub enum PickAction {
    /// Take this item id (or `#tag`, or entity id).
    Choose(String),
    /// Start typing in the search line.
    Search,
    Close,
    Stay,
}

const SHAPES: [Shape; 4] = [Shape::Square, Shape::Circle, Shape::Diamond, Shape::Hexagon];

/// Sizes and colours, close to the quest window's.
/// A panel's width: the book moves the quest window aside to make room.
pub const PANEL_W: f32 = 212.0;
const LABEL_W: f32 = 72.0;
const BAR: f32 = 13.0;
const ROW: f32 = 14.0;
const PAD: f32 = 4.0;
pub const GAP: f32 = 4.0;
const BACK: u32 = 0xF41C_2230;
const EDGE: u32 = 0xFF3A_4458;
const BAR_BACK: u32 = 0xFF20_2030;
const SLOT: u32 = 0xFF0C_0E14;
const SLOT_EDGE: u32 = 0xFF4A_5468;
const FOCUS_EDGE: u32 = 0xFF80_90C0;
const CHIP: u32 = 0xFF2A_3044;
const CHIP_HOVER: u32 = 0xFF3A_4460;
const CHIP_ON: u32 = 0xFF3A_5A8A;
const CHIP_ON_EDGE: u32 = 0xFF7A_A0E0;
const SWITCH_OFF: u32 = 0xFF3A_3E4A;
const SWITCH_ON: u32 = 0xFF3C_A04C;
const KNOB: u32 = 0xFFE8_E8F0;
const DANGER: u32 = 0xFF7A_2E2E;
const DANGER_HOVER: u32 = 0xFFA0_3C3C;
const INK: u32 = 0xFF_FFFF;
const DIM_INK: u32 = 0x90_90A0;
const LABEL_INK: u32 = 0xB8_B8C4;
const CURSOR: u32 = 0xFFE0_E0E8;
const ADD_INK: u32 = 0x80_E080;
const CHANCE_INK: u32 = 0xFF_D860;
/// The picker's grid.
const CELL: f32 = 18.0;
const COLUMNS: usize = 9;
const ROWS: usize = 6;

/// A number as shown: whole numbers plain, others with up to two decimals.
fn number_text(value: f32) -> String {
    if value.fract() == 0.0 { format!("{value:.0}") } else { format!("{value:.2}").trim_end_matches('0').to_owned() }
}

/// `value` moved by `by`, rounded to hundredths (quarter steps stay exact).
fn stepped(value: f32, by: f32) -> f32 {
    ((value + by) * 100.0).round() / 100.0
}

/// The control `key` of `controls`.
fn control<'a>(controls: &'a [Control], key: &str) -> Option<&'a Control> {
    controls.iter().find(|c| match c {
        Control::Text { key: k, .. }
        | Control::Number { key: k, .. }
        | Control::Toggle { key: k, .. }
        | Control::Segments { key: k, .. }
        | Control::Shapes { key: k, .. }
        | Control::Item { key: k, .. }
        | Control::Rows { key: k, .. }
        | Control::Choose { key: k, .. }
        | Control::Danger { key: k, .. } => *k == key,
    })
}

/// The new value of number control `key` after `action`, if it is a step on it.
fn new_number(controls: &[Control], action: &PanelAction) -> Option<(&'static str, f32)> {
    let (key, by, set) = match action {
        PanelAction::Step(key, by) => (*key, *by, None),
        PanelAction::Set(key, value) => (*key, 0.0, Some(*value)),
        _ => return None,
    };
    match control(controls, key)? {
        Control::Number { value, step, min, max, .. } => {
            let moved = set.unwrap_or_else(|| stepped(*value, by * step)).clamp(*min, *max);
            Some((key, moved))
        }
        _ => None,
    }
}

// ---------------------------------------------------------------- quests

const VISIBILITIES: [(Visibility, &str); 4] = [
    (Visibility::Always, "Always"),
    (Visibility::UntilDepsVisible, "After a need shows"),
    (Visibility::UntilDepsComplete, "After needs done"),
    (Visibility::Secret, "Secret"),
];
const MODES: [(DependencyMode, &str); 4] = [
    (DependencyMode::AllCompleted, "All done"),
    (DependencyMode::OneCompleted, "One done"),
    (DependencyMode::AllStarted, "All started"),
    (DependencyMode::OneStarted, "One started"),
];

/// A quest's settings: everything its window does not show in place.
/// `auto` is the shape it has when it names none.
pub fn quest_controls(info: &QuestInfo, auto: Shape) -> Vec<Control> {
    let segments = |key, label, options: &[&str], at: usize| Control::Segments {
        key,
        label,
        options: options.iter().map(|o| (*o).to_owned()).collect(),
        at: Some(at),
    };
    vec![
        Control::Shapes { key: "shape", label: "Shape", at: info.shape, auto: Some(auto) },
        Control::Number { key: "size", label: "Size", value: info.size.unwrap_or(0.0), step: 0.25, min: 0.0, max: 4.0, zero: Some("auto") },
        segments("visibility", "Shown", &VISIBILITIES.map(|(_, l)| l), VISIBILITIES.iter().position(|(v, _)| *v == info.visibility).unwrap_or(0)),
        segments("needs", "Opens when", &MODES.map(|(_, l)| l), MODES.iter().position(|(m, _)| *m == info.dependency_mode).unwrap_or(0)),
        Control::Number {
            key: "min",
            label: "Needed",
            value: info.min_required.unwrap_or(0) as f32,
            step: 1.0,
            min: 0.0,
            max: 99.0,
            zero: Some("as above"),
        },
        Control::Toggle { key: "optional", label: "Optional", on: info.optional },
        Control::Toggle { key: "sequential", label: "In order", on: info.sequential_tasks },
        Control::Toggle { key: "repeatable", label: "Repeatable", on: info.repeatable },
        Control::Number {
            key: "cooldown",
            label: "Cooldown",
            value: info.repeat_cooldown as f32,
            step: 60.0,
            min: 0.0,
            max: 604_800.0,
            zero: Some("none"),
        },
    ]
}

/// `info` after `action` on its settings panel; false when nothing changed.
pub fn quest_change(info: &mut QuestInfo, auto: Shape, action: &PanelAction) -> bool {
    let before = info.clone();
    let controls = quest_controls(info, auto);
    if let Some((key, value)) = new_number(&controls, action) {
        match key {
            // Below a quarter, the size is the chapter's again.
            "size" => info.size = (value >= 0.25).then_some(value),
            "min" => info.min_required = (value >= 1.0).then_some(value as u32),
            _ => info.repeat_cooldown = value as u32,
        }
    }
    match action {
        PanelAction::Shape(_, shape) => info.shape = *shape,
        PanelAction::Segment("visibility", i) => info.visibility = VISIBILITIES.get(*i).map_or(info.visibility, |(v, _)| *v),
        PanelAction::Segment("needs", i) => info.dependency_mode = MODES.get(*i).map_or(info.dependency_mode, |(m, _)| *m),
        PanelAction::Toggle("optional") => info.optional = !info.optional,
        PanelAction::Toggle("sequential") => info.sequential_tasks = !info.sequential_tasks,
        PanelAction::Toggle("repeatable") => info.repeatable = !info.repeatable,
        _ => {}
    }
    *info != before
}

// ---------------------------------------------------------------- tasks

/// The dimensions offered by name; another one keeps its id as an option.
const DIMENSIONS: [(&str, &str); 3] =
    [("minecraft:overworld", "Overworld"), ("minecraft:the_nether", "Nether"), ("minecraft:the_end", "The End")];

/// The dimension segments for `dimension`, with "Any" first when `any`.
fn dimension_segments(key: &'static str, label: &'static str, dimension: Option<&str>, any: bool) -> Control {
    let mut options: Vec<String> = Vec::new();
    if any {
        options.push("Any".to_owned());
    }
    options.extend(DIMENSIONS.iter().map(|(_, name)| (*name).to_owned()));
    let known = dimension.and_then(|d| DIMENSIONS.iter().position(|(id, _)| *id == d));
    let at = match (dimension, known) {
        (None, _) => Some(0).filter(|_| any),
        (Some(_), Some(k)) => Some(k + usize::from(any)),
        (Some(other), None) => {
            options.push(pretty(other));
            Some(options.len() - 1)
        }
    };
    Control::Segments { key, label, options, at }
}

/// The dimension chosen as segment `i` (None: any).
fn dimension_at(i: usize, any: bool, current: Option<&str>) -> Option<String> {
    let i = if any {
        match i.checked_sub(1) {
            None => return None,
            Some(i) => i,
        }
    } else {
        i
    };
    DIMENSIONS.get(i).map(|(id, _)| (*id).to_owned()).or_else(|| current.map(str::to_owned))
}

/// `[x, y, z]` as `x y z`.
fn place_text(at: [i32; 3]) -> String {
    format!("{} {} {}", at[0], at[1], at[2])
}

/// What a task asks, as controls (its type stays: delete it and add another).
pub fn task_controls(task: &Task) -> Vec<Control> {
    let count = |value: u32| Control::Number { key: "count", label: "How many", value: value as f32, step: 1.0, min: 1.0, max: 9999.0, zero: None };
    let mut controls = match &task.kind {
        TaskKind::Item { item, count: n, consume } => vec![
            Control::Item { key: "item", label: "Item", item: String::from(item.clone()), mob: false },
            count(*n),
            Control::Toggle { key: "consume", label: "Handed in", on: *consume },
        ],
        TaskKind::Craft { item, count: n } => {
            vec![Control::Item { key: "item", label: "Item", item: String::from(item.clone()), mob: false }, count(*n)]
        }
        TaskKind::Kill { entity, count: n } => vec![Control::Item { key: "entity", label: "Mob", item: entity.clone(), mob: true }, count(*n)],
        TaskKind::XpLevels { levels } => {
            vec![Control::Number { key: "levels", label: "Level", value: *levels as f32, step: 1.0, min: 1.0, max: 1000.0, zero: None }]
        }
        TaskKind::Dimension { dimension } => vec![dimension_segments("dimension", "Dimension", Some(dimension), false)],
        TaskKind::Location { dimension, from, to } => vec![
            dimension_segments("dimension", "Dimension", dimension.as_deref(), true),
            Control::Text { key: "from", label: "From x y z", value: place_text(*from), hint: "" },
            Control::Text { key: "to", label: "To x y z", value: place_text(*to), hint: "" },
        ],
        TaskKind::Advancement { advancement } => vec![Control::Choose {
            key: "advancement",
            label: "Advancement",
            value: advancement.clone(),
            icon: "minecraft:knowledge_book".to_owned(),
            name: advancement_name(advancement),
        }],
        TaskKind::Checkmark => Vec::new(),
    };
    controls.push(Control::Text { key: "title", label: "Title", value: task.title.clone().unwrap_or_default(), hint: "automatic" });
    controls.push(Control::Danger { key: "delete", label: "Delete this task" });
    controls
}

/// An advancement's name for the editor: its group and its own name
/// (`minecraft:story/mine_stone` is "Story: Mine stone"), the namespace
/// added when it is not the game's.
pub fn advancement_name(id: &str) -> String {
    let (namespace, path) = id.split_once(':').unwrap_or(("minecraft", id));
    let name = pretty(path);
    let group = path.rsplit_once('/').map(|(group, _)| pretty(group));
    let named = match group {
        Some(group) if group != name => format!("{group}: {name}"),
        _ => name,
    };
    if namespace == "minecraft" { named } else { format!("{named} ({namespace})") }
}

/// `task` after `action`; false when nothing changed.
pub fn task_change(task: &mut Task, action: &PanelAction) -> bool {
    let before = task.clone();
    let controls = task_controls(task);
    let number = new_number(&controls, action).map(|(_, v)| v as u32);
    match (&mut task.kind, action) {
        (TaskKind::Item { count, .. } | TaskKind::Craft { count, .. } | TaskKind::Kill { count, .. }, _) if number.is_some() => {
            *count = number.unwrap_or(1);
        }
        (TaskKind::XpLevels { levels }, _) if number.is_some() => *levels = number.unwrap_or(1),
        (TaskKind::Item { consume, .. }, PanelAction::Toggle("consume")) => *consume = !*consume,
        (TaskKind::Dimension { dimension }, PanelAction::Segment(_, i)) => {
            *dimension = dimension_at(*i, false, Some(dimension)).unwrap_or_else(|| dimension.clone());
        }
        (TaskKind::Location { dimension, .. }, PanelAction::Segment(_, i)) => *dimension = dimension_at(*i, true, dimension.as_deref()),
        _ => {}
    }
    *task != before
}

/// Text typed into task control `key`, or why it is not right.
pub fn task_text(task: &mut Task, key: &str, text: &str) -> Result<(), String> {
    let text = text.trim();
    let place = |label: &str| -> Result<[i32; 3], String> {
        let parts: Vec<i32> = text.split_whitespace().filter_map(|p| p.parse().ok()).collect();
        <[i32; 3]>::try_from(parts).map_err(|_| format!("{label}: three whole numbers, x y z."))
    };
    match (&mut task.kind, key) {
        (_, "title") => task.title = Some(text.to_owned()).filter(|t| !t.is_empty()),
        (TaskKind::Location { from, .. }, "from") => *from = place("From")?,
        (TaskKind::Location { to, .. }, "to") => *to = place("To")?,
        (TaskKind::Advancement { advancement }, "advancement") => {
            *advancement = resource_id(text).ok_or_else(|| format!("`{text}` is not an advancement id."))?;
        }
        _ => {}
    }
    Ok(())
}

/// An item (or entity) picked for task control `key`.
pub fn task_pick(task: &mut Task, key: &str, id: &str) {
    match (&mut task.kind, key) {
        (TaskKind::Item { item, .. } | TaskKind::Craft { item, .. }, "item") => {
            if let Ok(found) = ItemMatch::try_from(id.to_owned()) {
                *item = found;
            }
        }
        (TaskKind::Kill { entity, .. }, "entity") => *entity = id.to_owned(),
        (TaskKind::Advancement { advancement }, "advancement") => {
            if let Some(found) = resource_id(id) {
                *advancement = found;
            }
        }
        _ => {}
    }
}

// ---------------------------------------------------------------- rewards

/// Whether a reward is given as soon as its quest is done (else the player
/// clicks it); `pack` is the pack's setting, for a reward that names none.
fn given_at_once(auto: AutoClaim, pack: bool) -> bool {
    match auto {
        AutoClaim::Default => pack,
        AutoClaim::Enabled => true,
        AutoClaim::Disabled => false,
    }
}

/// What a reward (or a table entry) gives, as controls.
fn kind_controls(kind: &RewardKind) -> Vec<Control> {
    let number = |key, label, value: u32, max: f32| Control::Number { key, label, value: value as f32, step: 1.0, min: 1.0, max, zero: None };
    match kind {
        RewardKind::Item { item, count, max_count } => vec![
            Control::Item { key: "item", label: "Item", item: item.clone(), mob: false },
            number("count", "How many", *count, 9999.0),
            // From the count up: the count itself means exactly that many.
            Control::Number {
                key: "max",
                label: "Up to",
                value: max_count.unwrap_or(*count).max(*count) as f32,
                step: 1.0,
                min: *count as f32,
                max: 9999.0,
                zero: Some("exactly"),
            },
        ],
        RewardKind::Xp { .. } | RewardKind::XpLevels { .. } => {
            let (amount, levels) = match kind {
                RewardKind::XpLevels { levels } => (*levels, true),
                RewardKind::Xp { points } => (*points, false),
                _ => (0, false),
            };
            vec![
                Control::Segments {
                    key: "unit",
                    label: "Experience",
                    options: vec!["Points".to_owned(), "Levels".to_owned()],
                    at: Some(usize::from(levels)),
                },
                number("amount", "How much", amount, if levels { 1000.0 } else { 100_000.0 }),
            ]
        }
        RewardKind::Command { command } => {
            vec![Control::Text { key: "command", label: "Command", value: command.clone(), hint: "give {player} diamond" }]
        }
        RewardKind::Toast { text } => vec![Control::Text { key: "text", label: "Message", value: text.clone(), hint: "Well done!" }],
        // The table's entries are listed by the reward's panel; a click on
        // its name picks another table from a menu.
        RewardKind::Random { table } | RewardKind::Choice { table } => vec![Control::Choose {
            key: "table",
            label: "Table",
            value: table.clone(),
            icon: "minecraft:barrel".to_owned(),
            name: table.clone(),
        }],
    }
}

/// A new experience reward's amount in points, and in levels.
const XP_POINTS: u32 = 50;
const XP_LEVELS: u32 = 1;

/// `kind` after `action` on its controls.
fn kind_change(kind: &mut RewardKind, action: &PanelAction) {
    let controls = kind_controls(kind);
    let number = new_number(&controls, action);
    match (kind, number, action) {
        (RewardKind::Item { count, max_count, .. }, Some(("count", v)), _) => {
            *count = v as u32;
            // A range that no longer goes above the count is gone.
            *max_count = max_count.filter(|max| *max > *count);
        }
        (RewardKind::Item { count, max_count, .. }, Some(("max", v)), _) => *max_count = (v as u32 > *count).then_some(v as u32),
        (RewardKind::Xp { points }, Some((_, v)), _) => *points = v as u32,
        (RewardKind::XpLevels { levels }, Some((_, v)), _) => *levels = v as u32,
        (kind @ RewardKind::Xp { .. }, _, PanelAction::Segment("unit", 1)) => {
            // Each unit starts from its own default: 50 points, 1 level.
            *kind = RewardKind::XpLevels { levels: XP_LEVELS };
        }
        (kind @ RewardKind::XpLevels { .. }, _, PanelAction::Segment("unit", 0)) => {
            *kind = RewardKind::Xp { points: XP_POINTS };
        }
        _ => {}
    }
}

/// Text typed into a reward kind's control `key`.
fn kind_text(kind: &mut RewardKind, key: &str, text: &str) -> Result<(), String> {
    match (kind, key) {
        (RewardKind::Command { command }, "command") => {
            let line = text.trim().trim_start_matches('/');
            if line.is_empty() {
                return Err("Write the command to run.".to_owned());
            }
            *command = line.to_owned();
        }
        (RewardKind::Toast { text: message }, "text") => *message = text.trim().to_owned(),
        _ => {}
    }
    Ok(())
}

/// A reward's controls: what it gives, its title, when it is claimed, for whom.
/// `pack` is the pack's "rewards claimed by themselves" setting.
pub fn reward_controls(title: Option<&str>, auto: AutoClaim, team: bool, kind: &RewardKind, pack: bool) -> Vec<Control> {
    let mut controls = kind_controls(kind);
    controls.push(Control::Text { key: "title", label: "Title", value: title.unwrap_or("").to_owned(), hint: "automatic" });
    // A choice is always picked by a click.
    if !matches!(kind, RewardKind::Choice { .. }) {
        controls.push(Control::Toggle { key: "auto", label: "Given at once", on: given_at_once(auto, pack) });
    }
    controls.push(Control::Toggle { key: "team", label: "Whole party", on: team });
    controls.push(Control::Danger { key: "delete", label: "Delete this reward" });
    controls
}

/// A reward's parts after `action`; false when nothing changed.
pub fn reward_change(auto: &mut AutoClaim, team: &mut bool, kind: &mut RewardKind, pack: bool, action: &PanelAction) -> bool {
    let before = (*auto, *team, kind.clone());
    match action {
        PanelAction::Toggle("auto") => *auto = if given_at_once(*auto, pack) { AutoClaim::Disabled } else { AutoClaim::Enabled },
        PanelAction::Toggle("team") => *team = !*team,
        _ => kind_change(kind, action),
    }
    (*auto, *team, kind.clone()) != before
}

/// Text typed into reward control `key` (`title` included).
pub fn reward_text(title: &mut Option<String>, kind: &mut RewardKind, key: &str, text: &str) -> Result<(), String> {
    if key == "title" {
        *title = Some(text.trim().to_owned()).filter(|t| !t.is_empty());
        return Ok(());
    }
    kind_text(kind, key, text)
}

/// An item picked for a reward's item.
pub fn reward_pick(kind: &mut RewardKind, id: &str) {
    if let RewardKind::Item { item, .. } = kind
        && !id.starts_with('#')
    {
        *item = id.to_owned();
    }
}

/// How the rewards use a table: drawn at random (with this entry's chance,
/// in percent), with no odds to set, or not at all.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TableUse {
    Random(f32),
    /// Chosen from, or the only entry of a random table: no odds to set.
    NoOdds,
    Unused,
}

/// A reward table entry's controls: what it gives, its title, its chance
/// (a random table) or its weight (a table no reward uses).
pub fn entry_controls(title: Option<&str>, weight: u32, kind: &RewardKind, used: TableUse) -> Vec<Control> {
    let mut controls = kind_controls(kind);
    controls.push(Control::Text { key: "title", label: "Title", value: title.unwrap_or("").to_owned(), hint: "automatic" });
    match used {
        TableUse::Random(percent) => {
            controls.push(Control::Number { key: "chance", label: "Chance %", value: (percent * 10.0).round() / 10.0, step: 1.0, min: 0.1, max: 99.9, zero: None });
        }
        TableUse::NoOdds => {}
        TableUse::Unused => {
            controls.push(Control::Number { key: "weight", label: "Weight", value: weight as f32, step: 1.0, min: 0.0, max: 1000.0, zero: None });
        }
    }
    controls.push(Control::Danger { key: "delete", label: "Delete this entry" });
    controls
}

/// How the pack's rewards use table `table`, for its entry `index`.
fn table_use(book: &Book, table: &str, index: usize) -> TableUse {
    let kinds = book.file().chapters.iter().flat_map(|c| &c.quests).flat_map(|q| &q.rewards).map(|r| &r.kind);
    let (mut random, mut choice) = (false, false);
    for kind in kinds {
        match kind {
            RewardKind::Random { table: t } if t == table => random = true,
            RewardKind::Choice { table: t } if t == table => choice = true,
            _ => {}
        }
    }
    let entries = book.table(table).map_or(&[][..], |t| &t.entries[..]);
    let total: u32 = entries.iter().map(|e| e.weight).sum();
    let weight = entries.get(index).map_or(0, |e| e.weight);
    match (random, choice) {
        (true, _) if entries.len() <= 1 => TableUse::NoOdds,
        (true, _) => TableUse::Random(if total == 0 { 0.0 } else { weight as f32 * 100.0 / total as f32 }),
        (false, true) => TableUse::NoOdds,
        (false, false) => TableUse::Unused,
    }
}

/// An entry's parts after `action`; false when nothing changed.
pub fn entry_change(weight: &mut u32, kind: &mut RewardKind, action: &PanelAction) -> bool {
    let before = (*weight, kind.clone());
    let controls = entry_controls(None, *weight, kind, TableUse::Unused);
    match new_number(&controls, action) {
        Some(("weight", v)) => *weight = v as u32,
        _ => kind_change(kind, action),
    }
    (*weight, kind.clone()) != before
}

// ---------------------------------------------------------------- chapters and the pack

const PROGRESSIONS: [(Progression, &str); 2] = [(Progression::Linear, "Linear"), (Progression::Flexible, "Flexible")];

/// A chapter's controls; `auto` is the pack's quest shape, `groups` the pack's groups.
pub fn chapter_controls(info: &ChapterInfo, auto: Shape, groups: &[Group]) -> Vec<Control> {
    let mut group_options = vec!["None".to_owned()];
    group_options.extend(groups.iter().map(|g| if g.title.is_empty() { g.id.clone() } else { g.title.clone() }));
    let group_at = match &info.group {
        None => Some(0),
        Some(id) => groups.iter().position(|g| g.id == *id).map(|i| i + 1),
    };
    let mut progressions = vec!["Pack's".to_owned()];
    progressions.extend(PROGRESSIONS.iter().map(|(_, l)| (*l).to_owned()));
    let progression_at = match info.progression {
        None => 0,
        Some(p) => PROGRESSIONS.iter().position(|(q, _)| *q == p).map_or(0, |i| i + 1),
    };
    vec![
        Control::Text { key: "title", label: "Title", value: info.title.clone(), hint: "" },
        Control::Text { key: "subtitle", label: "Subtitle", value: info.subtitle.clone().unwrap_or_default(), hint: "none" },
        Control::Item { key: "icon", label: "Icon", item: info.icon.clone().unwrap_or_default(), mob: false },
        Control::Segments { key: "group", label: "Group", options: group_options, at: group_at },
        Control::Toggle { key: "hidden", label: "Hidden", on: info.hidden },
        Control::Segments { key: "progression", label: "Progression", options: progressions, at: Some(progression_at) },
        Control::Shapes { key: "shape", label: "Quest shape", at: info.default_shape, auto: Some(auto) },
        Control::Number {
            key: "size",
            label: "Quest size",
            value: info.default_size.unwrap_or(0.0),
            step: 0.25,
            min: 0.0,
            max: 4.0,
            zero: Some("auto"),
        },
    ]
}

/// A chapter after `action`; false when nothing changed.
pub fn chapter_change(info: &mut ChapterInfo, auto: Shape, groups: &[Group], action: &PanelAction) -> bool {
    let before = info.clone();
    if let Some((_, v)) = new_number(&chapter_controls(info, auto, groups), action) {
        info.default_size = (v >= 0.25).then_some(v);
    }
    match action {
        PanelAction::Shape(_, shape) => info.default_shape = *shape,
        PanelAction::Toggle("hidden") => info.hidden = !info.hidden,
        PanelAction::Segment("group", i) => info.group = i.checked_sub(1).and_then(|g| groups.get(g)).map(|g| g.id.clone()),
        PanelAction::Segment("progression", i) => info.progression = i.checked_sub(1).and_then(|p| PROGRESSIONS.get(p)).map(|(p, _)| *p),
        _ => {}
    }
    *info != before
}

/// Text typed into chapter control `key`.
pub fn chapter_text(info: &mut ChapterInfo, key: &str, text: &str) {
    let text = text.trim().to_owned();
    match key {
        "title" => info.title = text,
        "subtitle" => info.subtitle = Some(text).filter(|t| !t.is_empty()),
        _ => {}
    }
}

/// The pack settings' controls.
pub fn settings_controls(settings: &PackSettings) -> Vec<Control> {
    vec![
        Control::Segments {
            key: "progression",
            label: "Progression",
            options: PROGRESSIONS.iter().map(|(_, l)| (*l).to_owned()).collect(),
            at: PROGRESSIONS.iter().position(|(p, _)| *p == settings.progression),
        },
        Control::Shapes { key: "shape", label: "Quest shape", at: Some(settings.default_shape), auto: None },
        Control::Number { key: "size", label: "Quest size", value: settings.default_size, step: 0.25, min: 0.25, max: 4.0, zero: None },
        Control::Toggle { key: "auto_claim", label: "Auto claim", on: settings.auto_claim_rewards },
        Control::Number {
            key: "delay",
            label: "Look (ticks)",
            value: settings.detection_delay as f32,
            step: 5.0,
            min: 5.0,
            max: 1200.0,
            zero: None,
        },
    ]
}

/// The pack settings after `action`; false when nothing changed.
pub fn settings_change(settings: &mut PackSettings, action: &PanelAction) -> bool {
    let before = settings.clone();
    match new_number(&settings_controls(settings), action) {
        Some(("size", v)) => settings.default_size = v,
        Some((_, v)) => settings.detection_delay = v as u32,
        None => {}
    }
    match action {
        PanelAction::Shape(_, Some(shape)) => settings.default_shape = *shape,
        PanelAction::Toggle("auto_claim") => settings.auto_claim_rewards = !settings.auto_claim_rewards,
        PanelAction::Segment("progression", i) => settings.progression = PROGRESSIONS.get(*i).map_or(settings.progression, |(p, _)| *p),
        _ => {}
    }
    *settings != before
}

/// A group's controls.
pub fn group_controls(title: &str) -> Vec<Control> {
    vec![
        Control::Text { key: "title", label: "Title", value: title.to_owned(), hint: "" },
        Control::Danger { key: "delete", label: "Delete the group" },
    ]
}

// ---------------------------------------------------------------- what a panel edits

/// What a panel edits. Quests, tasks and rewards by full id.
#[derive(Clone, Debug, PartialEq)]
pub enum Subject {
    Quest(String),
    Task(String),
    Reward(String),
    Chapter(String),
    Group(String),
    Settings,
    /// Entry `1` of reward table `0`.
    Entry(String, usize),
}

/// What the picker chooses for.
#[derive(Clone, Debug, PartialEq)]
pub enum Purpose {
    /// The icon of quest `0` (full id).
    QuestIcon(String),
    /// Item control `0` of the open panel.
    Panel(&'static str),
    /// A new task of quest `quest`.
    NewTask { quest: String, kind: NewTask },
    /// A new item reward of quest `0`.
    NewReward(String),
    /// A new random (or, with `choice`, chosen) reward of quest `quest`,
    /// from a new table whose first entry is the item picked.
    NewTableReward { quest: String, choice: bool },
    /// A new item entry of reward table `0`.
    NewEntry(String),
}

/// The new tasks that start with the picker.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NewTask {
    Have,
    HandIn,
    Craft,
    Kill,
}

/// What an entry of an "add" menu does: open the picker, or make it at once.
#[derive(Clone, Debug, PartialEq)]
pub enum Add {
    Pick(Purpose, bool),
    Edit(Edit),
}

/// The shape a quest has when it names none: its chapter's, else the pack's.
fn quest_auto(book: &Book, n: usize) -> Shape {
    let chapter = book.chapter(book.nodes()[n].chapter);
    chapter.default_shape.unwrap_or(book.settings().default_shape)
}

/// Whether `subject` is still in `book`.
pub fn subject_exists(book: &Book, subject: &Subject) -> bool {
    match subject {
        Subject::Quest(q) => book.find(q).is_some(),
        Subject::Task(t) => book.find_task(t).is_some(),
        Subject::Reward(r) => book.find_reward(r).is_some(),
        Subject::Chapter(c) => book.file().chapters.iter().any(|ch| ch.id == *c),
        Subject::Group(g) => book.settings().groups.iter().any(|gr| gr.id == *g),
        Subject::Settings => true,
        Subject::Entry(t, i) => book.table(t).is_some_and(|table| *i < table.entries.len()),
    }
}

/// The quest a subject belongs to, if any.
pub fn subject_quest(subject: &Subject) -> Option<&str> {
    match subject {
        Subject::Quest(q) => Some(q),
        Subject::Task(id) | Subject::Reward(id) => id.rsplit_once('/').map(|(q, _)| q),
        _ => None,
    }
}

/// A panel's title and controls for `subject`.
pub fn subject_panel(book: &Book, subject: &Subject) -> Option<(String, Vec<Control>)> {
    Some(match subject {
        Subject::Quest(q) => {
            let n = book.find(q)?;
            ("Quest settings".to_owned(), quest_controls(&QuestInfo::of(book.quest(n)), quest_auto(book, n)))
        }
        Subject::Task(t) => {
            let (n, i) = book.find_task(t)?;
            ("Task".to_owned(), task_controls(&book.quest(n).tasks[i]))
        }
        Subject::Reward(r) => {
            let (n, i) = book.find_reward(r)?;
            let reward = &book.quest(n).rewards[i];
            let pack = book.settings().auto_claim_rewards;
            let mut controls = reward_controls(reward.title.as_deref(), reward.auto_claim, reward.team, &reward.kind, pack);
            if let RewardKind::Random { table } | RewardKind::Choice { table } = &reward.kind {
                let choice = matches!(reward.kind, RewardKind::Choice { .. });
                let at = controls.iter().position(|c| matches!(c, Control::Choose { key: "table", .. })).map_or(0, |i| i + 1);
                controls.insert(at, Control::Text { key: "table_name", label: "Table name", value: table.clone(), hint: "" });
                controls.insert(at + 1, table_rows(book, table, choice));
            }
            ("Reward".to_owned(), controls)
        }
        Subject::Chapter(c) => {
            let chapter = book.file().chapters.iter().find(|ch| ch.id == *c)?;
            ("Chapter".to_owned(), chapter_controls(&ChapterInfo::of(chapter), book.settings().default_shape, &book.settings().groups))
        }
        Subject::Group(g) => {
            let group = book.settings().groups.iter().find(|gr| gr.id == *g)?;
            ("Group".to_owned(), group_controls(&group.title))
        }
        Subject::Settings => ("Pack settings".to_owned(), settings_controls(&PackSettings::of(book.settings()))),
        Subject::Entry(t, i) => {
            let entry = book.table(t)?.entries.get(*i)?;
            (format!("Table {t}, entry {}", i + 1), entry_controls(entry.title.as_deref(), entry.weight, &entry.kind, table_use(book, t, *i)))
        }
    })
}

/// Reward table `table`'s entries as a list: what each gives and, for a
/// random draw, its chance.
fn table_rows(book: &Book, table: &str, choice: bool) -> Control {
    let entries = book.table(table).map_or(&[][..], |t| &t.entries[..]);
    let rows = entries
        .iter()
        .enumerate()
        .map(|(i, e)| (reward_item(&e.kind).0, entry_label(e), (!choice).then(|| row_chance(book, table, i))))
        .collect();
    Control::Rows { key: "entries", label: if choice { "Choose from" } else { "Draws from" }, rows, add: "+ Add an item", step: DEFAULT_CHANCE_STEP }
}

/// The steps a list's chances move by, in percent, and the one at first.
pub const CHANCE_STEPS: [f32; 5] = [0.1, 0.5, 1.0, 2.0, 5.0];
pub const DEFAULT_CHANCE_STEP: f32 = 5.0;

/// Entry `index` of table `table`'s chance, in percent to a tenth.
pub fn row_chance(book: &Book, table: &str, index: usize) -> f32 {
    let entries = book.table(table).map_or(&[][..], |t| &t.entries[..]);
    let total: u32 = entries.iter().map(|e| e.weight).sum();
    match entries.get(index) {
        Some(e) if total > 0 => (f64::from(e.weight) * 1000.0 / f64::from(total)).round() as f32 / 10.0,
        _ => 0.0,
    }
}

/// A chance as shown: "25%", "12.5%".
pub fn chance_text(percent: f32) -> String {
    format!("{}%", number_text((percent * 10.0).round() / 10.0))
}

/// The edit a step on, or a value typed into, a chance of a reward's
/// table asks for: the new percent from `step` (added) or `typed`.
pub fn row_chance_edit(book: &Book, subject: &Subject, index: usize, step: f32, typed: Option<&str>) -> Result<Option<Edit>, String> {
    let Some(table) = subject_table(book, subject) else { return Ok(None) };
    if book.table(&table).is_none_or(|t| t.entries.len() < 2 || index >= t.entries.len()) {
        return Ok(None);
    }
    let percent = match typed {
        Some(text) => text.trim().trim_end_matches('%').trim().replace(',', ".").parse::<f32>().map_err(|_| "Chance: a number from 0.1 to 99.9.".to_owned())?,
        None => row_chance(book, &table, index) + step,
    };
    let percent = ((percent * 10.0).round() / 10.0).clamp(0.1, 99.9);
    Ok(((percent - row_chance(book, &table, index)).abs() > 0.05).then_some(Edit::SetChance { table, index, percent }))
}

/// The reward table a reward's panel lists, when it draws from one.
pub fn subject_table(book: &Book, subject: &Subject) -> Option<String> {
    let Subject::Reward(r) = subject else { return None };
    let (n, i) = book.find_reward(r)?;
    match &book.quest(n).rewards[i].kind {
        RewardKind::Random { table } | RewardKind::Choice { table } => Some(table.clone()),
        _ => None,
    }
}

/// The tables a drawn reward's panel offers (id, how many entries), and the
/// edit that makes the reward draw from each.
pub fn table_choices(book: &Book, subject: &Subject) -> Vec<(String, Edit)> {
    let Subject::Reward(r) = subject else { return Vec::new() };
    let Some((quest, reward)) = reward_of(book, r) else { return Vec::new() };
    book.file()
        .reward_tables
        .iter()
        .filter_map(|t| {
            let mut reward = reward.clone();
            match &mut reward.kind {
                RewardKind::Random { table } | RewardKind::Choice { table } => *table = t.id.clone(),
                _ => return None,
            }
            let label = format!("{} ({} {})", t.id, t.entries.len(), if t.entries.len() == 1 { "item" } else { "items" });
            Some((label, Edit::SaveReward { quest: quest.clone(), old: Some(reward.id.clone()), reward }))
        })
        .collect()
}

/// The task, reward or entry of `subject` as it is, to change.
fn task_of(book: &Book, id: &str) -> Option<(String, Task)> {
    let (n, i) = book.find_task(id)?;
    Some((book.nodes()[n].id.clone(), book.quest(n).tasks[i].clone()))
}

fn reward_of(book: &Book, id: &str) -> Option<(String, Reward)> {
    let (n, i) = book.find_reward(id)?;
    Some((book.nodes()[n].id.clone(), book.quest(n).rewards[i].clone()))
}

fn entry_of(book: &Book, table: &str, i: usize) -> Option<TableEntry> {
    book.table(table)?.entries.get(i).cloned()
}

fn chapter_info(book: &Book, id: &str) -> Option<ChapterInfo> {
    book.file().chapters.iter().find(|c| c.id == id).map(ChapterInfo::of)
}

/// The edit a click on `subject`'s panel asks for (a red button deletes).
pub fn subject_action(book: &Book, subject: &Subject, action: &PanelAction) -> Option<Edit> {
    let delete = matches!(action, PanelAction::Press("delete"));
    match subject {
        Subject::Quest(q) => {
            let n = book.find(q)?;
            let mut info = QuestInfo::of(book.quest(n));
            quest_change(&mut info, quest_auto(book, n), action).then(|| Edit::SetQuestInfo { quest: q.clone(), info })
        }
        Subject::Task(t) => {
            let (quest, mut task) = task_of(book, t)?;
            if delete {
                return Some(Edit::RemoveTask { quest, task: task.id });
            }
            let old = Some(task.id.clone());
            task_change(&mut task, action).then_some(Edit::SaveTask { quest, old, task })
        }
        Subject::Reward(r) => {
            let (quest, mut reward) = reward_of(book, r)?;
            if delete {
                return Some(Edit::RemoveReward { quest, reward: reward.id });
            }
            let pack = book.settings().auto_claim_rewards;
            let changed = reward_change(&mut reward.auto_claim, &mut reward.team, &mut reward.kind, pack, action);
            let old = Some(reward.id.clone());
            changed.then_some(Edit::SaveReward { quest, old, reward })
        }
        Subject::Chapter(c) => {
            let mut info = chapter_info(book, c)?;
            let changed = chapter_change(&mut info, book.settings().default_shape, &book.settings().groups, action);
            changed.then(|| Edit::SetChapter { chapter: c.clone(), info })
        }
        Subject::Group(g) => delete.then(|| Edit::DeleteGroup { group: g.clone() }),
        Subject::Settings => {
            let mut settings = PackSettings::of(book.settings());
            settings_change(&mut settings, action).then_some(Edit::SetSettings { settings })
        }
        Subject::Entry(t, i) => {
            if delete {
                return Some(Edit::RemoveEntry { table: t.clone(), index: *i });
            }
            // A chance changes every weight of the table.
            let entry = book.table(t)?.entries.get(*i)?;
            let controls = entry_controls(None, entry.weight, &entry.kind, table_use(book, t, *i));
            if let Some(("chance", percent)) = new_number(&controls, action) {
                return Some(Edit::SetChance { table: t.clone(), index: *i, percent });
            }
            let mut entry = entry_of(book, t, *i)?;
            let changed = entry_change(&mut entry.weight, &mut entry.kind, action);
            changed.then(|| Edit::SaveEntry { table: t.clone(), index: Some(*i), entry })
        }
    }
}

/// The edit text typed into `subject`'s control `key` asks for. A number
/// typed sets it (nothing typed: its lowest value).
pub fn subject_text(book: &Book, subject: &Subject, key: &str, text: &str) -> Result<Option<Edit>, String> {
    if let Some((_, controls)) = subject_panel(book, subject)
        && let Some(Control::Number { key: number, label, min, .. }) = control(&controls, key)
    {
        let value = match text.trim() {
            "" => *min,
            typed => typed.replace(',', ".").parse::<f32>().map_err(|_| format!("{label}: a number."))?,
        };
        return Ok(subject_action(book, subject, &PanelAction::Set(number, value)));
    }
    Ok(match subject {
        Subject::Task(t) => task_of(book, t).map(|(quest, mut task)| -> Result<Edit, String> {
            task_text(&mut task, key, text)?;
            Ok(Edit::SaveTask { quest, old: Some(task.id.clone()), task })
        }).transpose()?,
        Subject::Reward(_) if key == "table_name" => {
            subject_table(book, subject).map(|table| Edit::RenameTable { table, name: text.trim().to_owned() })
        }
        Subject::Reward(r) => reward_of(book, r).map(|(quest, mut reward)| -> Result<Edit, String> {
            reward_text(&mut reward.title, &mut reward.kind, key, text)?;
            Ok(Edit::SaveReward { quest, old: Some(reward.id.clone()), reward })
        }).transpose()?,
        Subject::Entry(t, i) => entry_of(book, t, *i).map(|mut entry| -> Result<Edit, String> {
            reward_text(&mut entry.title, &mut entry.kind, key, text)?;
            Ok(Edit::SaveEntry { table: t.clone(), index: Some(*i), entry })
        }).transpose()?,
        Subject::Chapter(c) => chapter_info(book, c).map(|mut info| {
            chapter_text(&mut info, key, text);
            Edit::SetChapter { chapter: c.clone(), info }
        }),
        Subject::Group(g) => Some(Edit::SetGroup { group: g.clone(), title: text.trim().to_owned() }),
        Subject::Quest(_) | Subject::Settings => None,
    })
}

/// What text or number control `key` of `subject` holds now, to start
/// typing from.
pub fn subject_value(book: &Book, subject: &Subject, key: &str) -> String {
    subject_panel(book, subject)
        .and_then(|(_, controls)| match control(&controls, key) {
            Some(Control::Text { value, .. } | Control::Choose { value, .. }) => Some(value.clone()),
            Some(Control::Number { value, .. }) => Some(number_text(*value)),
            _ => None,
        })
        .unwrap_or_default()
}

/// Whether item control `key` of `subject` picks a mob, and may take a tag.
pub fn subject_pick_kind(book: &Book, subject: &Subject, key: &str) -> (bool, bool) {
    let mob = subject_panel(book, subject).is_some_and(|(_, controls)| matches!(control(&controls, key), Some(Control::Item { mob: true, .. })));
    (mob, matches!(subject, Subject::Task(_)) && !mob)
}

/// The edit choosing item (or mob) `id` for `purpose` asks for; `subject`
/// is the open panel's.
pub fn picked(book: &Book, purpose: &Purpose, subject: Option<&Subject>, id: &str) -> Option<Edit> {
    match purpose {
        Purpose::QuestIcon(q) => {
            let mut info = QuestInfo::of(book.quest(book.find(q)?));
            info.icon = Some(id.to_owned()).filter(|i| !i.starts_with('#'));
            Some(Edit::SetQuestInfo { quest: q.clone(), info })
        }
        Purpose::NewTask { quest, kind } => {
            let item = || ItemMatch::try_from(id.to_owned()).ok();
            let kind = match kind {
                NewTask::Have => TaskKind::Item { item: item()?, count: 1, consume: false },
                NewTask::HandIn => TaskKind::Item { item: item()?, count: 1, consume: true },
                NewTask::Craft => TaskKind::Craft { item: item()?, count: 1 },
                NewTask::Kill => TaskKind::Kill { entity: id.to_owned(), count: 1 },
            };
            Some(Edit::SaveTask { quest: quest.clone(), old: None, task: Task { id: String::new(), title: None, kind } })
        }
        Purpose::NewReward(quest) => {
            let kind = RewardKind::Item { item: id.to_owned(), count: 1, max_count: None };
            let reward = Reward { id: String::new(), title: None, auto_claim: AutoClaim::Default, team: false, kind };
            Some(Edit::SaveReward { quest: quest.clone(), old: None, reward })
        }
        Purpose::NewTableReward { quest, choice } => {
            Some(Edit::NewTableReward { quest: quest.clone(), choice: *choice, item: id.to_owned() }).filter(|_| !id.starts_with('#'))
        }
        Purpose::NewEntry(table) => {
            let kind = RewardKind::Item { item: id.to_owned(), count: 1, max_count: None };
            let entry = TableEntry { weight: 1, title: None, kind };
            Some(Edit::SaveEntry { table: table.clone(), index: None, entry }).filter(|_| !id.starts_with('#'))
        }
        Purpose::Panel(key) => match subject? {
            Subject::Task(t) => {
                let (quest, mut task) = task_of(book, t)?;
                task_pick(&mut task, key, id);
                Some(Edit::SaveTask { quest, old: Some(task.id.clone()), task })
            }
            Subject::Reward(r) => {
                let (quest, mut reward) = reward_of(book, r)?;
                reward_pick(&mut reward.kind, id);
                Some(Edit::SaveReward { quest, old: Some(reward.id.clone()), reward })
            }
            Subject::Entry(t, i) => {
                let mut entry = entry_of(book, t, *i)?;
                reward_pick(&mut entry.kind, id);
                Some(Edit::SaveEntry { table: t.clone(), index: Some(*i), entry })
            }
            Subject::Chapter(c) => {
                let mut info = chapter_info(book, c)?;
                info.icon = Some(id.to_owned()).filter(|i| !i.starts_with('#'));
                Some(Edit::SetChapter { chapter: c.clone(), info })
            }
            _ => None,
        },
    }
}

/// Menu icons drawn by the book rather than as items: the checkmark task's
/// round button, and a question mark (a random reward).
pub const CHECK_ICON: &str = "horizon_quests:check_task";
pub const QUESTION_ICON: &str = "horizon_quests:question";

/// The "add a task" menu of quest `quest`: (label, icon, what it does).
pub fn add_task_menu(quest: &str) -> Vec<(String, String, Add)> {
    let pick = |kind, mobs| Add::Pick(Purpose::NewTask { quest: quest.to_owned(), kind }, mobs);
    let make = |kind: TaskKind| Add::Edit(Edit::SaveTask { quest: quest.to_owned(), old: None, task: Task { id: String::new(), title: None, kind } });
    vec![
        ("Have items", "minecraft:chest", pick(NewTask::Have, false)),
        ("Hand in items", "minecraft:hopper", pick(NewTask::HandIn, false)),
        ("Craft items", "minecraft:crafting_table", pick(NewTask::Craft, false)),
        ("Kill mobs", "minecraft:iron_sword", pick(NewTask::Kill, true)),
        ("Tick a box", CHECK_ICON, make(TaskKind::Checkmark)),
        ("Reach a level", "minecraft:experience_bottle", make(TaskKind::XpLevels { levels: 10 })),
        ("Visit a dimension", "minecraft:obsidian", make(TaskKind::Dimension { dimension: "minecraft:the_nether".into() })),
        ("Reach a place", "minecraft:compass", make(TaskKind::Location { dimension: None, from: [0, -64, 0], to: [16, 320, 16] })),
        ("Get an advancement", "minecraft:knowledge_book", make(TaskKind::Advancement { advancement: "minecraft:story/mine_stone".into() })),
    ]
    .into_iter()
    .map(|(label, icon, add)| (label.to_owned(), icon.to_owned(), add))
    .collect()
}

/// The "add a reward" menu of quest `quest`.
pub fn add_reward_menu(quest: &str) -> Vec<(String, String, Add)> {
    let make = |kind: RewardKind| {
        let reward = Reward { id: String::new(), title: None, auto_claim: AutoClaim::Default, team: false, kind };
        Add::Edit(Edit::SaveReward { quest: quest.to_owned(), old: None, reward })
    };
    // A drawn reward starts with its first item and a table of its own.
    let drawn = |choice| Add::Pick(Purpose::NewTableReward { quest: quest.to_owned(), choice }, false);
    let menu = vec![
        ("Items", "minecraft:chest", Add::Pick(Purpose::NewReward(quest.to_owned()), false)),
        ("Experience", "minecraft:experience_bottle", make(RewardKind::Xp { points: XP_POINTS })),
        ("Run a command", "minecraft:command_block", make(RewardKind::Command { command: "say {player} did it!".into() })),
        ("Show a message", "minecraft:paper", make(RewardKind::Toast { text: "Well done!".into() })),
        ("Random item", QUESTION_ICON, drawn(false)),
        ("Item of their choice", "minecraft:barrel", drawn(true)),
    ];
    menu.into_iter().map(|(label, icon, add)| (label.to_owned(), icon.to_owned(), add)).collect()
}

// ---------------------------------------------------------------- the panel on screen

/// What is being typed into a panel: its control's key and the field.
pub type Typing<'a> = Option<(&'a str, &'a TextField)>;

fn fill(draws: &mut Vec<Draw>, rect: [f32; 4], colour: u32) {
    draws.push(Draw::Fill { rect, colour });
}

fn frame(draws: &mut Vec<Draw>, [x, y, w, h]: [f32; 4], colour: u32) {
    for edge in [[x, y, w, 1.0], [x, y + h - 1.0, w, 1.0], [x, y + 1.0, 1.0, h - 2.0], [x + w - 1.0, y + 1.0, 1.0, h - 2.0]] {
        draws.push(Draw::Fill { rect: edge, colour });
    }
}

fn text(draws: &mut Vec<Draw>, text: String, at: [f32; 2], colour: u32) {
    if !text.is_empty() {
        draws.push(Draw::Text { text, at, colour, shadow: false });
    }
}

/// A node shape drawn `size` wide at `at`, from the node sheet; a plain
/// square without it.
fn shape_draw(shape: Shape, at: [f32; 2], size: f32) -> Draw {
    let dst = [at[0], at[1], size, size];
    let fallback = vec![Draw::Fill { rect: [at[0] + 2.0, at[1] + 2.0, size - 4.0, size - 4.0], colour: 0xFF8C_8CA0 }];
    Draw::Sprite { name: NODE_SPRITE.to_owned(), src: node_cell(shape, QuestState::Available), dst, fallback }
}

/// Buttons laid out in rows of `width` from `x`: each one's rect.
fn chip_rows(labels: &[String], x: f32, y: f32, width: f32, measure: &dyn Fn(&str) -> f32) -> Vec<[f32; 4]> {
    let mut rects = Vec::new();
    let (mut cx, mut cy) = (x, y);
    for label in labels {
        let w = (measure(label) + 8.0).min(width);
        if cx > x && cx + w > x + width {
            cx = x;
            cy += ROW;
        }
        rects.push([cx, cy, w, ROW - 2.0]);
        cx += w + 2.0;
    }
    rects
}

/// How tall control `c` is in a panel.
fn control_h(c: &Control, field_w: f32, measure: &dyn Fn(&str) -> f32) -> f32 {
    match c {
        Control::Segments { options, .. } => {
            let rects = chip_rows(options, 0.0, 0.0, field_w, measure);
            rects.last().map_or(ROW, |r| r[1] + ROW)
        }
        Control::Item { .. } | Control::Choose { .. } => 20.0,
        Control::Shapes { .. } => 20.0,
        // Its label, its rows, "add", and the steps when it has chances.
        Control::Rows { rows, .. } => (rows.len() + if rows.iter().filter(|r| r.2.is_some()).count() > 1 { 3 } else { 2 }) as f32 * ROW,
        Control::Danger { .. } => ROW + 4.0,
        _ => ROW,
    }
}

/// Where a panel `h` tall goes: beside `anchor` (the quest window) inside
/// `area`, on its right when there is room, else on its left, else over it.
fn panel_place(anchor: [f32; 4], area: [f32; 4], h: f32) -> [f32; 4] {
    let w = PANEL_W.min(area[2] - 2.0 * PAD);
    let right = anchor[0] + anchor[2] + GAP;
    let x = if right + w <= area[0] + area[2] - PAD {
        right
    } else if anchor[0] - GAP - w >= area[0] + PAD {
        anchor[0] - GAP - w
    } else {
        area[0] + area[2] - PAD - w
    };
    let h = h.min(area[3] - 2.0 * PAD);
    let y = anchor[1].min(area[1] + area[3] - PAD - h).max(area[1] + PAD);
    [x, y, w, h]
}

/// A panel titled `title` with `controls`, beside `anchor` in `area`: its
/// rect, what to draw, where clicks land (later on top). `clock` blinks
/// the cursor of the text control being typed in.
#[allow(clippy::too_many_arguments)]
pub fn panel(
    title: &str,
    controls: &[Control],
    anchor: [f32; 4],
    area: [f32; 4],
    typing: Typing<'_>,
    mouse: [f32; 2],
    clock: u32,
    measure: &dyn Fn(&str) -> f32,
) -> ([f32; 4], Vec<Draw>, Vec<([f32; 4], PanelAction)>) {
    let field_w = PANEL_W - 2.0 * PAD - LABEL_W;
    let content: f32 = controls.iter().map(|c| control_h(c, field_w, measure) + 2.0).sum();
    let rect @ [x, y, w, h] = panel_place(anchor, area, BAR + PAD + content + PAD);
    let mut draws = Vec::new();
    let mut zones = vec![(rect, PanelAction::Stay)];
    fill(&mut draws, rect, BACK);
    frame(&mut draws, rect, EDGE);
    fill(&mut draws, [x + 1.0, y + 1.0, w - 2.0, BAR - 1.0], BAR_BACK);
    text(&mut draws, fit_text(title, w - 2.0 * PAD - 10.0, measure), [x + PAD, y + 3.0], INK);
    let close = [x + w - 11.0, y + 2.0, 9.0, 9.0];
    text(&mut draws, "x".to_owned(), [close[0] + 2.0, close[1]], if canvas::inside(close, mouse) { INK } else { DIM_INK });
    zones.push((close, PanelAction::Close));

    let field_x = x + PAD + LABEL_W;
    let field_w = (x + w - PAD - field_x).max(0.0);
    let mut row_y = y + BAR + PAD;
    for c in controls {
        let ch = control_h(c, field_w, measure);
        if row_y + ch > y + h {
            break;
        }
        let label = match c {
            Control::Text { label, .. }
            | Control::Number { label, .. }
            | Control::Toggle { label, .. }
            | Control::Segments { label, .. }
            | Control::Shapes { label, .. }
            | Control::Item { label, .. }
            | Control::Rows { label, .. }
            | Control::Choose { label, .. } => *label,
            Control::Danger { .. } => "",
        };
        let label_y = row_y + if ch >= 20.0 && !matches!(c, Control::Segments { .. }) { 6.0 } else { 3.0 };
        text(&mut draws, fit_text(label, LABEL_W - 2.0, measure), [x + PAD, label_y], LABEL_INK);
        match c {
            Control::Text { key, value, hint, .. } => {
                let field = [field_x, row_y, field_w, ROW - 1.0];
                let typed = typing.filter(|(k, _)| k == key).map(|(_, f)| f);
                fill(&mut draws, field, SLOT);
                frame(&mut draws, field, if typed.is_some() { FOCUS_EDGE } else if canvas::inside(field, mouse) { SLOT_EDGE } else { EDGE });
                let at = [field[0] + 3.0, field[1] + 3.0];
                match typed {
                    Some(f) => {
                        let (shown, _, cursor) = f.shown(0, field[2] - 6.0, measure);
                        text(&mut draws, shown, at, INK);
                        if let Some(cx) = cursor.filter(|_| (clock / 10) % 2 == 0) {
                            fill(&mut draws, [at[0] + cx, at[1] - 1.0, 1.0, 9.0], CURSOR);
                        }
                    }
                    None if value.is_empty() => text(&mut draws, fit_text(hint, field[2] - 6.0, measure), at, DIM_INK),
                    None => text(&mut draws, fit_text(value, field[2] - 6.0, measure), at, INK),
                }
                zones.push((field, PanelAction::Type(key)));
            }
            Control::Number { key, value, zero, min, .. } => {
                let minus = [field_x, row_y, 12.0, ROW - 2.0];
                let plus = [field_x + field_w - 12.0, row_y, 12.0, ROW - 2.0];
                for (button, sign, by) in [(minus, "-", -1.0), (plus, "+", 1.0)] {
                    fill(&mut draws, button, if canvas::inside(button, mouse) { CHIP_HOVER } else { CHIP });
                    text(&mut draws, sign.to_owned(), [button[0] + 4.0, button[1] + 2.0], INK);
                    zones.push((button, PanelAction::Step(key, by)));
                }
                let lowest = *value <= *min;
                let shown = match zero {
                    Some(name) if lowest => (*name).to_owned(),
                    _ => number_text(*value),
                };
                let middle = [minus[0] + 14.0, row_y, field_w - 28.0, ROW - 2.0];
                fill(&mut draws, middle, SLOT);
                match typing.filter(|(k, _)| k == key).map(|(_, f)| f) {
                    Some(f) => {
                        frame(&mut draws, middle, FOCUS_EDGE);
                        let (typed, _, cursor) = f.shown(0, middle[2] - 6.0, measure);
                        let at = [middle[0] + 3.0, row_y + 2.0];
                        text(&mut draws, typed, at, INK);
                        if let Some(cx) = cursor.filter(|_| (clock / 10) % 2 == 0) {
                            fill(&mut draws, [at[0] + cx, at[1] - 1.0, 1.0, 9.0], CURSOR);
                        }
                    }
                    None => {
                        if canvas::inside(middle, mouse) {
                            frame(&mut draws, middle, SLOT_EDGE);
                        }
                        let tw = measure(&shown);
                        let ink = if zero.is_some() && lowest { DIM_INK } else { INK };
                        text(&mut draws, fit_text(&shown, middle[2] - 4.0, measure), [middle[0] + ((middle[2] - tw) / 2.0).max(2.0), row_y + 2.0], ink);
                    }
                }
                // A click types the number; the wheel over it steps it (the book reads this zone).
                zones.push((middle, PanelAction::Step(key, 0.0)));
            }
            Control::Toggle { key, on, .. } => {
                let switch = [field_x, row_y + 2.0, 20.0, 9.0];
                fill(&mut draws, switch, if *on { SWITCH_ON } else { SWITCH_OFF });
                let knob_x = if *on { switch[0] + 12.0 } else { switch[0] + 1.0 };
                fill(&mut draws, [knob_x, switch[1] + 1.0, 7.0, 7.0], KNOB);
                text(&mut draws, (if *on { "On" } else { "Off" }).to_owned(), [switch[0] + 24.0, row_y + 3.0], if *on { INK } else { DIM_INK });
                zones.push(([field_x, row_y, field_w, ROW], PanelAction::Toggle(key)));
            }
            Control::Segments { key, options, at, .. } => {
                for (i, chip) in chip_rows(options, field_x, row_y, field_w, measure).into_iter().enumerate() {
                    let chosen = *at == Some(i);
                    fill(&mut draws, chip, if chosen { CHIP_ON } else if canvas::inside(chip, mouse) { CHIP_HOVER } else { CHIP });
                    if chosen {
                        frame(&mut draws, chip, CHIP_ON_EDGE);
                    }
                    let label = fit_text(&options[i], chip[2] - 6.0, measure);
                    text(&mut draws, label, [chip[0] + 4.0, chip[1] + 2.5], if chosen { INK } else { LABEL_INK });
                    zones.push((chip, PanelAction::Segment(key, i)));
                }
            }
            Control::Shapes { key, at, auto, .. } => {
                let mut bx = field_x;
                let mut choices: Vec<Option<Shape>> = Vec::new();
                if auto.is_some() {
                    choices.push(None);
                }
                choices.extend(SHAPES.iter().map(|s| Some(*s)));
                for choice in choices {
                    let button = if choice.is_none() { [bx, row_y, measure("Default") + 8.0, 19.0] } else { [bx, row_y, 18.0, 19.0] };
                    let chosen = *at == choice;
                    fill(&mut draws, button, if chosen { CHIP_ON } else if canvas::inside(button, mouse) { CHIP_HOVER } else { CHIP });
                    if chosen {
                        frame(&mut draws, button, CHIP_ON_EDGE);
                    }
                    match choice {
                        // "Default": the chapter's (or the pack's) shape.
                        None => text(&mut draws, "Default".to_owned(), [button[0] + 4.0, button[1] + 6.0], if chosen { INK } else { LABEL_INK }),
                        Some(shape) => draws.push(shape_draw(shape, [button[0] + 2.0, button[1] + 2.5], 14.0)),
                    }
                    zones.push((button, PanelAction::Shape(key, choice)));
                    bx += button[2] + 2.0;
                }
            }
            Control::Item { key, item, mob, .. } => {
                let slot = [field_x, row_y, 19.0, 19.0];
                fill(&mut draws, slot, SLOT);
                frame(&mut draws, slot, if canvas::inside(slot, mouse) { FOCUS_EDGE } else { SLOT_EDGE });
                if !item.is_empty() {
                    let icon = if *mob { format!("{item}_spawn_egg") } else { item.clone() };
                    draws.push(Draw::Item { item: icon, at: [slot[0] + 1.5, slot[1] + 1.5], size: 16.0 });
                }
                let name = if item.is_empty() { "Click to choose".to_owned() } else { pretty(item.trim_start_matches('#')) };
                text(&mut draws, fit_text(&name, field_w - 24.0, measure), [slot[0] + 23.0, row_y + 6.0], if item.is_empty() { DIM_INK } else { INK });
                zones.push(([field_x, row_y, field_w, 19.0], PanelAction::Pick(key)));
            }
            Control::Choose { key, icon, name, .. } => {
                let slot = [field_x, row_y, 19.0, 19.0];
                fill(&mut draws, slot, SLOT);
                frame(&mut draws, slot, if canvas::inside(slot, mouse) { FOCUS_EDGE } else { SLOT_EDGE });
                draws.push(Draw::Item { item: icon.clone(), at: [slot[0] + 1.5, slot[1] + 1.5], size: 16.0 });
                text(&mut draws, fit_text(name, field_w - 24.0, measure), [slot[0] + 23.0, row_y + 6.0], INK);
                zones.push(([field_x, row_y, field_w, 19.0], PanelAction::Pick(key)));
            }
            Control::Rows { key, rows, add, step, .. } => {
                // Under the label, across the whole panel.
                let (left, width) = (x + PAD, w - 2.0 * PAD);
                let top = row_y + ROW;
                for (i, (icon, line, chance)) in rows.iter().enumerate() {
                    let row = [left, top + i as f32 * ROW, width, ROW - 1.0];
                    fill(&mut draws, row, if canvas::inside(row, mouse) { CHIP_HOVER } else { SLOT });
                    zones.push((row, PanelAction::Row(key, i)));
                    draws.push(Draw::Item { item: icon.clone(), at: [row[0] + 1.0, row[1] + 0.5], size: 12.0 });
                    // The chance on the right, always shown: - value +.
                    let mut name_w = width - 18.0;
                    if let Some(percent) = chance {
                        let plus = [row[0] + row[2] - 11.0, row[1], 11.0, row[3]];
                        let value = [plus[0] - 31.0, row[1], 30.0, row[3]];
                        let minus = [value[0] - 12.0, row[1], 11.0, row[3]];
                        name_w = minus[0] - row[0] - 18.0;
                        for (button, sign, by) in [(minus, "-", -1.0), (plus, "+", 1.0)] {
                            fill(&mut draws, button, if canvas::inside(button, mouse) { CHIP_HOVER } else { CHIP });
                            text(&mut draws, sign.to_owned(), [button[0] + 3.0, button[1] + 2.5], INK);
                            zones.push((button, PanelAction::RowStep(key, i, by)));
                        }
                        fill(&mut draws, value, SLOT);
                        let typed_key = format!("{key}#{i}");
                        match typing.filter(|(k, _)| *k == typed_key).map(|(_, f)| f) {
                            Some(f) => {
                                frame(&mut draws, value, FOCUS_EDGE);
                                let (shown, _, cursor) = f.shown(0, value[2] - 4.0, measure);
                                let at = [value[0] + 2.0, value[1] + 3.0];
                                text(&mut draws, shown, at, INK);
                                if let Some(cx) = cursor.filter(|_| (clock / 10) % 2 == 0) {
                                    fill(&mut draws, [at[0] + cx, at[1] - 1.0, 1.0, 9.0], CURSOR);
                                }
                            }
                            None => {
                                if canvas::inside(value, mouse) {
                                    frame(&mut draws, value, SLOT_EDGE);
                                }
                                let shown = chance_text(*percent);
                                let tw = measure(&shown);
                                text(&mut draws, shown, [value[0] + ((value[2] - tw) / 2.0).max(1.0), value[1] + 3.0], CHANCE_INK);
                            }
                        }
                        // A click types it; the wheel over it moves it by a step.
                        zones.push((value, PanelAction::RowStep(key, i, 0.0)));
                    }
                    text(&mut draws, fit_text(line, name_w.max(0.0), measure), [row[0] + 15.0, row[1] + 3.0], INK);
                }
                let foot = top + rows.len() as f32 * ROW;
                let has_chances = rows.iter().filter(|r| r.2.is_some()).count() > 1;
                let even_w = if has_chances { measure("Even") + 8.0 } else { 0.0 };
                let more = [left, foot, width - if has_chances { even_w + 2.0 } else { 0.0 }, ROW - 1.0];
                fill(&mut draws, more, if canvas::inside(more, mouse) { CHIP_HOVER } else { CHIP });
                text(&mut draws, (*add).to_owned(), [more[0] + 4.0, more[1] + 3.0], ADD_INK);
                zones.push((more, PanelAction::AddRow(key)));
                if has_chances {
                    let even = [left + width - even_w, foot, even_w, ROW - 1.0];
                    fill(&mut draws, even, if canvas::inside(even, mouse) { CHIP_HOVER } else { CHIP });
                    text(&mut draws, "Even".to_owned(), [even[0] + 4.0, even[1] + 3.0], LABEL_INK);
                    zones.push((even, PanelAction::Even(key)));
                    // What - / + and the wheel move a chance by.
                    let line = foot + ROW;
                    text(&mut draws, "Step".to_owned(), [left + 1.0, line + 3.0], LABEL_INK);
                    let mut cx = left + measure("Step") + 6.0;
                    for value in CHANCE_STEPS {
                        let label = number_text(value);
                        let chip = [cx, line, measure(&label) + 8.0, ROW - 1.0];
                        let chosen = (value - *step).abs() < 0.01;
                        fill(&mut draws, chip, if chosen { CHIP_ON } else if canvas::inside(chip, mouse) { CHIP_HOVER } else { CHIP });
                        if chosen {
                            frame(&mut draws, chip, CHIP_ON_EDGE);
                        }
                        text(&mut draws, label, [chip[0] + 4.0, chip[1] + 3.0], if chosen { INK } else { LABEL_INK });
                        zones.push((chip, PanelAction::ChanceStep(key, value)));
                        cx += chip[2] + 2.0;
                    }
                }
            }
            Control::Danger { key, label } => {
                let button = [x + PAD, row_y + 2.0, w - 2.0 * PAD, ROW];
                fill(&mut draws, button, if canvas::inside(button, mouse) { DANGER_HOVER } else { DANGER });
                let tw = measure(label);
                text(&mut draws, (*label).to_owned(), [button[0] + (button[2] - tw) / 2.0, button[1] + 3.0], INK);
                zones.push((button, PanelAction::Press(key)));
            }
        }
        row_y += ch + 2.0;
    }
    (rect, draws, zones)
}

// ---------------------------------------------------------------- the picker

/// The grid to choose an item, or a mob, from.
#[derive(Clone, Debug, PartialEq)]
pub struct Picker {
    pub query: TextField,
    /// Rows scrolled past.
    pub scroll: usize,
    /// Mobs (by their spawn eggs) instead of items.
    pub mobs: bool,
    /// Whether an item tag (`#minecraft:logs`) may be chosen.
    pub tags: bool,
    /// Advancements (id, icon item) to choose from, in place of items.
    pub advancements: Option<Vec<(String, String)>>,
}

impl Picker {
    pub fn new(mobs: bool, tags: bool) -> Self {
        Self { query: TextField::new("", 60), scroll: 0, mobs, tags, advancements: None }
    }

    /// A picker of the server's advancements.
    pub fn advancements(list: Vec<(String, String)>) -> Self {
        Self { advancements: Some(list), ..Self::new(false, false) }
    }

    /// What can be chosen, as (value, icon, name), filtered by the search
    /// line and sorted by name; a tag typed comes first. Each word must be
    /// found in the id or the name; `@word` keeps what a mod adds, by the
    /// namespace of its id (`@applied` for `appliedenergistics:...`).
    pub fn entries(&self, names: &HashMap<String, String>) -> Vec<(String, String, String)> {
        let typed = self.query.value.trim().to_lowercase();
        let (mods, words): (Vec<&str>, Vec<&str>) = typed.split_whitespace().filter(|w| *w != "@").partition(|w| w.starts_with('@'));
        let mods: Vec<&str> = mods.iter().map(|m| &m[1..]).collect();
        let matches = |value: &str, name: &str| {
            let namespace = value.trim_start_matches('#').split_once(':').map_or("minecraft", |(ns, _)| ns);
            mods.iter().all(|m| namespace.contains(m))
                && words.iter().all(|w| value.contains(w) || name.to_lowercase().contains(w))
        };
        let listed: Vec<(String, String, String)> = match &self.advancements {
            Some(list) => list.iter().map(|(id, icon)| (id.clone(), icon.clone(), advancement_name(id))).collect(),
            None => Vec::new(),
        };
        let mut found: Vec<(String, String, String)> = names
            .iter()
            .filter(|_| self.advancements.is_none())
            .filter_map(|(id, name)| {
                if self.mobs {
                    let entity = id.strip_suffix("_spawn_egg")?;
                    let mob = name.strip_suffix(" Spawn Egg").unwrap_or(name);
                    Some((entity.to_owned(), id.clone(), mob.to_owned()))
                } else {
                    Some((id.clone(), id.clone(), name.clone()))
                }
            })
            .chain(listed)
            .filter(|(value, _, name)| matches(value, name))
            .collect();
        found.sort_by(|a, b| a.2.cmp(&b.2).then_with(|| a.0.cmp(&b.0)));
        if self.tags
            && let Some(tag) = words.iter().find_map(|w| w.strip_prefix('#')).and_then(resource_id)
        {
            found.insert(0, (format!("#{tag}"), format!("#{tag}"), format!("Any item of #{tag}")));
        }
        found
    }

    /// How many rows the entries take beyond those shown.
    pub fn max_scroll(&self, names: &HashMap<String, String>) -> usize {
        self.entries(names).len().div_ceil(COLUMNS).saturating_sub(ROWS)
    }

    /// The picker over `area`: its rect, what to draw, where clicks land,
    /// and the tooltip of the entry under the pointer. `searching` says
    /// the search line has the keyboard.
    #[allow(clippy::too_many_arguments)]
    pub fn layout(
        &self,
        area: [f32; 4],
        names: &HashMap<String, String>,
        searching: bool,
        mouse: [f32; 2],
        clock: u32,
        measure: &dyn Fn(&str) -> f32,
    ) -> ([f32; 4], Vec<Draw>, Vec<([f32; 4], PickAction)>, Option<Vec<String>>) {
        let w = COLUMNS as f32 * CELL + 2.0 * PAD;
        let h = BAR + ROW + PAD + ROWS as f32 * CELL + 2.0 * PAD;
        let [cx, cy] = canvas::centre(area);
        let rect @ [x, y, _, _] = [cx - w / 2.0, cy - h / 2.0, w, h];
        let mut draws = Vec::new();
        let mut zones = vec![(area, PickAction::Close), (rect, PickAction::Stay)];
        fill(&mut draws, rect, 0xFF1C_2230);
        frame(&mut draws, rect, EDGE);
        fill(&mut draws, [x + 1.0, y + 1.0, w - 2.0, BAR - 1.0], BAR_BACK);
        let title = if self.advancements.is_some() {
            "Choose an advancement"
        } else if self.mobs {
            "Choose a mob"
        } else {
            "Choose an item"
        };
        text(&mut draws, title.to_owned(), [x + PAD, y + 3.0], INK);
        let close = [x + w - 11.0, y + 2.0, 9.0, 9.0];
        text(&mut draws, "x".to_owned(), [close[0] + 2.0, close[1]], if canvas::inside(close, mouse) { INK } else { DIM_INK });
        zones.push((close, PickAction::Close));

        let search = [x + PAD, y + BAR + 2.0, w - 2.0 * PAD, ROW - 1.0];
        fill(&mut draws, search, SLOT);
        frame(&mut draws, search, if searching { FOCUS_EDGE } else { EDGE });
        let at = [search[0] + 3.0, search[1] + 3.0];
        if self.query.value.is_empty() && !searching {
            let hint = if self.tags { "Search (@mod, #tag)" } else { "Search (@mod)" };
            text(&mut draws, hint.to_owned(), at, DIM_INK);
        } else {
            let (shown, _, cursor) = self.query.shown(0, search[2] - 6.0, measure);
            text(&mut draws, shown, at, INK);
            if let Some(c) = cursor.filter(|_| searching && (clock / 10) % 2 == 0) {
                fill(&mut draws, [at[0] + c, at[1] - 1.0, 1.0, 9.0], CURSOR);
            }
        }
        zones.push((search, PickAction::Search));

        let entries = self.entries(names);
        let top = search[1] + ROW + PAD;
        let mut tip = None;
        for (i, (value, icon, name)) in entries.iter().skip(self.scroll * COLUMNS).take(COLUMNS * ROWS).enumerate() {
            let cell = [x + PAD + (i % COLUMNS) as f32 * CELL, top + (i / COLUMNS) as f32 * CELL, CELL, CELL];
            if canvas::inside(cell, mouse) {
                fill(&mut draws, cell, CHIP_HOVER);
                tip = Some(vec![name.clone(), value.clone()]);
            }
            draws.push(Draw::Item { item: icon.clone(), at: [cell[0] + 1.0, cell[1] + 1.0], size: 16.0 });
            zones.push((cell, PickAction::Choose(value.clone())));
        }
        if entries.is_empty() {
            text(&mut draws, "Nothing matches.".to_owned(), [x + PAD, top + 4.0], DIM_INK);
        }
        (rect, draws, zones, tip)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::hq_model::Quest;

    fn measure(text: &str) -> f32 {
        text.chars().count() as f32 * 6.0
    }

    #[test]
    fn quest_settings_change_one_thing_a_click() {
        let mut info = QuestInfo::of(&Quest { id: "q".into(), title: "Q".into(), ..Quest::default() });
        assert!(quest_change(&mut info, Shape::Square, &PanelAction::Shape("shape", Some(Shape::Diamond))));
        assert_eq!(info.shape, Some(Shape::Diamond));
        // Size: from auto (0) one step up is a quarter, and back to auto.
        quest_change(&mut info, Shape::Square, &PanelAction::Step("size", 1.0));
        assert_eq!(info.size, Some(0.25));
        quest_change(&mut info, Shape::Square, &PanelAction::Step("size", -1.0));
        assert_eq!(info.size, None);
        quest_change(&mut info, Shape::Square, &PanelAction::Segment("visibility", 3));
        assert_eq!(info.visibility, Visibility::Secret);
        quest_change(&mut info, Shape::Square, &PanelAction::Toggle("repeatable"));
        quest_change(&mut info, Shape::Square, &PanelAction::Step("cooldown", 1.0));
        assert!(info.repeatable && info.repeat_cooldown == 60);
        assert!(!quest_change(&mut info, Shape::Square, &PanelAction::Step("min", -1.0)));
    }

    #[test]
    fn task_panels_follow_the_type() {
        let mut task = Task { id: "t".into(), title: None, kind: TaskKind::Kill { entity: "minecraft:zombie".into(), count: 1 } };
        assert!(task_change(&mut task, &PanelAction::Step("count", 1.0)));
        task_pick(&mut task, "entity", "minecraft:skeleton");
        assert_eq!(task.kind, TaskKind::Kill { entity: "minecraft:skeleton".into(), count: 2 });
        task_text(&mut task, "title", " Bones ").unwrap();
        assert_eq!(task.title.as_deref(), Some("Bones"));

        let mut place = Task { id: "p".into(), title: None, kind: TaskKind::Location { dimension: None, from: [0, 0, 0], to: [1, 1, 1] } };
        task_change(&mut place, &PanelAction::Segment("dimension", 2));
        assert!(matches!(&place.kind, TaskKind::Location { dimension: Some(d), .. } if d == "minecraft:the_nether"));
        assert!(task_text(&mut place, "from", "1 2").is_err());
        task_text(&mut place, "from", "-5 64 10").unwrap();
        assert!(matches!(place.kind, TaskKind::Location { from: [-5, 64, 10], .. }));

        let mut item = Task { id: "i".into(), title: None, kind: TaskKind::Item { item: ItemMatch::Id("minecraft:dirt".into()), count: 1, consume: false } };
        task_pick(&mut item, "item", "#minecraft:logs");
        task_change(&mut item, &PanelAction::Toggle("consume"));
        assert_eq!(item.kind, TaskKind::Item { item: ItemMatch::Tag("minecraft:logs".into()), count: 1, consume: true });
        // Counts never go below one.
        assert!(!task_change(&mut item, &PanelAction::Step("count", -1.0)));
    }

    #[test]
    fn rewards_and_entries_change() {
        let (mut auto, mut team) = (AutoClaim::Default, false);
        let mut kind = RewardKind::Item { item: "minecraft:bread".into(), count: 2, max_count: None };
        // "Up to" starts at the count: one step up is a range 2-3.
        reward_change(&mut auto, &mut team, &mut kind, false, &PanelAction::Step("max", 1.0));
        assert_eq!(kind, RewardKind::Item { item: "minecraft:bread".into(), count: 2, max_count: Some(3) });
        // A count that reaches the top ends the range.
        reward_change(&mut auto, &mut team, &mut kind, false, &PanelAction::Set("count", 5.0));
        assert_eq!(kind, RewardKind::Item { item: "minecraft:bread".into(), count: 5, max_count: None });
        // "Given at once": with the pack's setting off, a click turns it on.
        reward_change(&mut auto, &mut team, &mut kind, false, &PanelAction::Toggle("auto"));
        reward_change(&mut auto, &mut team, &mut kind, false, &PanelAction::Toggle("team"));
        assert_eq!((auto, team), (AutoClaim::Enabled, true));
        reward_change(&mut auto, &mut team, &mut kind, true, &PanelAction::Toggle("auto"));
        assert_eq!(auto, AutoClaim::Disabled);
        // Experience: points and levels are one reward with a unit.
        let mut xp = RewardKind::Xp { points: 30 };
        reward_change(&mut auto, &mut team, &mut xp, false, &PanelAction::Segment("unit", 1));
        assert_eq!(xp, RewardKind::XpLevels { levels: 1 });
        reward_change(&mut auto, &mut team, &mut xp, false, &PanelAction::Set("amount", 3.0));
        reward_change(&mut auto, &mut team, &mut xp, false, &PanelAction::Segment("unit", 0));
        assert_eq!(xp, RewardKind::Xp { points: 50 });
        let draw = RewardKind::Random { table: "a".into() };
        assert!(matches!(&kind_controls(&draw)[0], Control::Choose { key: "table", value, .. } if value == "a"));
        let mut title = None;
        let mut command = RewardKind::Command { command: "say hi".into() };
        assert!(reward_text(&mut title, &mut command, "command", "  ").is_err());
        reward_text(&mut title, &mut command, "command", "/give {player} diamond").unwrap();
        assert_eq!(command, RewardKind::Command { command: "give {player} diamond".into() });
        let mut weight = 1;
        entry_change(&mut weight, &mut kind, &PanelAction::Step("weight", 1.0));
        assert_eq!(weight, 2);
    }

    #[test]
    fn chapters_and_settings_change() {
        let groups = vec![Group { id: "main".into(), title: "Main".into() }];
        let mut info = ChapterInfo { title: "C".into(), subtitle: None, icon: None, group: None, hidden: false, progression: None, default_shape: None, default_size: None };
        chapter_change(&mut info, Shape::Square, &groups, &PanelAction::Segment("group", 1));
        chapter_change(&mut info, Shape::Square, &groups, &PanelAction::Segment("progression", 2));
        assert_eq!((info.group.as_deref(), info.progression), (Some("main"), Some(Progression::Flexible)));
        chapter_text(&mut info, "subtitle", "  ");
        assert_eq!(info.subtitle, None);
        let mut settings = PackSettings { progression: Progression::Linear, default_shape: Shape::Square, default_size: 1.0, auto_claim_rewards: false, detection_delay: 20 };
        settings_change(&mut settings, &PanelAction::Step("delay", 1.0));
        settings_change(&mut settings, &PanelAction::Shape("shape", Some(Shape::Circle)));
        assert_eq!((settings.detection_delay, settings.default_shape), (25, Shape::Circle));
        assert!(!settings_change(&mut settings, &PanelAction::Shape("shape", None)));
    }

    #[test]
    fn the_panel_sits_beside_the_quest_window_and_every_control_has_a_zone() {
        let info = QuestInfo::of(&Quest::default());
        let controls = quest_controls(&info, Shape::Square);
        let area = [0.0, 0.0, 600.0, 300.0];
        let anchor = [100.0, 50.0, 200.0, 120.0];
        let (rect, _, zones) = panel("Quest settings", &controls, anchor, area, None, [-1.0, -1.0], 0, &measure);
        assert_eq!(rect[0], 300.0 + GAP);
        for action in [PanelAction::Shape("shape", None), PanelAction::Step("size", 1.0), PanelAction::Segment("needs", 3), PanelAction::Toggle("optional"), PanelAction::Step("cooldown", 0.0)] {
            assert!(zones.iter().any(|(_, a)| *a == action), "{action:?}");
        }
        // No room on the right: on the left.
        let (rect, ..) = panel("x", &controls, [380.0, 50.0, 200.0, 120.0], area, None, [-1.0, -1.0], 0, &measure);
        assert_eq!(rect[0], 380.0 - GAP - PANEL_W);
    }

    #[test]
    fn the_picker_finds_items_mobs_and_tags() {
        let names: HashMap<String, String> = [
            ("minecraft:oak_log", "Oak Log"),
            ("minecraft:zombie_spawn_egg", "Zombie Spawn Egg"),
            ("minecraft:stone", "Stone"),
            ("appliedenergistics:quartz_glass", "Quartz Glass"),
            ("appliedenergistics:sky_stone", "Sky Stone"),
        ]
        .into_iter()
        .map(|(a, b)| (a.to_owned(), b.to_owned()))
        .collect();
        let values = |picker: &Picker| picker.entries(&names).into_iter().map(|e| e.0).collect::<Vec<_>>();
        let mut picker = Picker::new(false, true);
        assert_eq!(picker.entries(&names).len(), 5);
        // `@` keeps one mod's items, other words narrow them down.
        "@applied".chars().for_each(|c| picker.query.type_char(c));
        assert_eq!(values(&picker), ["appliedenergistics:quartz_glass", "appliedenergistics:sky_stone"]);
        " stone".chars().for_each(|c| picker.query.type_char(c));
        assert_eq!(values(&picker), ["appliedenergistics:sky_stone"]);
        picker.query = TextField::new("@minecraft stone", 60);
        assert_eq!(values(&picker), ["minecraft:stone"]);
        picker.query = TextField::new("", 60);
        "#logs".chars().for_each(|c| picker.query.type_char(c));
        let found = picker.entries(&names);
        assert_eq!(found[0].0, "#minecraft:logs");
        let mobs = Picker::new(true, false);
        assert_eq!(mobs.entries(&names), vec![("minecraft:zombie".to_owned(), "minecraft:zombie_spawn_egg".to_owned(), "Zombie".to_owned())]);
        let (_, _, zones, _) = mobs.layout([0.0, 0.0, 400.0, 300.0], &names, false, [-1.0, -1.0], 0, &measure);
        assert!(zones.iter().any(|(_, a)| *a == PickAction::Choose("minecraft:zombie".into())));
    }

    #[test]
    fn advancements_are_chosen_from_a_list() {
        let list = vec![
            ("minecraft:story/mine_stone".to_owned(), "minecraft:wooden_pickaxe".to_owned()),
            ("minecraft:nether/root".to_owned(), "minecraft:red_nether_bricks".to_owned()),
            ("mymod:big/thing".to_owned(), "minecraft:stone".to_owned()),
        ];
        assert_eq!(advancement_name("minecraft:story/mine_stone"), "Story: Mine stone");
        assert_eq!(advancement_name("mymod:big/thing"), "Big: Thing (mymod)");
        let names: HashMap<String, String> = [("minecraft:stone".to_owned(), "Stone".to_owned())].into_iter().collect();
        let mut picker = Picker::advancements(list);
        let found: Vec<String> = picker.entries(&names).into_iter().map(|e| e.0).collect();
        assert_eq!(found, ["mymod:big/thing", "minecraft:nether/root", "minecraft:story/mine_stone"]);
        "stone".chars().for_each(|c| picker.query.type_char(c));
        assert_eq!(picker.entries(&names)[0].1, "minecraft:wooden_pickaxe");
        let mut task = Task { id: "t".into(), title: None, kind: TaskKind::Advancement { advancement: "minecraft:story/root".into() } };
        assert!(matches!(&task_controls(&task)[0], Control::Choose { name, .. } if name == "Story: Root"));
        task_pick(&mut task, "advancement", "minecraft:nether/root");
        assert_eq!(task.kind, TaskKind::Advancement { advancement: "minecraft:nether/root".into() });
    }

    #[test]
    fn numbers_read_well() {
        assert_eq!(number_text(2.0), "2");
        assert_eq!(number_text(1.25), "1.25");
        assert_eq!(number_text(0.5), "0.5");
        assert_eq!(stepped(0.75, 0.25), 1.0);
    }
}
