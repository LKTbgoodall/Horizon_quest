// The quest book as the player sees it: where everything goes on screen and
// what a click, a drag or the wheel does. `BookView::scene` turns the book,
// the progress and the view into a list of things to draw (fills, quads,
// text, item icons) and of zones to click; the client glue only carries the
// draws out with the game's helpers and sends the requests. Pure: no game
// types; the game's text width comes in as a function.
//
// Colours are `0xAARRGGBB` for fills and quads, `0xRRGGBB` for text. There
// is no 2D clip. The canvas is drawn first and the bands round it (chapter
// list, panel, header, footer) are opaque and drawn after, so a node's box
// or a line that sticks out of the canvas ends up covered. Text and item
// icons are only drawn where they fit whole: icons may not be drawn in the
// same pass as the fills.
use super::hq_canvas::{self as canvas, View};
use super::hq_logic::{Book, QuestState, Status};
use super::hq_model::{ItemMatch, Progress, Quest, Reward, RewardKind, Shape, TableEntry, Task, TaskKind};
use super::hq_net::ToServer;
use std::collections::VecDeque;

#[allow(dead_code, reason = "phase 2 glue calls it step by step")]
const SHADE: u32 = 0xD010_1018;
#[allow(dead_code, reason = "phase 2 glue calls it step by step")]
const BAND: u32 = 0xFF20_2030;
#[allow(dead_code, reason = "phase 2 glue calls it step by step")]
const SIDEBAR: u32 = 0xFF18_1822;
#[allow(dead_code, reason = "phase 2 glue calls it step by step")]
const PANEL: u32 = 0xFF1C_1C28;
#[allow(dead_code, reason = "phase 2 glue calls it step by step")]
const ROW_CURRENT: u32 = 0xFF34_344A;
#[allow(dead_code, reason = "phase 2 glue calls it step by step")]
const ROW_HOVER: u32 = 0xFF2A_2A3A;
#[allow(dead_code, reason = "phase 2 glue calls it step by step")]
const BUTTON: u32 = 0xFF3A_5A8A;
#[allow(dead_code, reason = "phase 2 glue calls it step by step")]
const BUTTON_HOVER: u32 = 0xFF4A_74B0;
#[allow(dead_code, reason = "phase 2 glue calls it step by step")]
const TOOLTIP: u32 = 0xF010_0818;
#[allow(dead_code, reason = "phase 2 glue calls it step by step")]
const NODE_FILL: u32 = 0xFF26_2632;
#[allow(dead_code, reason = "phase 2 glue calls it step by step")]
const SELECTED: u32 = 0xFFFF_C040;
#[allow(dead_code, reason = "phase 2 glue calls it step by step")]
const LINE: u32 = 0xFF60_6070;
#[allow(dead_code, reason = "phase 2 glue calls it step by step")]
const LINE_DONE: u32 = 0xFF40_A050;
#[allow(dead_code, reason = "phase 2 glue calls it step by step")]
const BAR: u32 = 0xFF30_303C;
#[allow(dead_code, reason = "phase 2 glue calls it step by step")]
const INK: u32 = 0xFF_FFFF;
#[allow(dead_code, reason = "phase 2 glue calls it step by step")]
const DIM_INK: u32 = 0xA0_A0B0;
#[allow(dead_code, reason = "phase 2 glue calls it step by step")]
const GOLD_INK: u32 = 0xFF_C040;

/// A line of text's height, a chapter row's, and the space left round
/// things, in GUI pixels.
#[allow(dead_code, reason = "phase 2 glue calls it step by step")]
const TEXT: f32 = 10.0;
#[allow(dead_code, reason = "phase 2 glue calls it step by step")]
const ROW: f32 = 14.0;
#[allow(dead_code, reason = "phase 2 glue calls it step by step")]
const PAD: f32 = 4.0;
/// The panel's title bar.
#[allow(dead_code, reason = "phase 2 glue calls it step by step")]
const TITLE_BAR: f32 = 14.0;
/// The room left round a chapter fitted to the canvas.
#[allow(dead_code, reason = "phase 2 glue calls it step by step")]
const MARGIN: f32 = 16.0;
/// The most entries of a reward table the quest panel lists.
#[allow(dead_code, reason = "phase 3 glue calls it step by step")]
const TABLE_ROWS: usize = 12;
/// The icon shown for a quest that names none and whose first task has none.
#[allow(dead_code, reason = "phase 2 glue calls it step by step")]
const DEFAULT_ICON: &str = "minecraft:book";

/// One thing to draw. In order: later ones cover earlier ones.
#[derive(Clone, Debug, PartialEq)]
#[allow(dead_code, reason = "phase 2 glue calls it step by step")]
pub enum Draw {
    Fill { rect: [f32; 4], colour: u32 },
    Quad { corners: [[f32; 2]; 4], colour: u32 },
    Text { text: String, at: [f32; 2], colour: u32, shadow: bool },
    /// An item's icon, `size` pixels wide, its top-left corner at `at`.
    Item { item: String, at: [f32; 2], size: f32 },
}

/// What a click does. Nodes are `Book` node indexes; ids are full ids.
#[derive(Clone, Debug, PartialEq, Eq)]
#[allow(dead_code, reason = "phase 2 glue calls it step by step")]
pub enum Action {
    Chapter(usize),
    Quest(usize),
    /// Close the quest panel.
    Deselect,
    Check(String),
    Submit(String),
    Claim(String),
    /// Claim a choice reward, taking this entry of its table.
    Choose(String, u32),
    ClaimAll,
    /// Close the book.
    Close,
    /// Show this quest (full id) on the HUD, or stop showing it.
    Track(String),
    Untrack,
    /// Show the recipes that make this item (an id, or `#tag`).
    ShowRecipe(String),
    /// Turn the recipe view's page by this many.
    TurnRecipe(i32),
    CloseRecipe,
    /// A click on the recipe view itself: it stays.
    KeepRecipe,
}

/// What the glue does after a click.
#[derive(Clone, Debug, PartialEq, Eq)]
#[allow(dead_code, reason = "phase 2 glue calls it step by step")]
pub enum Reaction {
    Nothing,
    Send(ToServer),
    Close,
    /// The tracked quest changed: keep it in the player's settings.
    Tracked(Option<String>),
}

/// The book's parts on screen, as `[x, y, w, h]`.
#[derive(Clone, Copy, Debug, PartialEq)]
#[allow(dead_code, reason = "phase 2 glue calls it step by step")]
pub struct Regions {
    pub header: [f32; 4],
    pub sidebar: [f32; 4],
    pub canvas: [f32; 4],
    /// The quest panel, when a quest is selected.
    pub panel: Option<[f32; 4]>,
    pub footer: [f32; 4],
}

/// The parts of a book filling `rect`, with or without the quest panel.
#[allow(dead_code, reason = "phase 2 glue calls it step by step")]
pub fn regions(rect: [f32; 4], panel: bool) -> Regions {
    let [x, y, w, h] = rect;
    let body_y = y + canvas::HEADER;
    let body_h = (h - canvas::HEADER - canvas::FOOTER).max(0.0);
    let sidebar_w = (w * 0.25).clamp(90.0, 150.0).min(w.max(0.0));
    let panel_w = if panel { (w * 0.34).clamp(110.0, 200.0).min((w - sidebar_w).max(0.0)) } else { 0.0 };
    Regions {
        header: [x, y, w, canvas::HEADER],
        sidebar: [x, body_y, sidebar_w, body_h],
        canvas: [x + sidebar_w, body_y, (w - sidebar_w - panel_w).max(0.0), body_h],
        panel: panel.then_some([x + w - panel_w, body_y, panel_w, body_h]),
        footer: [x, y + h - canvas::FOOTER, w, canvas::FOOTER],
    }
}

/// One frame's worth of book: what to draw and where clicks land.
#[derive(Clone, Debug, PartialEq)]
#[allow(dead_code, reason = "phase 2 glue calls it step by step")]
pub struct Scene {
    pub regions: Regions,
    pub draws: Vec<Draw>,
    /// Later zones are on top.
    pub zones: Vec<([f32; 4], Action)>,
    /// How far the chapter list and the quest panel can scroll.
    pub sidebar_max_scroll: f32,
    pub panel_max_scroll: f32,
}

#[allow(dead_code, reason = "phase 2 glue calls it step by step")]
impl Scene {
    /// What a click at `p` does.
    pub fn action_at(&self, p: [f32; 2]) -> Option<&Action> {
        self.zones.iter().rev().find(|(rect, _)| canvas::inside(*rect, p)).map(|(_, action)| action)
    }

    fn fill(&mut self, rect: [f32; 4], colour: u32) {
        self.draws.push(Draw::Fill { rect, colour });
    }

    fn text(&mut self, text: impl Into<String>, at: [f32; 2], colour: u32) {
        let text = text.into();
        if !text.is_empty() {
            self.draws.push(Draw::Text { text, at, colour, shadow: true });
        }
    }

    fn polygon(&mut self, shape: Shape, centre: [f32; 2], radius: f32, colour: u32) {
        for corners in polygon_quads(&shape_points(shape, centre, radius)) {
            self.draws.push(Draw::Quad { corners, colour });
        }
    }

    /// A button with its right edge at `right`: its box, label and zone.
    /// Returns its left edge.
    fn button(&mut self, s: &Screen<'_>, label: &str, right: f32, y: f32, action: Action) -> f32 {
        let w = (s.measure)(label) + 6.0;
        let rect = [right - w, y, w, TEXT];
        self.fill(rect, if canvas::inside(rect, s.mouse) { BUTTON_HOVER } else { BUTTON });
        self.text(label, [rect[0] + 3.0, y + 1.0], INK);
        self.zones.push((rect, action));
        rect[0]
    }
}

/// What the book is drawn from, this frame.
#[derive(Clone, Copy)]
#[allow(dead_code, reason = "phase 2 glue calls it step by step")]
pub struct Screen<'a> {
    pub book: &'a Book,
    pub progress: &'a Progress,
    /// The name this player's claims are recorded under.
    pub player: &'a str,
    /// The whole book, `[x, y, w, h]` in GUI pixels, and the pointer.
    pub rect: [f32; 4],
    pub mouse: [f32; 2],
    /// The width of a text in the game's font.
    pub measure: &'a dyn Fn(&str) -> f32,
}

/// A row of the chapter list.
#[derive(Clone, Debug, PartialEq, Eq)]
#[allow(dead_code, reason = "phase 2 glue calls it step by step")]
pub enum Row {
    /// A group's heading.
    Group(String),
    Chapter(usize),
}

/// The chapter list: chapters without a group first, then each group of
/// quests.json with its chapters. Hidden chapters are not listed.
#[allow(dead_code, reason = "phase 2 glue calls it step by step")]
pub fn sidebar_rows(book: &Book) -> Vec<Row> {
    let file = book.file();
    let groups = &file.settings.groups;
    let group_of = |c: usize| file.chapters[c].group.as_deref().filter(|g| groups.iter().any(|group| group.id == *g));
    let listed = || (0..file.chapters.len()).filter(|&c| !file.chapters[c].hidden);
    let mut rows: Vec<Row> = listed().filter(|&c| group_of(c).is_none()).map(Row::Chapter).collect();
    for (g, group) in groups.iter().enumerate() {
        let members: Vec<usize> = listed().filter(|&c| group_of(c) == Some(group.id.as_str())).collect();
        // Two groups of the same id: the first one has the chapters.
        if !members.is_empty() && !groups[..g].iter().any(|earlier| earlier.id == group.id) {
            rows.push(Row::Group(if group.title.is_empty() { group.id.clone() } else { group.title.clone() }));
            rows.extend(members.into_iter().map(Row::Chapter));
        }
    }
    rows
}

/// The corners of a shape of this radius round `centre`, in the order
/// `hq_canvas::line_quad` goes round its quads.
#[allow(dead_code, reason = "phase 2 glue calls it step by step")]
pub fn shape_points(shape: Shape, centre: [f32; 2], radius: f32) -> Vec<[f32; 2]> {
    let [cx, cy] = centre;
    let (sides, start) = match shape {
        Shape::Square => {
            let r = radius;
            return vec![[cx - r, cy + r], [cx + r, cy + r], [cx + r, cy - r], [cx - r, cy - r]];
        }
        Shape::Diamond => (4, 0.0),
        Shape::Hexagon => (6, 0.0),
        Shape::Octagon => (8, std::f32::consts::PI / 8.0),
        Shape::Circle => (16, 0.0),
    };
    (0..sides)
        .map(|k| {
            let angle = start + k as f32 * std::f32::consts::TAU / sides as f32;
            [cx + radius * angle.cos(), cy - radius * angle.sin()]
        })
        .collect()
}

/// A convex polygon with an even number of corners, as quads fanned from
/// its first corner.
#[allow(dead_code, reason = "phase 2 glue calls it step by step")]
pub fn polygon_quads(points: &[[f32; 2]]) -> Vec<[[f32; 2]; 4]> {
    (1..points.len().saturating_sub(2)).step_by(2).map(|i| [points[0], points[i], points[i + 1], points[i + 2]]).collect()
}

/// `text` cut into lines at most `width` wide, at spaces where it can.
#[allow(dead_code, reason = "phase 2 glue calls it step by step")]
pub fn wrap(text: &str, width: f32, measure: &dyn Fn(&str) -> f32) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        let longer = if line.is_empty() { word.to_owned() } else { format!("{line} {word}") };
        if measure(&longer) <= width {
            line = longer;
            continue;
        }
        if !line.is_empty() {
            lines.push(std::mem::take(&mut line));
        }
        // A word wider than a line is cut, at least one character a line.
        let mut rest = word;
        while measure(rest) > width && rest.chars().count() > 1 {
            let cut = rest
                .char_indices()
                .skip(1)
                .map(|(i, _)| i)
                .take_while(|&i| measure(&rest[..i]) <= width)
                .last()
                .unwrap_or_else(|| rest.chars().next().map_or(0, char::len_utf8));
            lines.push(rest[..cut].to_owned());
            rest = &rest[cut..];
        }
        line = rest.to_owned();
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}

/// `text`, shortened with `...` when it is wider than `width`.
#[allow(dead_code, reason = "phase 2 glue calls it step by step")]
pub fn fit_text(text: &str, width: f32, measure: &dyn Fn(&str) -> f32) -> String {
    if measure(text) <= width {
        return text.to_owned();
    }
    let mut ends: Vec<usize> = text.char_indices().map(|(i, _)| i).collect();
    while let Some(end) = ends.pop() {
        let short = format!("{}...", text[..end].trim_end());
        if measure(&short) <= width {
            return short;
        }
    }
    String::new()
}

/// `minecraft:oak_log` → `Oak log`: a name for an id, until the book shows
/// the game's own names.
#[allow(dead_code, reason = "phase 2 glue calls it step by step")]
pub fn pretty(id: &str) -> String {
    let path = id.rsplit(':').next().unwrap_or(id);
    let name = path.rsplit('/').next().unwrap_or(path).replace('_', " ");
    let mut chars = name.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => name,
    }
}

#[allow(dead_code, reason = "phase 2 glue calls it step by step")]
fn counted(count: u32, name: &str) -> String {
    if count == 1 { name.to_owned() } else { format!("{count}x {name}") }
}

/// What a task asks, in a few words: its title, or one made from its type.
#[allow(dead_code, reason = "phase 2 glue calls it step by step")]
pub fn task_label(task: &Task) -> String {
    if let Some(title) = task.title.as_deref().filter(|title| !title.trim().is_empty()) {
        return title.to_owned();
    }
    match &task.kind {
        TaskKind::Item { item, count, consume } => {
            let name = match item {
                ItemMatch::Id(id) => pretty(id),
                ItemMatch::Tag(tag) => format!("{} (any)", pretty(tag)),
            };
            let what = counted(*count, &name);
            if *consume { format!("Hand in {what}") } else { what }
        }
        TaskKind::Checkmark => "Confirm".to_owned(),
        TaskKind::Kill { entity, count } => format!("Kill {}", counted(*count, &pretty(entity))),
        TaskKind::Craft { item, count } => {
            let name = match item {
                ItemMatch::Id(id) => pretty(id),
                ItemMatch::Tag(tag) => format!("{} (any)", pretty(tag)),
            };
            format!("Craft {}", counted(*count, &name))
        }
        TaskKind::XpLevels { levels } => format!("Reach level {levels}"),
        TaskKind::Dimension { dimension } => format!("Go to: {}", pretty(dimension)),
        TaskKind::Location { dimension, from, to } => {
            // The middle of the box is where to walk to.
            let middle = |axis: usize| (i64::from(from[axis]) + i64::from(to[axis])).div_euclid(2);
            let place = format!("Go near {}, {}, {}", middle(0), middle(1), middle(2));
            match dimension {
                Some(dimension) => format!("{place} ({})", pretty(dimension)),
                None => place,
            }
        }
        TaskKind::Advancement { advancement } => format!("Advancement: {}", pretty(advancement)),
    }
}

/// What a reward of this kind gives, in a few words.
#[allow(dead_code, reason = "phase 3 glue calls it step by step")]
pub fn kind_label(kind: &RewardKind) -> String {
    match kind {
        RewardKind::Item { item, count } => counted(*count, &pretty(item)),
        RewardKind::Xp { points } => format!("{points} XP"),
        RewardKind::XpLevels { levels: 1 } => "1 level".to_owned(),
        RewardKind::XpLevels { levels } => format!("{levels} levels"),
        RewardKind::Command { .. } => "A special reward".to_owned(),
        RewardKind::Toast { text } if !text.trim().is_empty() => text.clone(),
        RewardKind::Toast { .. } => "A message".to_owned(),
        RewardKind::Random { table } => format!("Random: {}", pretty(table)),
        RewardKind::Choice { table } => format!("Your choice: {}", pretty(table)),
    }
}

/// What something given is called: its title when it has one, else a name
/// made from its kind.
#[allow(dead_code, reason = "phase 3 glue calls it step by step")]
pub fn titled(title: Option<&str>, kind: &RewardKind) -> String {
    match title.filter(|title| !title.trim().is_empty()) {
        Some(title) => title.to_owned(),
        None => kind_label(kind),
    }
}

/// What a reward gives, in a few words: its title, or one made from its
/// type.
#[allow(dead_code, reason = "phase 2 glue calls it step by step")]
pub fn reward_label(reward: &Reward) -> String {
    titled(reward.title.as_deref(), &reward.kind)
}

/// What a reward table's entry gives, in a few words.
#[allow(dead_code, reason = "phase 3 glue calls it step by step")]
pub fn entry_label(entry: &TableEntry) -> String {
    titled(entry.title.as_deref(), &entry.kind)
}

/// The item a quest's node shows.
#[allow(dead_code, reason = "phase 2 glue calls it step by step")]
pub fn quest_icon(quest: &Quest) -> &str {
    if let Some(icon) = &quest.icon {
        return icon;
    }
    match quest.tasks.first().map(|task| &task.kind) {
        Some(TaskKind::Item { item: ItemMatch::Id(id), .. } | TaskKind::Craft { item: ItemMatch::Id(id), .. }) => id,
        Some(TaskKind::Craft { .. }) => "minecraft:crafting_table",
        Some(TaskKind::Kill { .. }) => "minecraft:iron_sword",
        Some(TaskKind::XpLevels { .. }) => "minecraft:experience_bottle",
        Some(TaskKind::Dimension { .. } | TaskKind::Location { .. }) => "minecraft:compass",
        Some(TaskKind::Advancement { .. }) => "minecraft:knowledge_book",
        _ => DEFAULT_ICON,
    }
}

/// A state's colour (a node's frame) and its name.
#[allow(dead_code, reason = "phase 2 glue calls it step by step")]
pub fn state_look(state: QuestState) -> (u32, &'static str) {
    match state {
        QuestState::Locked => (0xFF50_505A, "Locked"),
        QuestState::Available => (0xFFD8_D8E0, "Available"),
        QuestState::InProgress => (0xFFE8_C040, "In progress"),
        QuestState::RewardsWaiting => (0xFF50_B8FF, "Rewards waiting"),
        QuestState::Done => (0xFF40_B050, "Completed"),
    }
}

/// A node's box on screen.
#[allow(dead_code, reason = "phase 2 glue calls it step by step")]
fn node_box(view: &View, origin: [f32; 2], quest: &Quest, size: f32) -> [f32; 4] {
    let [sx, sy] = view.to_screen(origin, canvas::cell_centre([quest.x, quest.y]));
    let side = canvas::NODE * size * view.zoom;
    [sx - side / 2.0, sy - side / 2.0, side, side]
}

/// The part of `a` inside `b`.
#[allow(dead_code, reason = "phase 2 glue calls it step by step")]
fn intersect(a: [f32; 4], b: [f32; 4]) -> Option<[f32; 4]> {
    let (x0, y0) = (a[0].max(b[0]), a[1].max(b[1]));
    let (x1, y1) = ((a[0] + a[2]).min(b[0] + b[2]), (a[1] + a[3]).min(b[1] + b[3]));
    (x1 > x0 && y1 > y0).then_some([x0, y0, x1 - x0, y1 - y0])
}

/// Whether `inner` is wholly inside `outer`.
#[allow(dead_code, reason = "phase 2 glue calls it step by step")]
fn contains(outer: [f32; 4], inner: [f32; 4]) -> bool {
    inner[0] >= outer[0]
        && inner[1] >= outer[1]
        && inner[0] + inner[2] <= outer[0] + outer[2]
        && inner[1] + inner[3] <= outer[1] + outer[3]
}

/// The view that shows all of a chapter's visible quests in `area`, never
/// zoomed in past 1.
#[allow(dead_code, reason = "phase 2 glue calls it step by step")]
pub fn fit(book: &Book, chapter: usize, statuses: &[Status], area: [f32; 4]) -> View {
    if chapter >= book.file().chapters.len() {
        return View::default();
    }
    let mut bounds: Option<[f32; 4]> = None;
    for n in book.chapter_nodes(chapter).filter(|&n| statuses[n].visible) {
        let quest = book.quest(n);
        let [x, y] = canvas::cell_centre([quest.x, quest.y]);
        let half = canvas::NODE * book.nodes()[n].size / 2.0;
        let b = bounds.unwrap_or([x - half, y - half, x + half, y + half]);
        bounds = Some([b[0].min(x - half), b[1].min(y - half), b[2].max(x + half), b[3].max(y + half)]);
    }
    let Some([x0, y0, x1, y1]) = bounds else {
        return View::default();
    };
    let across = (area[2] - 2.0 * MARGIN) / (x1 - x0).max(1.0);
    let down = (area[3] - 2.0 * MARGIN) / (y1 - y0).max(1.0);
    let zoom = across.min(down).min(1.0).clamp(canvas::MIN_ZOOM, canvas::MAX_ZOOM);
    View { pan: [(x0 + x1) / 2.0, (y0 + y1) / 2.0], zoom }
}

/// A line of the quest panel.
#[allow(dead_code, reason = "phase 2 glue calls it step by step")]
struct PanelRow {
    text: String,
    colour: u32,
    button: Option<(&'static str, Action)>,
    /// An item (or `#tag`) the row is about: an icon before the text (for
    /// an item), and a click on it shows its recipes.
    item: Option<String>,
}

#[allow(dead_code, reason = "phase 2 glue calls it step by step")]
impl PanelRow {
    fn new(text: impl Into<String>, colour: u32) -> Self {
        Self { text: text.into(), colour, button: None, item: None }
    }

    /// Adds `text` and its button to `rows`, `room` pixels wide: on one row
    /// when they fit, else the text wrapped and the button on a row below.
    fn push(
        rows: &mut Vec<PanelRow>,
        s: &Screen<'_>,
        room: f32,
        text: String,
        colour: u32,
        button: Option<(&'static str, Action)>,
    ) {
        Self::push_item(rows, s, room, text, colour, button, None);
    }

    /// As `push`, for a row about `item`: its first line gets the icon and
    /// the link to its recipes.
    fn push_item(
        rows: &mut Vec<PanelRow>,
        s: &Screen<'_>,
        room: f32,
        text: String,
        colour: u32,
        button: Option<(&'static str, Action)>,
        item: Option<String>,
    ) {
        let icon_w = if item.as_deref().is_some_and(|i| !i.starts_with('#')) { ICON_ROW } else { 0.0 };
        let button_w = button.as_ref().map_or(0.0, |(label, _)| (s.measure)(label) + 6.0 + PAD);
        if (s.measure)(&text) + button_w + icon_w <= room {
            rows.push(PanelRow { text, colour, button, item });
            return;
        }
        let mut lines = wrap(&text, (room - icon_w).max(1.0), s.measure).into_iter();
        if let Some(first) = lines.next() {
            rows.push(PanelRow { text: first, colour, button: None, item });
        }
        rows.extend(lines.map(|line| PanelRow::new(line, colour)));
        if button.is_some() {
            rows.push(PanelRow { text: String::new(), colour, button, item: None });
        }
    }
}

/// What the player looks at in the book: kept by the client between frames.
#[derive(Clone, Debug, Default, PartialEq)]
#[allow(dead_code, reason = "phase 2 glue calls it step by step")]
pub struct BookView {
    /// The chapter shown, and how its canvas is looked at.
    pub chapter: usize,
    pub view: View,
    /// The quest whose panel is open.
    pub selected: Option<usize>,
    pub sidebar_scroll: f32,
    pub panel_scroll: f32,
    /// The last thing the server said (a refusal), for the footer.
    pub notice: String,
    /// The player's party, for the header.
    pub party: Option<String>,
    /// The quest shown on the HUD (full id).
    pub tracked: Option<String>,
    /// The recipe view over the book, when open.
    pub recipe: Option<RecipeOverlay>,
    /// An item whose recipes the player asked for: the glue looks them up
    /// and opens `recipe`.
    pub recipe_wanted: Option<String>,
    /// Whether a chapter was fitted to the canvas yet.
    fitted: bool,
}

#[allow(dead_code, reason = "phase 2 glue calls it step by step")]
impl BookView {
    /// The book opens, or a new pack arrived: keep the place when there is
    /// one, else show the first chapter.
    pub fn open(&mut self, s: &Screen<'_>) {
        let listed: Vec<usize> = sidebar_rows(s.book)
            .into_iter()
            .filter_map(|row| match row {
                Row::Chapter(c) => Some(c),
                Row::Group(_) => None,
            })
            .collect();
        if !self.fitted || !listed.contains(&self.chapter) {
            self.show_chapter(s, listed.first().copied().unwrap_or(0));
        }
        if self.selected.is_some_and(|n| n >= s.book.nodes().len()) {
            self.selected = None;
        }
    }

    /// Turns to chapter `c`, fitted to the canvas.
    pub fn show_chapter(&mut self, s: &Screen<'_>, c: usize) {
        let statuses = s.book.statuses(s.progress, s.player);
        self.chapter = c;
        self.selected = None;
        self.panel_scroll = 0.0;
        self.view = fit(s.book, c, &statuses, regions(s.rect, false).canvas);
        self.fitted = true;
    }

    /// The selected quest, if its panel shows: it must be visible and in
    /// the chapter shown.
    fn shown_selection(&self, book: &Book, statuses: &[Status]) -> Option<usize> {
        self.selected.filter(|&n| n < statuses.len() && statuses[n].visible && book.nodes()[n].chapter == self.chapter)
    }

    /// Opens quest `n`'s panel, or closes the panel. The panel takes room
    /// from the canvas: the view moves so the quests stay where they are,
    /// then just enough for the selected quest not to end up under the panel.
    fn select(&mut self, s: &Screen<'_>, selected: Option<usize>) {
        let statuses = s.book.statuses(s.progress, s.player);
        let origin = |view: &Self| {
            canvas::centre(regions(s.rect, view.shown_selection(s.book, &statuses).is_some()).canvas)
        };
        let before = origin(self);
        if self.selected != selected {
            self.selected = selected;
            self.panel_scroll = 0.0;
        }
        let after = origin(self);
        self.view.pan[0] += (after[0] - before[0]) / self.view.zoom;
        self.view.pan[1] += (after[1] - before[1]) / self.view.zoom;

        if let Some(n) = self.shown_selection(s.book, &statuses) {
            let area = regions(s.rect, true).canvas;
            let b = node_box(&self.view, canvas::centre(area), s.book.quest(n), s.book.nodes()[n].size);
            // How far the node, and a little room round it, sticks out of the canvas.
            let out = |low: f32, high: f32, from: f32, to: f32| {
                if high + PAD > to { high + PAD - to } else { (low - PAD - from).min(0.0) }
            };
            self.view.pan[0] += out(b[0], b[0] + b[2], area[0], area[0] + area[2]) / self.view.zoom;
            self.view.pan[1] += out(b[1], b[1] + b[3], area[1], area[1] + area[3]) / self.view.zoom;
        }
    }

    /// A left click at the pointer.
    pub fn click(&mut self, s: &Screen<'_>) -> Reaction {
        let action = self.scene(s).action_at(s.mouse).cloned();
        match action {
            Some(Action::Chapter(c)) => self.show_chapter(s, c),
            Some(Action::Quest(n)) => self.select(s, Some(n)),
            Some(Action::Deselect) => self.select(s, None),
            Some(Action::Check(task)) => return Reaction::Send(ToServer::Check { task }),
            Some(Action::Submit(task)) => return Reaction::Send(ToServer::Submit { task }),
            Some(Action::Claim(reward)) => return Reaction::Send(ToServer::Claim { reward, choice: None }),
            Some(Action::Choose(reward, entry)) => {
                return Reaction::Send(ToServer::Claim { reward, choice: Some(entry) });
            }
            Some(Action::ClaimAll) => return Reaction::Send(ToServer::ClaimAll),
            Some(Action::Close) => return Reaction::Close,
            Some(Action::Track(quest)) => {
                self.tracked = Some(quest);
                return Reaction::Tracked(self.tracked.clone());
            }
            Some(Action::Untrack) => {
                self.tracked = None;
                return Reaction::Tracked(None);
            }
            Some(Action::ShowRecipe(item)) => self.recipe_wanted = Some(item),
            Some(Action::TurnRecipe(by)) => {
                if let Some(recipe) = &mut self.recipe {
                    recipe.turn(by);
                }
            }
            Some(Action::CloseRecipe) => self.recipe = None,
            Some(Action::KeepRecipe) | None => {}
        }
        Reaction::Nothing
    }

    /// The pointer dragged by `delta`: over the canvas, it pans.
    pub fn drag(&mut self, s: &Screen<'_>, delta: [f32; 2]) {
        if self.recipe.is_none() && canvas::inside(self.scene(s).regions.canvas, s.mouse) {
            self.view.drag(delta);
        }
    }

    /// The wheel turned `steps` notches (positive: away from the player):
    /// it zooms the canvas, and scrolls the chapter list and the panel.
    pub fn scroll(&mut self, s: &Screen<'_>, steps: f64) {
        let scene = self.scene(s);
        let regions = scene.regions;
        let moved = |scroll: f32, by: f32, max: f32| (scroll - steps as f32 * by).clamp(0.0, max.max(0.0));
        if !steps.is_finite() || self.recipe.is_some() {
        } else if canvas::inside(regions.sidebar, s.mouse) {
            self.sidebar_scroll = moved(self.sidebar_scroll, ROW, scene.sidebar_max_scroll);
        } else if regions.panel.is_some_and(|panel| canvas::inside(panel, s.mouse)) {
            self.panel_scroll = moved(self.panel_scroll, 3.0 * TEXT, scene.panel_max_scroll);
        } else if canvas::inside(regions.canvas, s.mouse) {
            self.view.zoom_at(canvas::centre(regions.canvas), s.mouse, steps);
        }
    }

    /// Everything to draw this frame, and where clicks land.
    pub fn scene(&self, s: &Screen<'_>) -> Scene {
        let statuses = s.book.statuses(s.progress, s.player);
        let selected = self.shown_selection(s.book, &statuses);
        let regions = regions(s.rect, selected.is_some());
        let mut scene =
            Scene { regions, draws: Vec::new(), zones: Vec::new(), sidebar_max_scroll: 0.0, panel_max_scroll: 0.0 };
        scene.fill(s.rect, SHADE);
        let tooltip = self.draw_canvas(&mut scene, s, &statuses, selected);
        self.draw_sidebar(&mut scene, s);
        if let (Some(n), Some(panel)) = (selected, regions.panel) {
            self.draw_panel(&mut scene, s, &statuses, n, panel);
        }
        self.draw_bands(&mut scene, s, &statuses);
        if let Some(recipe) = &self.recipe {
            draw_recipe(&mut scene, s, recipe);
        } else if let Some(text) = tooltip {
            draw_tooltip(&mut scene, s, &text);
        }
        scene
    }

    /// The chapter's lines and nodes. Returns the title of the quest under
    /// the pointer.
    fn draw_canvas(
        &self,
        scene: &mut Scene,
        s: &Screen<'_>,
        statuses: &[Status],
        selected: Option<usize>,
    ) -> Option<String> {
        let book = s.book;
        if book.file().chapters.is_empty() {
            // No pack in the world and the example chapter turned off.
            let area = scene.regions.canvas;
            let [cx, cy] = canvas::centre(area);
            let width = (s.measure)(NO_QUESTS);
            scene.text(NO_QUESTS, [cx - width / 2.0, cy - 4.0], INK);
            return None;
        }
        if self.chapter >= book.file().chapters.len() {
            return None;
        }
        let area = scene.regions.canvas;
        let origin = canvas::centre(area);
        let shown: Vec<usize> = book.chapter_nodes(self.chapter).filter(|&n| statuses[n].visible).collect();
        let centre_of = |n: usize| {
            let quest = book.quest(n);
            self.view.to_screen(origin, canvas::cell_centre([quest.x, quest.y]))
        };

        // Dependency lines first, under the nodes. Those to another
        // chapter's quests are not drawn.
        let width = (2.0 * self.view.zoom).max(1.0);
        for &n in &shown {
            for &d in &book.nodes()[n].dependencies {
                if !shown.contains(&d) {
                    continue;
                }
                let colour = if s.progress.is_completed(&book.nodes()[d].id) { LINE_DONE } else { LINE };
                if let Some(corners) = canvas::line_quad(centre_of(d), centre_of(n), width)
                    && canvas::overlaps(area, canvas::quad_bounds(&corners))
                {
                    scene.draws.push(Draw::Quad { corners, colour });
                }
            }
        }

        let mut tooltip = None;
        for &n in &shown {
            let node = &book.nodes()[n];
            let quest = book.quest(n);
            let b = node_box(&self.view, origin, quest, node.size);
            // Only the part of the node inside the canvas takes clicks.
            let Some(part) = intersect(b, area) else {
                continue;
            };
            let centre = canvas::centre(b);
            let radius = b[2] / 2.0;
            if selected == Some(n) {
                scene.polygon(node.shape, centre, radius + 2.0, SELECTED);
            }
            scene.polygon(node.shape, centre, radius, state_look(statuses[n].state).0);
            scene.polygon(node.shape, centre, (radius - 1.5).max(0.0), NODE_FILL);
            // A diamond has less room inside than the other shapes.
            let share = if node.shape == Shape::Diamond { 0.45 } else { 0.6 };
            let icon = (b[2] * share).clamp(4.0, 64.0);
            let at = [centre[0] - icon / 2.0, centre[1] - icon / 2.0];
            if contains(area, [at[0], at[1], icon, icon]) {
                scene.draws.push(Draw::Item { item: quest_icon(quest).to_owned(), at, size: icon });
            }
            scene.zones.push((part, Action::Quest(n)));
            if canvas::inside(part, s.mouse) {
                tooltip = Some(if quest.title.is_empty() { quest.id.clone() } else { quest.title.clone() });
            }
        }
        tooltip.filter(|_| canvas::inside(area, s.mouse))
    }

    /// The chapter list.
    fn draw_sidebar(&self, scene: &mut Scene, s: &Screen<'_>) {
        let area = scene.regions.sidebar;
        scene.fill(area, SIDEBAR);
        let rows = sidebar_rows(s.book);
        scene.sidebar_max_scroll = (rows.len() as f32 * ROW + 2.0 * PAD - area[3]).max(0.0);
        let [x, _, w, _] = area;
        for (i, row) in rows.iter().enumerate() {
            let rect = [x, area[1] + PAD + i as f32 * ROW - self.sidebar_scroll, w, ROW];
            if !contains(area, rect) {
                continue;
            }
            match row {
                Row::Group(title) => {
                    let title = fit_text(title, w - 2.0 * PAD, s.measure);
                    scene.text(title, [x + PAD, rect[1] + 3.0], GOLD_INK);
                }
                Row::Chapter(c) => {
                    let chapter = s.book.chapter(*c);
                    if *c == self.chapter {
                        scene.fill(rect, ROW_CURRENT);
                    } else if canvas::inside(rect, s.mouse) {
                        scene.fill(rect, ROW_HOVER);
                    }
                    let mut left = x + PAD;
                    if let Some(icon) = &chapter.icon {
                        scene.draws.push(Draw::Item { item: icon.clone(), at: [left, rect[1] + 2.0], size: 10.0 });
                        left += 13.0;
                    }
                    let name = if chapter.title.is_empty() { &chapter.id } else { &chapter.title };
                    scene.text(fit_text(name, x + w - PAD - left, s.measure), [left, rect[1] + 2.0], INK);
                    // How far the chapter is, as a thin bar under its name.
                    let (done, total) = s.book.chapter_tally(*c, s.progress);
                    let track = [x + PAD, rect[1] + ROW - 2.5, w - 2.0 * PAD, 1.5];
                    scene.fill(track, BAR);
                    if done > 0 {
                        scene.fill([track[0], track[1], track[2] * done as f32 / total as f32, track[3]], LINE_DONE);
                    }
                    scene.zones.push((rect, Action::Chapter(*c)));
                }
            }
        }
    }

    /// The panel of quest `n`: what it is, its tasks and its rewards.
    fn draw_panel(&self, scene: &mut Scene, s: &Screen<'_>, statuses: &[Status], n: usize, panel: [f32; 4]) {
        let book = s.book;
        let node = &book.nodes()[n];
        let quest = book.quest(n);
        let [x, y, w, h] = panel;
        let room = (w - 2.0 * PAD).max(0.0);
        scene.fill(panel, PANEL);

        let mut rows: Vec<PanelRow> = Vec::new();
        if let Some(subtitle) = &quest.subtitle {
            rows.extend(wrap(subtitle, room, s.measure).into_iter().map(|line| PanelRow::new(line, DIM_INK)));
        }
        let (colour, state) = state_look(statuses[n].state);
        rows.push(PanelRow::new(state, colour & 0xFF_FFFF));
        if quest.optional {
            rows.push(PanelRow::new("Optional", DIM_INK));
        }
        if quest.repeatable {
            let times = s.progress.repeats.get(&node.id).copied().unwrap_or(0);
            let done = match times {
                0 => String::new(),
                1 => ", done once".to_owned(),
                times => format!(", done {times} times"),
            };
            rows.extend(
                wrap(&format!("Repeatable{done}"), room, s.measure).into_iter().map(|line| PanelRow::new(line, DIM_INK)),
            );
        }
        if statuses[n].state == QuestState::Locked {
            rows.push(PanelRow::new("Needs:", GOLD_INK));
            for &d in &node.dependencies {
                let dependency = book.quest(d);
                let known = statuses[d].visible;
                let name = if !known {
                    "???"
                } else if dependency.title.is_empty() {
                    &dependency.id
                } else {
                    &dependency.title
                };
                let mark = if s.progress.is_completed(&book.nodes()[d].id) { "[x]" } else { "[ ]" };
                rows.push(PanelRow::new(fit_text(&format!("{mark} {name}"), room, s.measure), DIM_INK));
            }
        }
        for paragraph in &quest.description {
            rows.push(PanelRow::new("", INK));
            rows.extend(wrap(paragraph, room, s.measure).into_iter().map(|line| PanelRow::new(line, INK)));
        }

        if !quest.tasks.is_empty() {
            rows.push(PanelRow::new("", INK));
            rows.push(PanelRow::new(if quest.sequential_tasks { "Tasks, in order" } else { "Tasks" }, GOLD_INK));
        }
        for (t, task) in quest.tasks.iter().enumerate() {
            let id = book.task_id(n, t);
            let goal = task.kind.goal();
            let done = s.progress.task(&id).min(goal);
            let finished = done >= goal || s.progress.is_completed(&node.id);
            // Not open, or waiting for the tasks before it.
            let counts = book.task_counts(n, t, s.progress);
            let mark = if finished { "[x]" } else { "[ ]" };
            let count = if goal > 1 { format!("  {done}/{goal}") } else { String::new() };
            let button = match task.kind {
                _ if !counts || finished => None,
                TaskKind::Checkmark => Some(("Check", Action::Check(id))),
                TaskKind::Item { consume: true, .. } => Some(("Submit", Action::Submit(id))),
                _ => None,
            };
            let text = format!("{mark} {}{count}", task_label(task));
            let waiting = quest.sequential_tasks && !counts;
            let item = match &task.kind {
                TaskKind::Item { item, .. } | TaskKind::Craft { item, .. } => Some(String::from(item.clone())),
                _ => None,
            };
            PanelRow::push_item(&mut rows, s, room, text, if finished || waiting { DIM_INK } else { INK }, button, item);
        }

        if !quest.rewards.is_empty() {
            rows.push(PanelRow::new("", INK));
            rows.push(PanelRow::new("Rewards", GOLD_INK));
        }
        let completed = s.progress.is_completed(&node.id);
        for (r, reward) in quest.rewards.iter().enumerate() {
            let id = book.reward_id(n, r);
            let label = reward_label(reward);
            let item = match &reward.kind {
                RewardKind::Item { item, .. } => Some(item.clone()),
                _ => None,
            };
            let label = if reward.team { format!("{label} (team)") } else { label };
            if s.progress.claimed_for(&id, s.player, reward.team) {
                PanelRow::push_item(&mut rows, s, room, format!("{label} (claimed)"), DIM_INK, None, item);
                continue;
            }
            // A reward from a table lists what the table holds: a choice
            // reward's entries are picked there, a random one's are only shown.
            let entries = reward.kind.table().and_then(|table| book.table(table)).map(|table| &table.entries);
            let choice = matches!(reward.kind, RewardKind::Choice { .. });
            let button = (completed && !choice).then(|| ("Claim", Action::Claim(id.clone())));
            PanelRow::push_item(&mut rows, s, room, label, INK, button, item);
            for (e, entry) in entries.into_iter().flatten().enumerate().take(TABLE_ROWS) {
                let pick = (completed && choice).then(|| ("Pick", Action::Choose(id.clone(), e as u32)));
                let colour = if pick.is_some() { INK } else { DIM_INK };
                PanelRow::push(&mut rows, s, room, format!("- {}", entry_label(entry)), colour, pick);
            }
            if let Some(more) = entries.map(|entries| entries.len().saturating_sub(TABLE_ROWS)).filter(|more| *more > 0) {
                rows.push(PanelRow::new(format!("- ... and {more} more"), DIM_INK));
            }
        }

        // The rows scroll under the title bar; only whole rows are drawn.
        let body = [x, y + TITLE_BAR, w, (h - TITLE_BAR).max(0.0)];
        scene.panel_max_scroll = (rows.len() as f32 * TEXT + 2.0 * PAD - body[3]).max(0.0);
        for (i, row) in rows.into_iter().enumerate() {
            let top = body[1] + PAD + i as f32 * TEXT - self.panel_scroll;
            if !contains(body, [x, top, w, TEXT]) {
                continue;
            }
            let mut right = x + w - PAD;
            if let Some((label, action)) = row.button {
                right = scene.button(s, label, right, top, action) - PAD;
            }
            let mut left = x + PAD;
            if let Some(item) = &row.item {
                // The icon and the text open the item's recipes. The hover
                // shade goes first, under the icon.
                let link = [x + PAD, top, (right - x - PAD).max(0.0), TEXT];
                if canvas::inside(link, s.mouse) {
                    scene.fill(link, ROW_HOVER);
                }
                scene.zones.push((link, Action::ShowRecipe(item.clone())));
                if !item.starts_with('#') {
                    scene.draws.push(Draw::Item { item: item.clone(), at: [left, top], size: 9.0 });
                    left += ICON_ROW;
                }
            }
            let text = fit_text(&row.text, right - left, s.measure);
            scene.text(text, [left, top + 1.0], row.colour);
        }

        scene.fill([x, y, w, TITLE_BAR], BAND);
        let close = scene.button(s, "x", x + w - PAD, y + 2.0, Action::Deselect);
        let tracking = self.tracked.as_deref() == Some(node.id.as_str());
        let track = if tracking {
            scene.button(s, "Untrack", close - PAD, y + 2.0, Action::Untrack)
        } else {
            scene.button(s, "Track", close - PAD, y + 2.0, Action::Track(node.id.clone()))
        };
        let title = if quest.title.is_empty() { &quest.id } else { &quest.title };
        scene.text(fit_text(title, track - PAD - (x + PAD), s.measure), [x + PAD, y + 3.0], INK);
    }

    /// The header (chapter title, tally, close) and the footer (claim all,
    /// notice, help), over anything under them.
    fn draw_bands(&self, scene: &mut Scene, s: &Screen<'_>, statuses: &[Status]) {
        let header = scene.regions.header;
        let [x, y, w, _] = header;
        scene.fill(header, BAND);
        let mut right = scene.button(s, "x", x + w - 6.0, y + 7.0, Action::Close) - 6.0;
        if let Some(party) = &self.party {
            let label = format!("Party {party}");
            let width = (s.measure)(&label);
            // Only when the chapter's name keeps some room.
            if width < w / 3.0 {
                right -= width;
                scene.text(label, [right, y + 8.0], GOLD_INK);
                right -= 10.0;
            }
        }
        if let Some(chapter) = s.book.file().chapters.get(self.chapter) {
            let (done, total) = s.book.chapter_tally(self.chapter, s.progress);
            let tally = format!("{done}/{total}");
            right -= (s.measure)(&tally);
            scene.text(tally, [right, y + 8.0], DIM_INK);
            let name = if chapter.title.is_empty() { &chapter.id } else { &chapter.title };
            let title = fit_text(name, right - 6.0 - (x + 6.0), s.measure);
            let after = x + 6.0 + (s.measure)(&title) + 8.0;
            scene.text(title, [x + 6.0, y + 8.0], INK);
            if let Some(subtitle) = &chapter.subtitle {
                scene.text(fit_text(subtitle, right - 6.0 - after, s.measure), [after, y + 8.0], DIM_INK);
            }
        }

        let footer = scene.regions.footer;
        let [x, y, w, _] = footer;
        scene.fill(footer, BAND);
        let mut left = x + 6.0;
        let waiting = statuses.iter().filter(|status| status.state == QuestState::RewardsWaiting).count();
        if waiting > 0 {
            let label = format!("Claim all ({waiting})");
            let width = (s.measure)(&label) + 6.0;
            scene.button(s, &label, left + width, y + 2.0, Action::ClaimAll);
            left += width + 6.0;
        }
        let help = "drag: move | wheel: zoom | Esc: close";
        let help_w = (s.measure)(help);
        let notice = fit_text(&self.notice, x + w - 6.0 - left, s.measure);
        let notice_end = left + (s.measure)(&notice);
        scene.text(notice, [left, y + 3.0], GOLD_INK);
        if x + w - 6.0 - help_w >= notice_end + 6.0 {
            scene.text(help, [x + w - 6.0 - help_w, y + 3.0], DIM_INK);
        }
    }
}

/// One recipe, as the recipe view draws it: item ids only.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RecipeView {
    /// Where it is made, with a note: "Crafting table", "Furnace, 10 s".
    pub station: String,
    /// The grid's columns; the slots fill it row by row.
    pub width: u32,
    /// Each slot's items, any one of which fits (shown in turn); empty for
    /// an empty cell.
    pub slots: Vec<Vec<String>>,
    pub result: String,
    pub count: u32,
}

/// The recipe view over the book: the recipes that make one item.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RecipeOverlay {
    /// The item's name.
    pub title: String,
    pub recipes: Vec<RecipeView>,
    pub page: usize,
    /// Window ticks it has been open: a slot with several items shows each
    /// for a second.
    pub clock: u32,
}

impl RecipeOverlay {
    fn turn(&mut self, by: i32) {
        let n = self.recipes.len();
        if n > 0 {
            let n = n as i64;
            self.page = (self.page as i64 + i64::from(by)).rem_euclid(n) as usize;
        }
    }
}

/// The width an icon takes at the start of a panel row.
const ICON_ROW: f32 = 11.0;
/// A recipe cell: an 18-pixel square holding a 16-pixel icon.
const CELL: f32 = 18.0;
const RECIPE_BOX: [f32; 2] = [200.0, 120.0];
const CELL_BACK: u32 = 0xFF2E_2E3C;

/// The recipe view, centred over the book. A click outside closes it.
fn draw_recipe(scene: &mut Scene, s: &Screen<'_>, recipe: &RecipeOverlay) {
    let [x, y, w, h] = s.rect;
    scene.zones.push((s.rect, Action::CloseRecipe));
    let [bw, bh] = [RECIPE_BOX[0].min(w), RECIPE_BOX[1].min(h)];
    let b = [x + (w - bw) / 2.0, y + (h - bh) / 2.0, bw, bh];
    scene.fill(b, PANEL);
    scene.zones.push((b, Action::KeepRecipe));
    scene.fill([b[0], b[1], bw, TITLE_BAR], BAND);
    let mut right = scene.button(s, "x", b[0] + bw - PAD, b[1] + 2.0, Action::CloseRecipe) - PAD;
    let n = recipe.recipes.len();
    if n > 1 {
        right = scene.button(s, ">", right, b[1] + 2.0, Action::TurnRecipe(1)) - 2.0;
        let page = format!("{}/{n}", recipe.page.min(n - 1) + 1);
        right -= (s.measure)(&page);
        scene.text(page.clone(), [right, b[1] + 3.0], DIM_INK);
        right = scene.button(s, "<", right - 2.0, b[1] + 2.0, Action::TurnRecipe(-1)) - PAD;
    }
    scene.text(fit_text(&format!("Recipes: {}", recipe.title), right - b[0] - PAD, s.measure), [b[0] + PAD, b[1] + 3.0], INK);

    let body_y = b[1] + TITLE_BAR + PAD;
    let Some(shown) = recipe.recipes.get(recipe.page.min(n.saturating_sub(1))) else {
        scene.text("No recipe makes this item.", [b[0] + PAD, body_y + 2.0], DIM_INK);
        return;
    };
    let columns = shown.width.max(1) as usize;
    let grid_x = b[0] + PAD;
    let mut grid_bottom = body_y;
    for (i, slot) in shown.slots.iter().enumerate() {
        let cell = [grid_x + (i % columns) as f32 * CELL, body_y + (i / columns) as f32 * CELL, CELL - 1.0, CELL - 1.0];
        if !contains(b, cell) {
            continue;
        }
        scene.fill(cell, CELL_BACK);
        if !slot.is_empty() {
            let item = &slot[(recipe.clock / 20) as usize % slot.len()];
            scene.draws.push(Draw::Item { item: item.clone(), at: [cell[0] + 0.5, cell[1] + 0.5], size: 16.0 });
        }
        grid_bottom = grid_bottom.max(cell[1] + CELL);
    }
    let grid_right = grid_x + columns.min(shown.slots.len().max(1)) as f32 * CELL;
    let arrow_x = grid_right + 6.0;
    let mid = body_y + ((grid_bottom - body_y).max(CELL) - CELL) / 2.0;
    scene.text("->", [arrow_x, mid + 5.0], INK);
    let out = [arrow_x + (s.measure)("->") + 6.0, mid, CELL - 1.0, CELL - 1.0];
    if contains(b, out) {
        scene.fill(out, CELL_BACK);
        scene.draws.push(Draw::Item { item: shown.result.clone(), at: [out[0] + 0.5, out[1] + 0.5], size: 16.0 });
        if shown.count > 1 {
            let count = shown.count.to_string();
            scene.text(count.clone(), [out[0] + CELL - 1.0 - (s.measure)(&count), out[1] + CELL - 9.0], INK);
        }
    }
    let station_y = grid_bottom.max(mid + CELL) + PAD;
    scene.text(fit_text(&shown.station, bw - 2.0 * PAD, s.measure), [b[0] + PAD, station_y], DIM_INK);
}

/// The most task lines the tracked quest's box lists.
const TRACKED_TASKS: usize = 6;
/// The widest the tracked quest's box gets.
const TRACKED_WIDTH: f32 = 180.0;

/// The lines of the tracked quest's box: its title, then its tasks or what
/// waits. None when the pack has no such quest or it is not visible.
fn tracked_lines(book: &Book, progress: &Progress, player: &str, quest: &str) -> Option<Vec<(String, u32)>> {
    let n = book.find(quest)?;
    let statuses = book.statuses(progress, player);
    if !statuses[n].visible {
        return None;
    }
    let found = book.quest(n);
    let title = if found.title.is_empty() { found.id.clone() } else { found.title.clone() };
    let mut lines = vec![(title, GOLD_INK)];
    match statuses[n].state {
        QuestState::Locked => lines.push(("Not open yet".to_owned(), DIM_INK)),
        QuestState::RewardsWaiting => lines.push(("Done: rewards in the quest book".to_owned(), LINE_DONE & 0xFF_FFFF)),
        QuestState::Done => lines.push(("Done".to_owned(), LINE_DONE & 0xFF_FFFF)),
        QuestState::Available | QuestState::InProgress => {
            for (t, task) in found.tasks.iter().enumerate().take(TRACKED_TASKS) {
                let goal = task.kind.goal();
                let done = progress.task(&book.task_id(n, t)).min(goal);
                let count = if goal > 1 { format!(" {done}/{goal}") } else { String::new() };
                let mark = if done >= goal { "[x]" } else { "[ ]" };
                let colour = if done >= goal { DIM_INK } else { INK };
                lines.push((format!("{mark} {}{count}", task_label(task)), colour));
            }
            if found.tasks.len() > TRACKED_TASKS {
                lines.push((format!("... and {} more", found.tasks.len() - TRACKED_TASKS), DIM_INK));
            }
        }
    }
    Some(lines)
}

/// The tracked quest's box on the HUD: its size, then its draws at `at`.
/// None when there is nothing to show.
pub fn tracked_size(book: &Book, progress: &Progress, player: &str, quest: &str, measure: &dyn Fn(&str) -> f32) -> Option<[f32; 2]> {
    let lines = tracked_lines(book, progress, player, quest)?;
    let widest = lines.iter().map(|(text, _)| measure(text)).fold(0.0, f32::max).min(TRACKED_WIDTH - 2.0 * PAD);
    Some([widest + 2.0 * PAD, lines.len() as f32 * TOAST_LINE + 2.0 * PAD])
}

pub fn tracked_draws(book: &Book, progress: &Progress, player: &str, quest: &str, at: [f32; 2], measure: &dyn Fn(&str) -> f32) -> Vec<Draw> {
    let (Some(lines), Some([w, h])) = (tracked_lines(book, progress, player, quest), tracked_size(book, progress, player, quest, measure)) else {
        return Vec::new();
    };
    let [x, y] = at;
    let mut draws = vec![Draw::Fill { rect: [x, y, w, h], colour: TOAST_BACK }, Draw::Fill { rect: [x, y, 1.0, h], colour: TOAST_EDGE }];
    for (i, (text, colour)) in lines.into_iter().enumerate() {
        let text = fit_text(&text, w - 2.0 * PAD, measure);
        draws.push(Draw::Text { text, at: [x + PAD, y + PAD + 1.0 + i as f32 * TOAST_LINE], colour, shadow: true });
    }
    draws
}

/// A small box with `text` by the pointer, kept on screen.
#[allow(dead_code, reason = "phase 2 glue calls it step by step")]
fn draw_tooltip(scene: &mut Scene, s: &Screen<'_>, text: &str) {
    let [x, y, w, h] = s.rect;
    let width = (s.measure)(text) + 6.0;
    let left = (s.mouse[0] + 8.0).min(x + w - width).max(x);
    let top = (s.mouse[1] - 14.0).min(y + h - 12.0).max(y);
    scene.fill([left, top, width, 12.0], TOOLTIP);
    scene.text(text, [left + 3.0, top + 2.0], INK);
}

/// What the canvas says when the pack has no chapter at all.
pub const NO_QUESTS: &str = "This world has no quests.";

/// Window ticks the book waits for the server's first answer before it says
/// none is coming (5 seconds).
pub const ANSWER_TIMEOUT: u32 = 100;

/// The line the book shows while it has no pack yet, `waited` ticks after it
/// asked: the server's notice when it sent one (an incompatible version),
/// else a server that never answered most likely has no Horizon Quests.
pub fn loading_line(notice: &str, waited: u32) -> &str {
    if !notice.is_empty() {
        notice
    } else if waited >= ANSWER_TIMEOUT {
        "No answer from the server: it may not have Horizon Quests."
    } else {
        "Loading quests from server..."
    }
}

/// A toast: a small box the HUD shows for a few seconds (a toast reward).
/// `icon` is an item id.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Toast {
    pub title: String,
    pub text: String,
    pub icon: Option<String>,
}

/// Window ticks a toast stays on the HUD unless the player's settings say
/// otherwise (5 seconds).
pub const TOAST_TICKS: u32 = 100;
/// The most toasts that wait their turn: more than that and the new one is
/// dropped, so a server cannot fill a client's memory with them.
const TOAST_QUEUE: usize = 8;
/// The widest a toast's box gets; longer lines are shortened.
const TOAST_WIDTH: f32 = 200.0;
const TOAST_ICON: f32 = 16.0;
const TOAST_LINE: f32 = 10.0;
const TOAST_BACK: u32 = 0xE010_1018;
const TOAST_EDGE: u32 = 0xFFFF_C040;
const TOAST_TITLE: u32 = 0xFF_C040;

/// The toasts waiting for the HUD, shown one at a time, oldest first.
#[derive(Clone, Debug, Default)]
pub struct Toasts {
    waiting: VecDeque<Toast>,
    /// Ticks the first one has been on show.
    shown: u32,
    /// Ticks each stays; 0: `TOAST_TICKS`.
    stay: u32,
}

impl Toasts {
    pub fn push(&mut self, toast: Toast) {
        if self.waiting.len() < TOAST_QUEUE {
            self.waiting.push_back(toast);
        }
    }

    /// The toast to draw now.
    pub fn current(&self) -> Option<&Toast> {
        self.waiting.front()
    }

    /// How many window ticks each toast stays (the player's settings).
    pub fn set_stay(&mut self, ticks: u32) {
        self.stay = ticks;
    }

    /// One window tick with the HUD in view: the toast on show leaves after
    /// its time (`TOAST_TICKS` unless set), and the next one takes its place.
    pub fn tick(&mut self) {
        if self.waiting.is_empty() {
            return;
        }
        self.shown += 1;
        if self.shown >= if self.stay == 0 { TOAST_TICKS } else { self.stay } {
            self.waiting.pop_front();
            self.shown = 0;
        }
    }
}

/// Where a toast's lines start, from its box's left edge.
fn toast_indent(toast: &Toast) -> f32 {
    if toast.icon.is_some() { PAD + TOAST_ICON + PAD } else { PAD }
}

/// A toast's lines (its title, then its text when it has one), shortened to
/// fit the widest box.
fn toast_lines(toast: &Toast, measure: &dyn Fn(&str) -> f32) -> Vec<String> {
    let room = TOAST_WIDTH - toast_indent(toast) - PAD;
    [&toast.title, &toast.text].into_iter().filter(|line| !line.is_empty()).map(|line| fit_text(line, room, measure)).collect()
}

/// The width and height of a toast's box.
pub fn toast_size(toast: &Toast, measure: &dyn Fn(&str) -> f32) -> [f32; 2] {
    let lines = toast_lines(toast, measure);
    let widest = lines.iter().map(|line| measure(line)).fold(0.0, f32::max);
    let icon = if toast.icon.is_some() { TOAST_ICON } else { 0.0 };
    [toast_indent(toast) + widest + PAD, (lines.len() as f32 * TOAST_LINE).max(icon) + 2.0 * PAD]
}

/// What draws `toast` in a box of `toast_size` whose top-left corner is `at`.
pub fn toast_draws(toast: &Toast, at: [f32; 2], measure: &dyn Fn(&str) -> f32) -> Vec<Draw> {
    let [w, h] = toast_size(toast, measure);
    let [x, y] = at;
    let mut draws = vec![
        Draw::Fill { rect: [x, y, w, h], colour: TOAST_BACK },
        Draw::Fill { rect: [x, y, w, 1.0], colour: TOAST_EDGE },
    ];
    for (i, line) in toast_lines(toast, measure).into_iter().enumerate() {
        let colour = if i == 0 { TOAST_TITLE } else { INK };
        let line_y = y + PAD + 1.0 + i as f32 * TOAST_LINE;
        draws.push(Draw::Text { text: line, at: [x + toast_indent(toast), line_y], colour, shadow: true });
    }
    if let Some(icon) = &toast.icon {
        draws.push(Draw::Item { item: icon.clone(), at: [x + PAD, y + PAD], size: TOAST_ICON });
    }
    draws
}

#[cfg(test)]
mod tests {
    use super::super::hq_progress::{Actor, QuestEvent, apply};
    use super::super::hq_validate::load;
    use super::*;

    const RECT: [f32; 4] = [0.0, 0.0, 480.0, 270.0];
    const NOWHERE: [f32; 2] = [-100.0, -100.0];

    /// Six pixels a character, like the game's font, near enough.
    fn measure(text: &str) -> f32 {
        text.chars().count() as f32 * 6.0
    }

    const ALPHA: &str = r#"{ "id": "a", "title": "Alpha", "subtitle": "The first one", "icon": "book", "quests": [
        { "id": "start", "title": "Start here", "subtitle": "A beginning", "x": 0, "y": 0,
          "description": ["One two three four five six seven eight nine ten eleven twelve thirteen fourteen fifteen."],
          "tasks": [ { "id": "ok", "type": "checkmark" } ],
          "rewards": [ { "id": "xp", "type": "xp", "points": 5 } ] },
        { "id": "next", "title": "Then this", "x": 2, "y": 0, "dependencies": ["start"], "shape": "circle",
          "tasks": [ { "id": "planks", "type": "item", "item": "oak_planks", "count": 4, "consume": true } ] },
        { "id": "far", "title": "A surprise", "x": 4, "y": 1, "dependencies": ["next"],
          "visibility": "until_deps_complete" } ] }"#;
    const BETA: &str = r#"{ "id": "b", "title": "Beta", "group": "main", "quests": [ { "id": "only", "title": "Only" } ] }"#;
    const HIDDEN: &str = r#"{ "id": "h", "title": "Hidden", "hidden": true, "quests": [ { "id": "ghost", "title": "Ghost" } ] }"#;

    fn book() -> Book {
        let settings = r#"{ "groups": [ { "id": "empty", "title": "Empty" }, { "id": "main", "title": "Main story" } ] }"#;
        let loaded = load(Some(settings), &[("1.json", ALPHA), ("2.json", BETA), ("3.json", HIDDEN)]);
        assert_eq!(loaded.problems, []);
        Book::new(loaded.file)
    }

    fn screen<'a>(book: &'a Book, progress: &'a Progress, mouse: [f32; 2]) -> Screen<'a> {
        Screen { book, progress, player: "steve", rect: RECT, mouse, measure: &measure }
    }

    /// An opened view of the test book.
    fn opened(book: &Book, progress: &Progress) -> BookView {
        let mut view = BookView::default();
        view.open(&screen(book, progress, NOWHERE));
        view
    }

    /// The middle of the zone that does `action`.
    fn zone(scene: &Scene, action: &Action) -> Option<[f32; 2]> {
        scene.zones.iter().rev().find(|(_, a)| a == action).map(|(rect, _)| canvas::centre(*rect))
    }

    fn click(view: &mut BookView, book: &Book, progress: &Progress, action: &Action) -> Reaction {
        let at = zone(&view.scene(&screen(book, progress, NOWHERE)), action).unwrap_or_else(|| panic!("no zone for {action:?}"));
        view.click(&screen(book, progress, at))
    }

    fn texts(scene: &Scene) -> Vec<&str> {
        scene
            .draws
            .iter()
            .filter_map(|draw| match draw {
                Draw::Text { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect()
    }

    /// Progress after `events`.
    fn progress_after(book: &Book, events: &[QuestEvent]) -> Progress {
        let mut progress = Progress::default();
        for event in events {
            apply(book, &mut progress, &Actor::new("steve", 1), event);
        }
        progress
    }

    #[test]
    fn regions_split_the_window() {
        let closed = regions(RECT, false);
        assert_eq!(closed.panel, None);
        assert_eq!(closed.header, [0.0, 0.0, 480.0, canvas::HEADER]);
        assert_eq!(closed.footer, [0.0, 270.0 - canvas::FOOTER, 480.0, canvas::FOOTER]);
        assert_eq!(closed.sidebar[2] + closed.canvas[2], 480.0);
        assert_eq!(closed.canvas[1], canvas::HEADER);
        assert_eq!(closed.canvas[3], 270.0 - canvas::HEADER - canvas::FOOTER);

        let open = regions(RECT, true);
        let panel = open.panel.unwrap();
        assert_eq!(open.sidebar, closed.sidebar);
        assert!((open.sidebar[2] + open.canvas[2] + panel[2] - 480.0).abs() < 0.01);
        assert!((panel[0] + panel[2] - 480.0).abs() < 0.01);

        // A window too small for anything: nothing goes negative.
        for rect in [[0.0, 0.0, 40.0, 20.0], [0.0, 0.0, 0.0, 0.0]] {
            let tiny = regions(rect, true);
            for part in [tiny.header, tiny.sidebar, tiny.canvas, tiny.panel.unwrap(), tiny.footer] {
                assert!(part[2] >= 0.0 && part[3] >= 0.0, "{part:?}");
            }
        }
    }

    #[test]
    fn shapes_are_fans_of_quads_inside_their_radius() {
        for (shape, quads) in
            [(Shape::Square, 1), (Shape::Diamond, 1), (Shape::Hexagon, 2), (Shape::Octagon, 3), (Shape::Circle, 7)]
        {
            let points = shape_points(shape, [100.0, 50.0], 10.0);
            assert_eq!(polygon_quads(&points).len(), quads, "{shape:?}");
            for p in &points {
                let far = ((p[0] - 100.0).powi(2) + (p[1] - 50.0).powi(2)).sqrt();
                assert!(far <= 10.0 * 1.415, "{shape:?} {p:?}");
            }
        }
        // The square goes round like a line's quad: bottom-left first, then right, then up.
        assert_eq!(shape_points(Shape::Square, [0.0, 0.0], 1.0), [[-1.0, 1.0], [1.0, 1.0], [1.0, -1.0], [-1.0, -1.0]]);
        let diamond = shape_points(Shape::Diamond, [0.0, 0.0], 1.0);
        assert!((diamond[0][0] - 1.0).abs() < 1e-5 && (diamond[1][1] + 1.0).abs() < 1e-5);
        assert_eq!(polygon_quads(&[[0.0, 0.0], [1.0, 0.0]]), Vec::<[[f32; 2]; 4]>::new());
    }

    #[test]
    fn text_wraps_and_shortens() {
        assert_eq!(wrap("one two three four", 48.0, &measure), ["one two", "three", "four"]);
        assert_eq!(wrap("  spaced   out  ", 600.0, &measure), ["spaced out"]);
        assert_eq!(wrap("", 60.0, &measure), Vec::<String>::new());
        // A word longer than a line is cut; a line narrower than a character still moves on.
        assert_eq!(wrap("abcdefghij kl", 24.0, &measure), ["abcd", "efgh", "ij", "kl"]);
        assert_eq!(wrap("héé", 1.0, &measure), ["h", "é", "é"]);

        assert_eq!(fit_text("short", 60.0, &measure), "short");
        assert_eq!(fit_text("a long title", 60.0, &measure), "a long...");
        assert_eq!(fit_text("a long title", 10.0, &measure), "");
    }

    #[test]
    fn labels() {
        assert_eq!(pretty("minecraft:oak_log"), "Oak log");
        assert_eq!(pretty("create:gears/big_gear"), "Big gear");
        assert_eq!(pretty(""), "");
        let task = |text: &str| serde_json::from_str::<Task>(text).unwrap();
        assert_eq!(task_label(&task(r#"{ "type": "item", "item": "oak_log", "count": 8 }"#)), "8x Oak log");
        assert_eq!(task_label(&task(r##"{ "type": "item", "item": "#minecraft:logs" }"##)), "Logs (any)");
        assert_eq!(
            task_label(&task(r#"{ "type": "item", "item": "rotten_flesh", "count": 2, "consume": true }"#)),
            "Hand in 2x Rotten flesh"
        );
        assert_eq!(task_label(&task(r#"{ "type": "kill", "entity": "minecraft:zombie", "count": 3 }"#)), "Kill 3x Zombie");
        assert_eq!(task_label(&task(r#"{ "type": "checkmark" }"#)), "Confirm");
        assert_eq!(task_label(&task(r#"{ "type": "checkmark", "title": "I read this" }"#)), "I read this");

        let reward = |text: &str| reward_label(&serde_json::from_str::<Reward>(text).unwrap());
        assert_eq!(reward(r#"{ "type": "item", "item": "minecraft:apple", "count": 2 }"#), "2x Apple");
        assert_eq!(reward(r#"{ "type": "xp", "points": 20 }"#), "20 XP");
        assert_eq!(reward(r#"{ "type": "xp_levels", "levels": 1 }"#), "1 level");
        assert_eq!(reward(r#"{ "type": "xp_levels", "levels": 3 }"#), "3 levels");

        let book = book();
        assert_eq!(quest_icon(book.quest(0)), DEFAULT_ICON);
        assert_eq!(quest_icon(book.quest(1)), "minecraft:oak_planks");
    }

    #[test]
    fn the_chapter_list_has_ungrouped_chapters_then_groups() {
        // The hidden chapter and the group without chapters are not listed.
        assert_eq!(sidebar_rows(&book()), [Row::Chapter(0), Row::Group("Main story".into()), Row::Chapter(1)]);
    }

    #[test]
    fn a_new_book_shows_the_first_chapter_fitted() {
        let (book, progress) = (book(), Progress::default());
        let view = opened(&book, &progress);
        assert_eq!((view.chapter, view.selected), (0, None));
        let scene = view.scene(&screen(&book, &progress, NOWHERE));

        // Both visible quests can be clicked, and are inside the canvas; the surprise is not there.
        for n in [0, 1] {
            let at = zone(&scene, &Action::Quest(n)).unwrap();
            assert!(canvas::inside(scene.regions.canvas, at), "{n}");
        }
        assert_eq!(zone(&scene, &Action::Quest(2)), None);
        assert!(zone(&scene, &Action::Chapter(0)).is_some() && zone(&scene, &Action::Chapter(1)).is_some());
        assert_eq!(zone(&scene, &Action::Chapter(2)), None);
        assert!(zone(&scene, &Action::Close).is_some());
        assert_eq!(zone(&scene, &Action::ClaimAll), None);

        let shown = texts(&scene);
        for text in ["Alpha", "The first one", "0/3", "Main story", "Beta"] {
            assert!(shown.contains(&text), "{text} in {shown:?}");
        }
        // Each listed chapter has a progress bar, empty for now.
        let bars = |scene: &Scene, colour: u32| {
            let list = scene.regions.sidebar;
            scene
                .draws
                .iter()
                .filter(|draw| matches!(draw, Draw::Fill { rect, colour: c } if *c == colour && rect[3] == 1.5 && contains(list, *rect)))
                .count()
        };
        assert_eq!((bars(&scene, BAR), bars(&scene, LINE_DONE)), (2, 0));
        // Beta's only quest has no task: once the engine has run, its bar is full.
        let done = progress_after(&book, &[QuestEvent::Refresh]);
        assert_eq!(bars(&view.scene(&screen(&book, &done, NOWHERE)), LINE_DONE), 1);
        // One icon per visible quest, and the chapter's in the list.
        let icons = scene.draws.iter().filter(|draw| matches!(draw, Draw::Item { .. })).count();
        assert_eq!(icons, 3);
        // One line, start to next, grey.
        let lines: Vec<u32> = scene
            .draws
            .iter()
            .filter_map(|draw| match draw {
                Draw::Quad { colour, .. } if [LINE, LINE_DONE].contains(colour) => Some(*colour),
                _ => None,
            })
            .collect();
        assert_eq!(lines, [LINE]);
    }

    #[test]
    fn hovering_a_quest_shows_its_title() {
        let (book, progress) = (book(), Progress::default());
        let view = opened(&book, &progress);
        let at = zone(&view.scene(&screen(&book, &progress, NOWHERE)), &Action::Quest(1)).unwrap();
        let scene = view.scene(&screen(&book, &progress, at));
        assert_eq!(texts(&scene).last(), Some(&"Then this"));
        assert!(!texts(&view.scene(&screen(&book, &progress, NOWHERE))).contains(&"Then this"));
    }

    #[test]
    fn selecting_a_quest_opens_its_panel_and_the_canvas_stays_put() {
        let (book, progress) = (book(), Progress::default());
        let mut view = opened(&book, &progress);
        let before = zone(&view.scene(&screen(&book, &progress, NOWHERE)), &Action::Quest(0)).unwrap();
        assert_eq!(click(&mut view, &book, &progress, &Action::Quest(0)), Reaction::Nothing);
        assert_eq!(view.selected, Some(0));

        let scene = view.scene(&screen(&book, &progress, NOWHERE));
        let after = zone(&scene, &Action::Quest(0)).unwrap();
        assert!((after[0] - before[0]).abs() < 0.01 && (after[1] - before[1]).abs() < 0.01, "{before:?} {after:?}");

        let panel = scene.regions.panel.unwrap();
        let shown = texts(&scene);
        for text in ["Start here", "A beginning", "Available", "Tasks", "[ ] Confirm", "Rewards", "5 XP", "Check"] {
            assert!(shown.contains(&text), "{text} in {shown:?}");
        }
        // The description is wrapped to the panel.
        for draw in &scene.draws {
            if let Draw::Text { text, at, .. } = draw
                && at[0] >= panel[0]
                && at[1] > panel[1]
                && at[1] < panel[1] + panel[3]
            {
                assert!(at[0] + measure(text) <= panel[0] + panel[2], "{text}");
            }
        }
        assert!(canvas::inside(panel, zone(&scene, &Action::Check("a/start/ok".into())).unwrap()));
        // Not completed: nothing to claim yet.
        assert_eq!(zone(&scene, &Action::Claim("a/start/xp".into())), None);

        assert_eq!(click(&mut view, &book, &progress, &Action::Deselect), Reaction::Nothing);
        assert_eq!(view.selected, None);
        let back = zone(&view.scene(&screen(&book, &progress, NOWHERE)), &Action::Quest(0)).unwrap();
        assert!((back[0] - before[0]).abs() < 0.01);
    }

    #[test]
    fn a_quest_selected_where_the_panel_opens_slides_into_view() {
        let (book, progress) = (book(), Progress::default());
        let mut view = opened(&book, &progress);
        // The second quest is on the right, where the panel will be.
        let at = zone(&view.scene(&screen(&book, &progress, NOWHERE)), &Action::Quest(1)).unwrap();
        assert!(canvas::inside(regions(RECT, true).panel.unwrap(), at));
        click(&mut view, &book, &progress, &Action::Quest(1));

        let scene = view.scene(&screen(&book, &progress, NOWHERE));
        let (rect, _) = scene.zones.iter().find(|(_, action)| *action == Action::Quest(1)).unwrap();
        assert!(contains(scene.regions.canvas, *rect), "{rect:?} in {:?}", scene.regions.canvas);
        assert_eq!(rect[2], canvas::NODE);
        // Its icon is drawn: it is whole. (The fourth is the panel's task item.)
        let icons = scene.draws.iter().filter(|draw| matches!(draw, Draw::Item { .. })).count();
        assert_eq!(icons, 4);
    }

    #[test]
    fn buttons_send_requests() {
        let book = book();
        let mut progress = Progress::default();
        let mut view = opened(&book, &progress);
        click(&mut view, &book, &progress, &Action::Quest(0));
        let check = Action::Check("a/start/ok".into());
        assert_eq!(
            click(&mut view, &book, &progress, &check),
            Reaction::Send(ToServer::Check { task: "a/start/ok".into() })
        );

        // The server did it: the quest is completed, its reward waits.
        progress = progress_after(&book, &[QuestEvent::Check { task: "a/start/ok".into() }]);
        let scene = view.scene(&screen(&book, &progress, NOWHERE));
        assert_eq!(zone(&scene, &check), None);
        let shown = texts(&scene);
        for text in ["Rewards waiting", "[x] Confirm", "Claim", "Claim all (1)", "1/3"] {
            assert!(shown.contains(&text), "{text} in {shown:?}");
        }
        assert_eq!(
            click(&mut view, &book, &progress, &Action::Claim("a/start/xp".into())),
            Reaction::Send(ToServer::Claim { reward: "a/start/xp".into(), choice: None })
        );
        assert_eq!(click(&mut view, &book, &progress, &Action::ClaimAll), Reaction::Send(ToServer::ClaimAll));

        // Claimed: no more buttons for it.
        progress = progress_after(
            &book,
            &[QuestEvent::Check { task: "a/start/ok".into() }, QuestEvent::ClaimAll],
        );
        let scene = view.scene(&screen(&book, &progress, NOWHERE));
        assert!(texts(&scene).contains(&"5 XP (claimed)"));
        assert_eq!(zone(&scene, &Action::ClaimAll), None);

        // The next quest is open now, and takes a hand-in. It sits under the
        // panel: close the panel to reach it.
        click(&mut view, &book, &progress, &Action::Deselect);
        click(&mut view, &book, &progress, &Action::Quest(1));
        let scene = view.scene(&screen(&book, &progress, NOWHERE));
        // Too long for one row with its button: the text wraps, the button goes below.
        let shown = texts(&scene);
        for text in ["[ ] Hand in 4x Oak", "planks 0/4", "Submit"] {
            assert!(shown.contains(&text), "{text} in {shown:?}");
        }
        assert_eq!(
            click(&mut view, &book, &progress, &Action::Submit("a/next/planks".into())),
            Reaction::Send(ToServer::Submit { task: "a/next/planks".into() })
        );
        assert_eq!(click(&mut view, &book, &progress, &Action::Close), Reaction::Close);
    }

    #[test]
    fn a_quest_is_tracked_and_untracked_from_its_panel() {
        let (book, progress) = (book(), Progress::default());
        let mut view = opened(&book, &progress);
        click(&mut view, &book, &progress, &Action::Quest(0));
        let track = Action::Track("a/start".into());
        assert_eq!(click(&mut view, &book, &progress, &track), Reaction::Tracked(Some("a/start".into())));
        let scene = view.scene(&screen(&book, &progress, NOWHERE));
        assert!(texts(&scene).contains(&"Untrack"));
        assert_eq!(click(&mut view, &book, &progress, &Action::Untrack), Reaction::Tracked(None));
        assert_eq!(view.tracked, None);
    }

    #[test]
    fn the_party_shows_in_the_header() {
        let (book, progress) = (book(), Progress::default());
        let mut view = opened(&book, &progress);
        assert!(!texts(&view.scene(&screen(&book, &progress, NOWHERE))).iter().any(|t| t.starts_with("Party")));
        view.party = Some("Miners".into());
        assert!(texts(&view.scene(&screen(&book, &progress, NOWHERE))).contains(&"Party Miners"));
    }

    #[test]
    fn an_item_row_opens_its_recipes_and_the_view_turns_and_closes() {
        let (book, progress) = (book(), progress_after(&book(), &[QuestEvent::Check { task: "a/start/ok".into() }]));
        let mut view = opened(&book, &progress);
        click(&mut view, &book, &progress, &Action::Quest(1));
        let show = Action::ShowRecipe("minecraft:oak_planks".into());
        assert!(zone(&view.scene(&screen(&book, &progress, NOWHERE)), &show).is_some());
        assert_eq!(click(&mut view, &book, &progress, &show), Reaction::Nothing);
        assert_eq!(view.recipe_wanted.take().as_deref(), Some("minecraft:oak_planks"));

        // The glue found two recipes.
        let planks = |log: &str| RecipeView {
            station: "Crafting table".into(),
            width: 1,
            slots: vec![vec![log.to_owned(), "minecraft:birch_log".into()]],
            result: "minecraft:oak_planks".into(),
            count: 4,
        };
        view.recipe = Some(RecipeOverlay { title: "Oak Planks".into(), recipes: vec![planks("minecraft:oak_log"), planks("minecraft:oak_wood")], page: 0, clock: 0 });
        let scene = view.scene(&screen(&book, &progress, NOWHERE));
        let shown = texts(&scene);
        for text in ["Recipes: Oak Planks", "1/2", "->", "4", "Crafting table"] {
            assert!(shown.contains(&text), "{text} in {shown:?}");
        }
        let items = |scene: &Scene| -> Vec<String> {
            scene.draws.iter().filter_map(|d| match d { Draw::Item { item, size, .. } if *size == 16.0 => Some(item.clone()), _ => None }).collect()
        };
        assert_eq!(items(&scene), ["minecraft:oak_log", "minecraft:oak_planks"]);
        // A slot with two items shows the second one a second later.
        view.recipe.as_mut().unwrap().clock = 20;
        assert_eq!(items(&view.scene(&screen(&book, &progress, NOWHERE)))[0], "minecraft:birch_log");
        // Turning goes round; the view keeps clicks on it, and closes on one outside.
        click(&mut view, &book, &progress, &Action::TurnRecipe(1));
        assert_eq!(view.recipe.as_ref().unwrap().page, 1);
        click(&mut view, &book, &progress, &Action::TurnRecipe(1));
        assert_eq!(view.recipe.as_ref().unwrap().page, 0);
        let scene = view.scene(&screen(&book, &progress, NOWHERE));
        // Under the view, the book's own zones do not take clicks.
        assert_eq!(scene.action_at(canvas::centre(RECT)), Some(&Action::KeepRecipe));
        assert_eq!(scene.action_at([RECT[0] + 1.0, RECT[1] + 1.0]), Some(&Action::CloseRecipe));
        click(&mut view, &book, &progress, &Action::CloseRecipe);
        assert!(view.recipe.is_none());

        // Nothing makes it: the view says so.
        view.recipe = Some(RecipeOverlay { title: "Bedrock".into(), ..RecipeOverlay::default() });
        assert!(texts(&view.scene(&screen(&book, &progress, NOWHERE))).contains(&"No recipe makes this item."));
    }

    #[test]
    fn hovering_an_item_row_keeps_its_icon_on_top() {
        let (book, progress) = (book(), progress_after(&book(), &[QuestEvent::Check { task: "a/start/ok".into() }]));
        let mut view = opened(&book, &progress);
        click(&mut view, &book, &progress, &Action::Quest(1));
        let show = Action::ShowRecipe("minecraft:oak_planks".into());
        let over = zone(&view.scene(&screen(&book, &progress, NOWHERE)), &show).unwrap();
        let scene = view.scene(&screen(&book, &progress, over));
        let shade = scene.draws.iter().position(|d| matches!(d, Draw::Fill { colour, .. } if *colour == ROW_HOVER)).unwrap();
        let icon = scene
            .draws
            .iter()
            .position(|d| matches!(d, Draw::Item { item, size, .. } if item == "minecraft:oak_planks" && *size == 9.0))
            .unwrap();
        assert!(shade < icon, "the hover shade ({shade}) is drawn before the icon ({icon})");
    }

    #[test]
    fn the_tracked_quest_box_lists_its_tasks_then_what_waits() {
        let book = book();
        let measure = |text: &str| text.len() as f32 * 5.0;
        let lines = |progress: &Progress, quest: &str| -> Vec<String> {
            tracked_draws(&book, progress, "steve", quest, [2.0, 2.0], &measure)
                .into_iter()
                .filter_map(|d| match d { Draw::Text { text, .. } => Some(text), _ => None })
                .collect()
        };
        let fresh = Progress::default();
        assert_eq!(lines(&fresh, "a/start"), ["Start here", "[ ] Confirm"]);
        assert_eq!(lines(&fresh, "a/nope"), Vec::<String>::new());
        assert!(tracked_size(&book, &fresh, "steve", "a/nope", &measure).is_none());
        let done = progress_after(&book, &[QuestEvent::Check { task: "a/start/ok".into() }]);
        assert_eq!(lines(&done, "a/start"), ["Start here", "Done: rewards in the quest book"]);
        assert_eq!(lines(&done, "a/next"), ["Then this", "[ ] Hand in 4x Oak planks 0/4"]);
        let size = tracked_size(&book, &done, "steve", "a/next", &measure).unwrap();
        assert!(size[0] <= TRACKED_WIDTH && size[1] > 0.0);
    }

    #[test]
    fn a_locked_quest_says_what_it_needs_and_has_no_buttons() {
        let (book, progress) = (book(), Progress::default());
        let mut view = opened(&book, &progress);
        click(&mut view, &book, &progress, &Action::Quest(1));
        let scene = view.scene(&screen(&book, &progress, NOWHERE));
        let shown = texts(&scene);
        for text in ["Locked", "Needs:", "[ ] Start here"] {
            assert!(shown.contains(&text), "{text} in {shown:?}");
        }
        assert_eq!(zone(&scene, &Action::Submit("a/next/planks".into())), None);
    }

    #[test]
    fn turning_to_another_chapter_closes_the_panel() {
        let (book, progress) = (book(), Progress::default());
        let mut view = opened(&book, &progress);
        click(&mut view, &book, &progress, &Action::Quest(0));
        assert_eq!(click(&mut view, &book, &progress, &Action::Chapter(1)), Reaction::Nothing);
        assert_eq!((view.chapter, view.selected), (1, None));
        let scene = view.scene(&screen(&book, &progress, NOWHERE));
        assert_eq!(scene.regions.panel, None);
        assert!(zone(&scene, &Action::Quest(3)).is_some());
        assert_eq!(zone(&scene, &Action::Quest(0)), None);
        // A lone quest sits in the middle of the canvas, not zoomed in.
        let middle = canvas::centre(scene.regions.canvas);
        let at = zone(&scene, &Action::Quest(3)).unwrap();
        assert!((at[0] - middle[0]).abs() < 0.01 && (at[1] - middle[1]).abs() < 0.01);
        assert_eq!(view.view.zoom, 1.0);
    }

    #[test]
    fn a_wide_chapter_is_zoomed_out_to_fit() {
        let wide = r#"{ "id": "w", "title": "Wide", "quests": [
            { "id": "left", "title": "t", "x": -15, "y": -5 }, { "id": "right", "title": "t", "x": 15, "y": 5 } ] }"#;
        let loaded = load(None, &[("w.json", wide)]);
        let (book, progress) = (Book::new(loaded.file), Progress::default());
        let view = opened(&book, &progress);
        assert!(view.view.zoom < 1.0 && view.view.zoom >= canvas::MIN_ZOOM);
        let scene = view.scene(&screen(&book, &progress, NOWHERE));
        for n in [0, 1] {
            let (rect, _) = scene.zones.iter().find(|(_, action)| *action == Action::Quest(n)).unwrap();
            assert!(contains(scene.regions.canvas, *rect), "{n}: {rect:?}");
        }
    }

    #[test]
    fn the_wheel_zooms_the_canvas_and_scrolls_the_lists() {
        let (book, progress) = (book(), Progress::default());
        let mut view = opened(&book, &progress);
        let regions = view.scene(&screen(&book, &progress, NOWHERE)).regions;

        let zoom = view.view.zoom;
        view.scroll(&screen(&book, &progress, canvas::centre(regions.canvas)), 1.0);
        assert!(view.view.zoom > zoom);
        view.scroll(&screen(&book, &progress, canvas::centre(regions.canvas)), f64::NAN);
        view.scroll(&screen(&book, &progress, NOWHERE), 5.0);
        assert!((view.view.zoom - zoom * canvas::ZOOM_STEP).abs() < 1e-4);

        // Three rows fit: the chapter list does not scroll.
        view.scroll(&screen(&book, &progress, canvas::centre(regions.sidebar)), -3.0);
        assert_eq!(view.sidebar_scroll, 0.0);

        // A short window: the panel's rows do not all fit, and scroll as far as the last.
        let short = |mouse| Screen { rect: [0.0, 0.0, 480.0, 110.0], ..screen(&book, &progress, mouse) };
        view.show_chapter(&short(NOWHERE), 0);
        let at = zone(&view.scene(&short(NOWHERE)), &Action::Quest(0)).unwrap();
        view.click(&short(at));
        let scene = view.scene(&short(NOWHERE));
        assert!(scene.panel_max_scroll > 0.0);
        assert!(!texts(&scene).contains(&"5 XP"));
        let over_panel = canvas::centre(scene.regions.panel.unwrap());
        view.scroll(&short(over_panel), -100.0);
        assert_eq!(view.panel_scroll, scene.panel_max_scroll);
        assert!(texts(&view.scene(&short(NOWHERE))).contains(&"5 XP"));
        view.scroll(&short(over_panel), 100.0);
        assert_eq!(view.panel_scroll, 0.0);
    }

    #[test]
    fn dragging_pans_only_over_the_canvas() {
        let (book, progress) = (book(), Progress::default());
        let mut view = opened(&book, &progress);
        let regions = view.scene(&screen(&book, &progress, NOWHERE)).regions;
        let pan = view.view.pan;
        view.drag(&screen(&book, &progress, canvas::centre(regions.sidebar)), [10.0, 10.0]);
        assert_eq!(view.view.pan, pan);
        view.drag(&screen(&book, &progress, canvas::centre(regions.canvas)), [10.0, 0.0]);
        assert!(view.view.pan[0] < pan[0]);
    }

    #[test]
    fn nodes_out_of_the_canvas_are_not_drawn() {
        let (book, progress) = (book(), Progress::default());
        let mut view = opened(&book, &progress);
        view.view.pan = [10_000.0, 0.0];
        let scene = view.scene(&screen(&book, &progress, NOWHERE));
        assert_eq!(zone(&scene, &Action::Quest(0)), None);
        // Only the chapter's icon in the list is left.
        assert_eq!(scene.draws.iter().filter(|draw| matches!(draw, Draw::Item { .. })).count(), 1);
        assert!(!scene.draws.iter().any(|draw| matches!(draw, Draw::Quad { .. })));
    }

    #[test]
    fn a_node_on_the_canvas_edge_keeps_its_box_but_not_its_icon() {
        let (book, progress) = (book(), Progress::default());
        let mut view = opened(&book, &progress);
        let scene = view.scene(&screen(&book, &progress, NOWHERE));
        let area = scene.regions.canvas;
        // Slide the first quest's centre onto the canvas's left edge.
        let at = zone(&scene, &Action::Quest(0)).unwrap();
        view.view.drag([area[0] - at[0], 0.0]);

        let scene = view.scene(&screen(&book, &progress, NOWHERE));
        let (rect, _) = scene.zones.iter().find(|(_, action)| *action == Action::Quest(0)).unwrap();
        // Half the node takes clicks; the chapter list, drawn after, covers the rest.
        assert!((rect[0] - area[0]).abs() < 0.01 && (rect[2] - canvas::NODE / 2.0).abs() < 0.01, "{rect:?}");
        let icons = scene.draws.iter().filter(|draw| matches!(draw, Draw::Item { .. })).count();
        assert_eq!(icons, 2);
        let sidebar_fill = scene.draws.iter().position(|draw| *draw == Draw::Fill { rect: scene.regions.sidebar, colour: SIDEBAR });
        let last_quad = scene.draws.iter().rposition(|draw| matches!(draw, Draw::Quad { .. }));
        assert!(sidebar_fill > last_quad);
        // The pointer over the covered half is over the chapter list, not the quest.
        assert_eq!(scene.action_at([area[0] - 3.0, at[1]]), None);
    }

    #[test]
    fn a_hidden_quest_cannot_stay_selected() {
        let (book, progress) = (book(), Progress::default());
        let mut view = opened(&book, &progress);
        // `far` (node 2) is hidden: no panel for it, even if something selected it.
        view.selected = Some(2);
        assert_eq!(view.scene(&screen(&book, &progress, NOWHERE)).regions.panel, None);
        view.selected = Some(99);
        assert_eq!(view.scene(&screen(&book, &progress, NOWHERE)).regions.panel, None);
        view.open(&screen(&book, &progress, NOWHERE));
        assert_eq!(view.selected, None);
    }

    #[test]
    fn an_empty_book_draws_its_frame() {
        let (book, progress) = (Book::new(Default::default()), Progress::default());
        let mut view = opened(&book, &progress);
        let scene = view.scene(&screen(&book, &progress, NOWHERE));
        assert_eq!(scene.zones.len(), 1);
        assert_eq!(view.click(&screen(&book, &progress, canvas::centre(scene.regions.canvas))), Reaction::Nothing);
        view.scroll(&screen(&book, &progress, canvas::centre(scene.regions.canvas)), 1.0);
    }

    #[test]
    fn the_footer_shows_the_notice() {
        let (book, progress) = (book(), Progress::default());
        let mut view = opened(&book, &progress);
        view.notice = "That quest is not open yet.".into();
        let shown = view.scene(&screen(&book, &progress, NOWHERE));
        assert!(texts(&shown).contains(&"That quest is not open yet."));
        assert!(texts(&shown).contains(&"drag: move | wheel: zoom | Esc: close"));
    }

    #[test]
    fn an_empty_pack_says_so_and_clicks_do_nothing() {
        let book = Book::new(load(None, &[]).file);
        let progress = Progress::default();
        let mut view = opened(&book, &progress);
        let shown = view.scene(&screen(&book, &progress, NOWHERE));
        assert!(texts(&shown).contains(&NO_QUESTS));
        let centre = canvas::centre(regions(RECT, false).canvas);
        assert_eq!(view.click(&screen(&book, &progress, centre)), Reaction::Nothing);
        view.drag(&screen(&book, &progress, centre), [5.0, 5.0]);
        view.scroll(&screen(&book, &progress, centre), 1.0);
    }

    /// One quest with everything the later versions added: optional,
    /// repeatable, tasks in order, a random and a choice reward.
    const GAMMA: &str = r#"{ "id": "g", "title": "Gamma", "quests": [
        { "id": "bake", "title": "Bake", "optional": true, "repeatable": true, "repeat_cooldown": 60,
          "sequential_tasks": true,
          "tasks": [ { "id": "bread", "type": "craft", "item": "bread", "count": 3 },
                     { "id": "ok", "type": "checkmark" } ],
          "rewards": [ { "id": "loot", "type": "random", "table": "loot" },
                       { "id": "pick", "type": "choice", "table": "picks" } ] } ] }"#;
    const LOOT: &str = r#"{ "id": "loot", "entries": [
        { "weight": 3, "type": "item", "item": "cookie", "count": 4 },
        { "title": "A cake!", "type": "item", "item": "cake" } ] }"#;
    const PICKS: &str = r#"{ "id": "picks", "entries": [
        { "type": "item", "item": "iron_ingot", "count": 4 },
        { "type": "xp_levels", "levels": 2 } ] }"#;

    fn gamma() -> Book {
        let loaded = super::super::hq_validate::load_pack(
            None,
            &[("g.json", GAMMA)],
            &[("loot.json", LOOT), ("picks.json", PICKS)],
        );
        assert_eq!(loaded.problems, []);
        Book::new(loaded.file)
    }

    #[test]
    fn labels_of_the_later_tasks_and_rewards() {
        let task = |json: &str| task_label(&serde_json::from_str::<Task>(json).unwrap());
        assert_eq!(task(r#"{ "type": "craft", "item": "minecraft:bread", "count": 3 }"#), "Craft 3x Bread");
        assert_eq!(task(r##"{ "type": "craft", "item": "#minecraft:planks" }"##), "Craft Planks (any)");
        assert_eq!(task(r#"{ "type": "xp_levels", "levels": 5 }"#), "Reach level 5");
        assert_eq!(task(r#"{ "type": "dimension", "dimension": "minecraft:the_nether" }"#), "Go to: The nether");
        // A box is named by its middle.
        assert_eq!(task(r#"{ "type": "location", "from": [-16, 60, -16], "to": [16, 70, 17] }"#), "Go near 0, 65, 0");
        assert_eq!(
            task(r#"{ "type": "location", "dimension": "minecraft:overworld", "from": [10, 0, 10], "to": [20, 0, 20] }"#),
            "Go near 15, 0, 15 (Overworld)"
        );
        assert_eq!(
            task(r#"{ "type": "advancement", "advancement": "minecraft:story/mine_stone" }"#),
            "Advancement: Mine stone"
        );
        assert_eq!(task(r#"{ "type": "xp_levels", "levels": 5, "title": "Get wiser" }"#), "Get wiser");

        let reward = |json: &str| reward_label(&serde_json::from_str::<Reward>(json).unwrap());
        assert_eq!(reward(r#"{ "type": "command", "command": "say hi" }"#), "A special reward");
        assert_eq!(reward(r#"{ "type": "command", "title": "A minute of speed", "command": "x" }"#), "A minute of speed");
        assert_eq!(reward(r#"{ "type": "toast", "text": "Mind the lava." }"#), "Mind the lava.");
        assert_eq!(reward(r#"{ "type": "toast" }"#), "A message");
        assert_eq!(reward(r#"{ "type": "random", "table": "baker_loot" }"#), "Random: Baker loot");
        assert_eq!(reward(r#"{ "type": "choice", "table": "picks" }"#), "Your choice: Picks");

        let book = gamma();
        let entries: Vec<String> = book.table("loot").unwrap().entries.iter().map(entry_label).collect();
        assert_eq!(entries, ["4x Cookie", "A cake!"]);
        // A craft quest shows what it makes.
        assert_eq!(quest_icon(book.quest(0)), "minecraft:bread");
    }

    #[test]
    fn the_panel_shows_order_repeats_and_what_a_table_holds() {
        let book = gamma();
        let mut progress = Progress::default();
        let mut view = opened(&book, &progress);
        click(&mut view, &book, &progress, &Action::Quest(0));
        let check = Action::Check("g/bake/ok".into());
        let claim = Action::Claim("g/bake/loot".into());
        let pick = |entry| Action::Choose("g/bake/pick".into(), entry);

        let scene = view.scene(&screen(&book, &progress, NOWHERE));
        let shown = texts(&scene);
        for text in ["Optional", "Repeatable", "Tasks, in order", "Random: Loot", "- 4x Cookie", "- A cake!"] {
            assert!(shown.contains(&text), "{text} in {shown:?}");
        }
        for text in ["Your choice: Picks", "- 4x Iron ingot", "- 2 levels"] {
            assert!(shown.contains(&text), "{text} in {shown:?}");
        }
        assert!(shown.iter().any(|text| text.contains("Craft 3x Bread")), "{shown:?}");
        // The checkmark waits for the bread, and nothing is claimed before the quest is done.
        assert_eq!(zone(&scene, &check), None);
        assert_eq!((zone(&scene, &claim), zone(&scene, &pick(0))), (None, None));

        // The bread is baked: the checkmark can be ticked.
        progress = progress_after(&book, &[QuestEvent::Crafted { task: "g/bake/bread".into(), count: 3 }]);
        assert_eq!(
            click(&mut view, &book, &progress, &check),
            Reaction::Send(ToServer::Check { task: "g/bake/ok".into() })
        );

        // Completed: the random reward is claimed as a whole, the choice entry by entry.
        progress = progress_after(
            &book,
            &[QuestEvent::Crafted { task: "g/bake/bread".into(), count: 3 }, QuestEvent::Check { task: "g/bake/ok".into() }],
        );
        let scene = view.scene(&screen(&book, &progress, NOWHERE));
        assert_eq!(zone(&scene, &Action::Claim("g/bake/pick".into())), None);
        assert_eq!(
            click(&mut view, &book, &progress, &claim),
            Reaction::Send(ToServer::Claim { reward: "g/bake/loot".into(), choice: None })
        );
        assert_eq!(
            click(&mut view, &book, &progress, &pick(1)),
            Reaction::Send(ToServer::Claim { reward: "g/bake/pick".into(), choice: Some(1) })
        );

        // Started over: the panel says how often it was done.
        let mut again = Progress::default();
        again.repeats.insert("g/bake".into(), 1);
        assert!(texts(&view.scene(&screen(&book, &again, NOWHERE))).contains(&"Repeatable, done once"));
        again.repeats.insert("g/bake".into(), 3);
        assert!(texts(&view.scene(&screen(&book, &again, NOWHERE))).contains(&"Repeatable, done 3 times"));
    }

    #[test]
    fn toasts_show_one_at_a_time_for_a_while() {
        let toast = |title: &str| Toast { title: title.into(), text: String::new(), icon: None };
        let mut toasts = Toasts::default();
        toasts.tick();
        assert!(toasts.current().is_none());
        toasts.push(toast("one"));
        toasts.push(toast("two"));
        for _ in 0..TOAST_TICKS - 1 {
            toasts.tick();
        }
        assert_eq!(toasts.current().unwrap().title, "one");
        toasts.tick();
        assert_eq!(toasts.current().unwrap().title, "two");
        for _ in 0..TOAST_TICKS {
            toasts.tick();
        }
        assert!(toasts.current().is_none());

        // A flood is cut short: the first ones stay, the rest is dropped.
        for i in 0..100 {
            toasts.push(toast(&i.to_string()));
        }
        let mut seen = Vec::new();
        while let Some(current) = toasts.current() {
            seen.push(current.title.clone());
            for _ in 0..TOAST_TICKS {
                toasts.tick();
            }
        }
        assert_eq!(seen, ["0", "1", "2", "3", "4", "5", "6", "7"]);

        // The player's settings make them stay longer.
        toasts.set_stay(240);
        toasts.push(toast("long"));
        for _ in 0..239 {
            toasts.tick();
        }
        assert_eq!(toasts.current().unwrap().title, "long");
        toasts.tick();
        assert!(toasts.current().is_none());
    }

    #[test]
    fn a_toast_is_a_box_with_its_lines_and_its_icon() {
        let toast =
            Toast { title: "Welcome".into(), text: "Mind the lava.".into(), icon: Some("minecraft:obsidian".into()) };
        // The icon, then the lines: the widest is 14 characters.
        let [w, h] = toast_size(&toast, &measure);
        assert_eq!([w, h], [PAD + 16.0 + PAD + 84.0 + PAD, 20.0 + 2.0 * PAD]);
        let draws = toast_draws(&toast, [100.0, 20.0], &measure);
        assert_eq!(draws[0], Draw::Fill { rect: [100.0, 20.0, w, h], colour: TOAST_BACK });
        let lines: Vec<(&str, [f32; 2])> = draws
            .iter()
            .filter_map(|draw| match draw {
                Draw::Text { text, at, .. } => Some((text.as_str(), *at)),
                _ => None,
            })
            .collect();
        assert_eq!(lines, [("Welcome", [124.0, 25.0]), ("Mind the lava.", [124.0, 35.0])]);
        assert!(draws.contains(&Draw::Item { item: "minecraft:obsidian".into(), at: [104.0, 24.0], size: 16.0 }));

        // A title alone, without an icon: one line.
        let plain = Toast { title: "Done".into(), text: String::new(), icon: None };
        assert_eq!(toast_size(&plain, &measure), [PAD + 24.0 + PAD, 10.0 + 2.0 * PAD]);
        assert_eq!(toast_draws(&plain, [0.0, 0.0], &measure).len(), 3);

        // A long line is shortened to the widest box.
        let long = Toast { title: "x".repeat(100), text: String::new(), icon: None };
        assert!(toast_size(&long, &measure)[0] <= TOAST_WIDTH);
        let shortened = toast_draws(&long, [0.0, 0.0], &measure);
        assert!(matches!(&shortened[2], Draw::Text { text, .. } if text.ends_with("...") && text.len() < 100));
    }

    #[test]
    fn loading_says_when_no_answer_comes() {
        assert_eq!(loading_line("", 0), "Loading quests from server...");
        assert_eq!(loading_line("", ANSWER_TIMEOUT - 1), "Loading quests from server...");
        assert!(loading_line("", ANSWER_TIMEOUT).contains("may not have Horizon Quests"));
        // A notice from the server (an incompatible version) wins at once.
        assert_eq!(loading_line("Incompatible quest mod", 0), "Incompatible quest mod");
    }
}
