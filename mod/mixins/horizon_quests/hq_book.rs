// The quest book as the player sees it: where everything goes on screen and
// what a click, a drag or the wheel does. `BookView::scene` turns the book,
// the progress and the view into a list of things to draw (fills, quads,
// text, item icons) and of zones to click; the client glue only carries the
// draws out with the game's helpers and sends the requests. Pure: no game
// types; the game's text width comes in as a function.
//
// Colours are `0xAARRGGBB` for fills and quads, `0xRRGGBB` for text. There
// is no 2D clip. The canvas is drawn first and the bands round it (chapter
// list, header, footer) are opaque and drawn after, so a node's box
// or a line that sticks out of the canvas ends up covered. Text and item
// icons are only drawn where they fit whole: icons may not be drawn in the
// same pass as the fills.
use std::collections::HashMap;
use super::hq_canvas::{self as canvas, View};
use super::hq_edit::{self as edit, Edit};
use super::hq_inline::{self as inline, Add, PanelAction, PickAction, Picker, Purpose, Subject};
use super::hq_edit::QuestInfo;
use super::hq_text::{Key, TextField};
use super::hq_logic::{Book, QuestState, Status};
use super::hq_model::{ItemMatch, Progress, Quest, Reward, RewardKind, RewardTable, Shape, TableEntry, Task, TaskKind, child_id};
use super::hq_net::ToServer;
use std::collections::VecDeque;

const SHADE: u32 = 0xD010_1018;
const BAND: u32 = 0xFF20_2030;
/// The chapter list: a panel over the canvas while the pointer is on it,
/// a thin strip with an arrow otherwise.
const LIST_BACK: u32 = 0x902A_2E36;
const LIST_EDGE: u32 = 0xFF4A_4E5A;
const LIST_HOVER: u32 = 0xFF36_3A44;
const LIST_CURRENT: u32 = 0xFF3A_3E48;
const LIST_OUTLINE: u32 = 0xFF9A_9EA8;
const LIST_ARROW: u32 = 0xFFC8_C8D0;
const STRIP_BACK: u32 = 0x9024_2830;
/// Text colours (no alpha): a group heading, a chapter's name, a chapter
/// all done, a chapter with rewards to claim.
const GROUP_INK: u32 = 0xD0_D0D8;
/// A chapter's name in the list: grey while nothing in it is started, blue
/// once something is, green once every quest is done. A red "!" at the end
/// of its row says rewards wait in it.
const CHAPTER_INK: u32 = 0xA0_A0AC;
const CHAPTER_STARTED: u32 = 0x58_A8F8;
const CHAPTER_DONE: u32 = 0x60_E060;
/// A row of the chapter list, its chapter icons, and the closed list's strip.
const LIST_ROW: f32 = 16.0;
const LIST_ICON: f32 = 12.0;
pub const STRIP: f32 = 18.0;
const PANEL: u32 = 0xFF1C_1C28;
const ROW_HOVER: u32 = 0xFF2A_2A3A;
const BUTTON: u32 = 0xFF3A_5A8A;
const BUTTON_HOVER: u32 = 0xFF4A_74B0;
const TOOLTIP: u32 = 0xF010_0418;
/// The tooltip's frame: lighter at the top, darker at the bottom.
const TOOLTIP_EDGE_TOP: u32 = 0xFF5A_2AB8;
const TOOLTIP_EDGE_BOTTOM: u32 = 0xFF2C_1460;
const NODE_FILL: u32 = 0xFF26_2632;
const LINE: u32 = 0xFF8C_7C88;
const LINE_DONE: u32 = 0xFF40_A050;
/// A line into the quest under the pointer: what it needs.
const LINE_NEEDS: u32 = 0xFF38_C8E8;
/// A line out of the quest under the pointer: what it opens.
const LINE_OPENS: u32 = 0xFFE8_C838;
/// A line into a quest that cannot be started yet: darker, half
/// see-through (in the sprite sheet).
const LINE_LOCKED: u32 = 0x8C5C_5664;
/// A dependency line is a row of thin chevrons pointing to the quest it
/// opens, with a bead between each two: a chevron every `CHEVRON_STEP`
/// canvas units (a node is `canvas::NODE`), `CHEVRON_SIZE` across, beads
/// `BEAD_SIZE` across. On the lines in focus they slide one step every
/// `FLOW_TICKS` ticks.
const CHEVRON_STEP: f32 = 8.5;
const CHEVRON_SIZE: f32 = 7.0;
const BEAD_SIZE: f32 = 2.8;
const FLOW_TICKS: u32 = 14;
/// The lines' sprite sheet (`tools/make-line-textures.py`, 512 x 352: the
/// game leaves out GUI textures over 1024 x 512): 16 cells a row; each line
/// colour (`LINE_COLOURS` order) has two rows of chevrons, one per
/// direction; the beads share the last row, one per colour.
pub const CHEVRON_SPRITE: &str = "horizon_quests:gui/chevrons";
pub const CHEVRON_CELL: f32 = 32.0;
const CHEVRON_DIRECTIONS: usize = 32;
const CHEVRON_COLUMNS: usize = 16;
const LINE_ROWS: usize = CHEVRON_DIRECTIONS / CHEVRON_COLUMNS;
const LINE_COLOURS: [u32; 5] = [LINE, LINE_DONE, LINE_NEEDS, LINE_OPENS, LINE_LOCKED];
const INK: u32 = 0xFF_FFFF;
const DIM_INK: u32 = 0xA0_A0B0;
const GOLD_INK: u32 = 0xFF_C040;

/// A line of text's height, a chapter row's, and the space left round
/// things, in GUI pixels.
const TEXT: f32 = 10.0;
const PAD: f32 = 4.0;
/// The recipe view's title bar.
const TITLE_BAR: f32 = 14.0;
/// The room left round a chapter fitted to the canvas.
const MARGIN: f32 = 16.0;
/// The most entries of a reward table a tooltip or the choice window lists.
const TABLE_ROWS: usize = 12;
/// The icon shown for a quest that names none and whose first task has none.
const DEFAULT_ICON: &str = "minecraft:book";

/// One thing to draw. In order: later ones cover earlier ones.
#[derive(Clone, Debug, PartialEq)]
pub enum Draw {
    Fill { rect: [f32; 4], colour: u32 },
    Quad { corners: [[f32; 2]; 4], colour: u32 },
    Text { text: String, at: [f32; 2], colour: u32, shadow: bool },
    /// An item's icon, `size` pixels wide, its top-left corner at `at`.
    Item { item: String, at: [f32; 2], size: f32 },
    /// Part `src` of GUI sprite `name` (a texture of the mod's content),
    /// stretched into `dst`; `fallback` is drawn instead when the game has
    /// no such sprite (the content half is not installed).
    Sprite { name: String, src: [f32; 4], dst: [f32; 4], fallback: Vec<Draw> },
}

/// What a click does. Nodes are `Book` node indexes; ids are full ids.
#[derive(Clone, Debug, PartialEq)]
pub enum Action {
    Chapter(usize),
    /// Fold or unfold a group of the chapter list (its id).
    Group(String),
    Quest(usize),
    /// Close the quest window.
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
    /// Open the window to pick an entry of this choice reward, or close it.
    OpenChoice(String),
    CloseChoice,
    /// A click on a window (the quest window, the recipe view) that does
    /// nothing: it stays.
    Stay,
    /// Edit mode (operators): turn it on or off.
    ToggleEdit,
    /// Ask the server for this change to the pack.
    Edit(Edit),
    /// Ask whether to delete this quest (full id).
    AskDelete(String),
    /// Pick, by clicking them, the quests quest `0` needs (`true`) or opens
    /// (`false`).
    Link(usize, bool),
    SaveToModpack,
    AskUseModpack,
    UseModpack,
    CloseMenu,
    /// Open a panel: a quest's settings, a task, a reward, a chapter...
    OpenPanel(Subject),
    /// A click on the open panel, on the picker.
    Panel(PanelAction),
    Pick(PickAction),
    /// Open the picker for this (`true`: mobs).
    PickFor(Purpose, bool),
    /// Start typing a quest's title, subtitle or description in its window.
    Type(Typed),
    /// A click in the text being typed: on line `0`, whose text starts at x `1`.
    Caret(usize, f32),
    /// The "add a task" and "add a reward" menus of a quest (full id).
    AddTask(String),
    AddReward(String),
    /// Ask whether to delete this chapter (id), this reward table (id).
    AskDeleteChapter(String),
    AskDeleteTable(String),
    /// The menu of the pack's reward tables, of one table's entries.
    Tables,
    OpenTable(String),
}

/// What is being typed in place.
#[derive(Clone, Debug, PartialEq)]
pub enum Typed {
    /// A quest's (full id) title, subtitle, description, in its window.
    Title(String),
    Subtitle(String),
    Description(String),
    /// A text control of the open panel.
    Panel(&'static str),
}

/// What the glue does after a click.
#[derive(Clone, Debug, PartialEq)]
pub enum Reaction {
    Nothing,
    Send(ToServer),
    Close,
    /// The tracked quest changed: keep it in the player's settings.
    Tracked(Option<String>),
    /// An edit: the glue sends it and applies it to its own copy of the
    /// pack at once (`hq_edit::apply`).
    Edit(Edit),
}

/// Edit mode's menu (a right click on the canvas): a title, then entries.
#[derive(Clone, Debug, PartialEq)]
pub struct Menu {
    pub at: [f32; 2],
    pub title: Option<String>,
    pub entries: Vec<(String, Action)>,
    /// An item drawn before each entry, when not empty.
    pub icons: Vec<String>,
}

/// A quest dragged in edit mode: where the pointer took it (grid cells from
/// its centre), where it is now (grid cells), and whether it moved yet: a
/// press and release in place opens the quest instead.
#[derive(Clone, Debug, PartialEq)]
struct Moving {
    node: usize,
    grab: [f32; 2],
    at: [f32; 2],
    pressed: [f32; 2],
    moved: bool,
}

/// The book's parts on screen, as `[x, y, w, h]`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Regions {
    /// The chapter list when open, over the canvas's left side.
    pub sidebar: [f32; 4],
    /// The closed list's strip, left of the canvas.
    pub strip: [f32; 4],
    pub canvas: [f32; 4],
    pub footer: [f32; 4],
}

/// The parts of a book filling `rect`. The quest window goes over the
/// canvas (`Scene::popup`).
pub fn regions(rect: [f32; 4]) -> Regions {
    let [x, y, w, h] = rect;
    let body_y = y;
    let body_h = (h - canvas::FOOTER).max(0.0);
    let sidebar_w = (w * 0.3).clamp(110.0, 170.0).min(w.max(0.0));
    let strip_w = STRIP.min(w.max(0.0));
    Regions {
        sidebar: [x, body_y, sidebar_w, body_h],
        strip: [x, body_y, strip_w, body_h],
        canvas: [x + strip_w, body_y, (w - strip_w).max(0.0), body_h],
        footer: [x, y + h - canvas::FOOTER, w, canvas::FOOTER],
    }
}

/// One frame's worth of book: what to draw and where clicks land.
#[derive(Clone, Debug, PartialEq)]
pub struct Scene {
    pub regions: Regions,
    pub draws: Vec<Draw>,
    /// Later zones are on top.
    pub zones: Vec<([f32; 4], Action)>,
    /// How far the chapter list and the quest window's text can scroll.
    pub sidebar_max_scroll: f32,
    pub popup_max_scroll: f32,
    /// The quest window, when a quest is selected.
    pub popup: Option<[f32; 4]>,
}

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
pub enum Row {
    /// A group's heading: its id and its title.
    Group { id: String, title: String },
    Chapter(usize),
}

/// The chapter list: chapters without a group first, then each group of
/// quests.json with its chapters, unless the group is folded (its id in
/// `folded`). Hidden chapters are not listed.
pub fn sidebar_rows(book: &Book, folded: &[String]) -> Vec<Row> {
    let file = book.file();
    let groups = &file.settings.groups;
    let group_of = |c: usize| file.chapters[c].group.as_deref().filter(|g| groups.iter().any(|group| group.id == *g));
    let listed = || (0..file.chapters.len()).filter(|&c| !file.chapters[c].hidden);
    let mut rows: Vec<Row> = listed().filter(|&c| group_of(c).is_none()).map(Row::Chapter).collect();
    for (g, group) in groups.iter().enumerate() {
        let members: Vec<usize> = listed().filter(|&c| group_of(c) == Some(group.id.as_str())).collect();
        // Two groups of the same id: the first one has the chapters.
        if !members.is_empty() && !groups[..g].iter().any(|earlier| earlier.id == group.id) {
            let title = if group.title.is_empty() { group.id.clone() } else { group.title.clone() };
            rows.push(Row::Group { id: group.id.clone(), title });
            if !folded.contains(&group.id) {
                rows.extend(members.into_iter().map(Row::Chapter));
            }
        }
    }
    rows
}

/// The corners of a shape of this radius round `centre`, turning
/// clockwise on screen from its bottom left (a square) or its right.
pub fn shape_points(shape: Shape, centre: [f32; 2], radius: f32) -> Vec<[f32; 2]> {
    let [cx, cy] = centre;
    let (sides, start) = match shape {
        Shape::Square => {
            let r = radius;
            return vec![[cx - r, cy + r], [cx + r, cy + r], [cx + r, cy - r], [cx - r, cy - r]];
        }
        Shape::Diamond => (4, 0.0),
        Shape::Hexagon => (6, 0.0),
        Shape::Circle => (16, 0.0),
    };
    (0..sides)
        .map(|k| {
            let angle = start + k as f32 * std::f32::consts::TAU / sides as f32;
            [cx + radius * angle.cos(), cy - radius * angle.sin()]
        })
        .collect()
}

/// A triangle `size` pixels across, its left (pointing right) or top
/// (pointing down) side starting at `at`'s x, centred on `at`'s y. One
/// quad, its last corner doubled, in `shape_points`' turning order.
fn arrow(at: [f32; 2], size: f32, down: bool) -> [[f32; 2]; 4] {
    let ([x, y], h) = (at, size / 2.0);
    if down {
        let top = y - h * 0.6;
        [[x + h, top + h * 1.6], [x + size, top], [x, top], [x, top]]
    } else {
        let left = x + h * 0.2;
        [[left, y + h], [left + h * 1.6, y], [left, y - h], [left, y - h]]
    }
}

/// A one-pixel frame just inside `rect`.
fn outline(scene: &mut Scene, rect: [f32; 4], colour: u32) {
    let [x, y, w, h] = rect;
    for edge in [[x, y, w, 1.0], [x, y + h - 1.0, w, 1.0], [x, y + 1.0, 1.0, h - 2.0], [x + w - 1.0, y + 1.0, 1.0, h - 2.0]] {
        scene.fill(edge, colour);
    }
}

/// The place of `colour` in the line sheet.
fn line_colour(colour: u32) -> usize {
    LINE_COLOURS.iter().position(|c| *c == colour).unwrap_or(0)
}

/// The cell of the line sheet for a chevron of `colour` going in direction
/// `dir` (a unit vector, y down): the nearest of the 32 drawn directions.
pub fn chevron_cell(colour: u32, dir: [f32; 2]) -> [f32; 4] {
    let turn = dir[1].atan2(dir[0]).rem_euclid(std::f32::consts::TAU) / std::f32::consts::TAU;
    let k = (turn * CHEVRON_DIRECTIONS as f32).round() as usize % CHEVRON_DIRECTIONS;
    let (column, row) = (k % CHEVRON_COLUMNS, line_colour(colour) * LINE_ROWS + k / CHEVRON_COLUMNS);
    [column as f32 * CHEVRON_CELL, row as f32 * CHEVRON_CELL, CHEVRON_CELL, CHEVRON_CELL]
}

/// The cell of the line sheet for a bead of `colour`.
pub fn bead_cell(colour: u32) -> [f32; 4] {
    let row = LINE_COLOURS.len() * LINE_ROWS;
    [line_colour(colour) as f32 * CHEVRON_CELL, row as f32 * CHEVRON_CELL, CHEVRON_CELL, CHEVRON_CELL]
}

/// What a line `len` long carries, `phase` (0 to 1) of a step on from rest:
/// `(distance along it, is a chevron)` in order, a chevron every `step`
/// (half a step in at rest) and a bead half way between each two.
pub fn line_marks(len: f32, step: f32, phase: f32) -> Vec<(f32, bool)> {
    let mut marks = Vec::new();
    if step <= 0.0 {
        return marks;
    }
    let half = step / 2.0;
    // Marks every half step, chevrons on the odd halves at rest.
    let shift = phase.rem_euclid(1.0) * step;
    let mut k = 0;
    loop {
        let at = shift + k as f32 * half;
        if at >= len {
            break;
        }
        if at > 0.0 {
            marks.push((at, k % 2 == 1));
        }
        k += 1;
    }
    marks
}

/// The closed chapter list's strip on the canvas's left, with an arrow
/// pointing right; the pointer on it opens the list.
fn draw_strip(scene: &mut Scene, s: &Screen<'_>) {
    let strip = scene.regions.strip;
    scene.fill(strip, if canvas::inside(strip, s.mouse) { LIST_HOVER } else { STRIP_BACK });
    scene.fill([strip[0] + strip[2] - 1.0, strip[1], 1.0, strip[3]], LIST_EDGE);
    let [cx, cy] = canvas::centre(strip);
    scene.draws.push(Draw::Quad { corners: arrow([cx - 3.0, cy], 7.0, false), colour: LIST_ARROW });
}

/// How a shape is drawn against a square of the same `size`: how much
/// bigger (a circle or a diamond inside the square's box looks smaller than
/// the square) and the share of the drawn box its icon takes (what room the
/// face leaves inside the ring, with a margin).
fn shape_look(shape: Shape) -> (f32, f32) {
    match shape {
        Shape::Square => (1.0, 0.58),
        Shape::Circle => (1.15, 0.5),
        Shape::Hexagon => (1.28, 0.46),
        Shape::Diamond => (1.5, 0.37),
    }
}

/// The share of a node's radius that is the frame's face, inside its ring
/// (`tools/make-node-textures.py`, `RINGS`: where the face starts, in units
/// of a 32-unit cell from the edge, measured square to each side).
fn face_share(shape: Shape) -> f32 {
    // The ratio of each shape's apothem to the radius `shape_points` takes.
    let apothem = match shape {
        Shape::Square | Shape::Circle => 1.0,
        Shape::Diamond => std::f32::consts::FRAC_1_SQRT_2,
        Shape::Hexagon => 0.866,
    };
    let ring = match shape {
        Shape::Square => 1.75,
        Shape::Circle => 2.3,
        Shape::Hexagon => 2.075,
        Shape::Diamond => 1.65,
    };
    1.0 - (ring / 16.0) / apothem
}

/// A shape filled with `colour`, as quads.
fn polygon_draws(shape: Shape, centre: [f32; 2], radius: f32, colour: u32) -> Vec<Draw> {
    polygon_quads(&shape_points(shape, centre, radius)).into_iter().map(|corners| Draw::Quad { corners, colour }).collect()
}

/// The node frames' sprite sheet (`tools/make-node-textures.py`): a row per
/// shape, a column per state, each cell `NODE_CELL` pixels square.
pub const NODE_SPRITE: &str = "horizon_quests:gui/nodes";
const NODE_CELL: f32 = 64.0;

/// The cell of the node sprite sheet for a node of this shape and state.
pub fn node_cell(shape: Shape, state: QuestState) -> [f32; 4] {
    let row = match shape {
        Shape::Square => 0.0,
        Shape::Circle => 1.0,
        Shape::Diamond => 2.0,
        Shape::Hexagon => 3.0,
    };
    let column = match state {
        QuestState::Locked => 0.0,
        QuestState::Available => 1.0,
        QuestState::InProgress => 2.0,
        // Rewards waiting look done; the book adds a "!" badge.
        QuestState::RewardsWaiting | QuestState::Done => 3.0,
    };
    [column * NODE_CELL, row * NODE_CELL, NODE_CELL, NODE_CELL]
}

/// A convex polygon with an even number of corners, as quads fanned from
/// its first corner.
pub fn polygon_quads(points: &[[f32; 2]]) -> Vec<[[f32; 2]; 4]> {
    (1..points.len().saturating_sub(2)).step_by(2).map(|i| [points[0], points[i], points[i + 1], points[i + 2]]).collect()
}

/// `text` cut into lines at most `width` wide, at spaces where it can.
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
pub fn pretty(id: &str) -> String {
    let path = id.rsplit(':').next().unwrap_or(id);
    let name = path.rsplit('/').next().unwrap_or(path).replace('_', " ");
    let mut chars = name.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => name,
    }
}

fn counted(count: u32, name: &str) -> String {
    if count == 1 { name.to_owned() } else { format!("{count}x {name}") }
}

/// What a task asks, in a few words: its title, or one made from its type.
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
pub fn kind_label(kind: &RewardKind) -> String {
    match kind {
        RewardKind::Item { item, count, max_count: Some(max) } if max > count => format!("{count}-{max}x {}", pretty(item)),
        RewardKind::Item { item, count, .. } => counted(*count, &pretty(item)),
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
pub fn titled(title: Option<&str>, kind: &RewardKind) -> String {
    match title.filter(|title| !title.trim().is_empty()) {
        Some(title) => title.to_owned(),
        None => kind_label(kind),
    }
}

/// What a reward gives, in a few words: its title, or one made from its
/// type.
pub fn reward_label(reward: &Reward) -> String {
    titled(reward.title.as_deref(), &reward.kind)
}

/// What a reward table's entry gives, in a few words.
pub fn entry_label(entry: &TableEntry) -> String {
    titled(entry.title.as_deref(), &entry.kind)
}

/// The item a quest's node shows.
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
pub fn state_look(state: QuestState) -> (u32, &'static str) {
    match state {
        QuestState::Locked => (0xFF50_505A, "Locked"),
        QuestState::Available => (0xFFD8_D8E0, "Available"),
        QuestState::InProgress => (0xFFE8_C040, "In progress"),
        // The done green: the quest is done, its "!" badge says what is left.
        QuestState::RewardsWaiting => (0xFF40_B050, "Rewards waiting"),
        QuestState::Done => (0xFF40_B050, "Completed"),
    }
}

/// A node's box on screen.
fn node_box(view: &View, origin: [f32; 2], place: [f32; 2], shape: Shape, size: f32) -> [f32; 4] {
    let [sx, sy] = view.to_screen(origin, canvas::cell_centre(place));
    let side = canvas::NODE * size * shape_look(shape).0 * view.zoom;
    [sx - side / 2.0, sy - side / 2.0, side, side]
}

/// The part of `a` inside `b`.
fn intersect(a: [f32; 4], b: [f32; 4]) -> Option<[f32; 4]> {
    let (x0, y0) = (a[0].max(b[0]), a[1].max(b[1]));
    let (x1, y1) = ((a[0] + a[2]).min(b[0] + b[2]), (a[1] + a[3]).min(b[1] + b[3]));
    (x1 > x0 && y1 > y0).then_some([x0, y0, x1 - x0, y1 - y0])
}

/// Whether `inner` is wholly inside `outer`.
fn contains(outer: [f32; 4], inner: [f32; 4]) -> bool {
    inner[0] >= outer[0]
        && inner[1] >= outer[1]
        && inner[0] + inner[2] <= outer[0] + outer[2]
        && inner[1] + inner[3] <= outer[1] + outer[3]
}

/// The view that shows all of a chapter's visible quests in `area`, never
/// zoomed in past 1.
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

/// What the player looks at in the book: kept by the client between frames.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BookView {
    /// The chapter shown, and how its canvas is looked at.
    pub chapter: usize,
    pub view: View,
    /// The quest whose window is open.
    pub selected: Option<usize>,
    pub sidebar_scroll: f32,
    /// Whether the chapter list is open: the pointer came onto its strip
    /// and has not left the list since (`hover`).
    pub sidebar_open: bool,
    /// The groups of the chapter list folded by the player (their ids).
    pub folded: Vec<String>,
    pub popup_scroll: f32,
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
    /// The choice reward whose entries are shown to pick from.
    pub choosing: Option<String>,
    /// Item id → its name in the player's language, from the game (the
    /// glue fills it); an item missing here is named from its id.
    pub item_names: HashMap<String, String>,
    /// Window ticks since the book was made: random and choice rewards
    /// show each of their entries in turn (`TABLE_TURN`).
    pub clock: u32,
    /// How far into the next tick this frame is (0 to 1), from the game:
    /// the lines in focus move smoothly between ticks.
    pub partial_tick: f32,
    /// Edit mode: whether the server lets this player edit, where the pack
    /// in use comes from (`ToClient::Editor`), and whether the mode is on.
    pub editor: bool,
    pub source: String,
    pub editing: bool,
    /// Edit mode's menu, while open.
    pub menu: Option<Menu>,
    /// Edit mode in place: the open panel, the picker and what it picks
    /// for, the text being typed.
    pub panel: Option<Subject>,
    pub picker: Option<(Picker, Purpose)>,
    pub typing: Option<(Typed, TextField)>,
    /// The book's rect at the last frame: where typed text wraps.
    rect: [f32; 4],
    /// The quest being dragged in edit mode.
    moving: Option<Moving>,
    /// Edit mode's link picking: a click on a quest makes quest `0` need it
    /// (`true`) or need quest `0` (`false`), or stops doing so.
    linking: Option<(usize, bool)>,
    /// Whether a chapter was fitted to the canvas yet.
    fitted: bool,
}

impl BookView {
    /// The book opens, or a new pack arrived: keep the place when there is
    /// one, else show the first chapter.
    pub fn open(&mut self, s: &Screen<'_>) {
        let listed: Vec<usize> = sidebar_rows(s.book, &[])
            .into_iter()
            .filter_map(|row| match row {
                Row::Chapter(c) => Some(c),
                Row::Group { .. } => None,
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
        self.popup_scroll = 0.0;
        self.view = fit(s.book, c, &statuses, regions(s.rect).canvas);
        self.fitted = true;
    }

    /// The selected quest, if its window shows: it must be visible and in
    /// the chapter shown.
    fn shown_selection(&self, book: &Book, statuses: &[Status]) -> Option<usize> {
        self.selected.filter(|&n| n < statuses.len() && statuses[n].visible && book.nodes()[n].chapter == self.chapter)
    }

    /// Closes the quest window: whether one was open (Escape closes it before
    /// the book).
    pub fn close_quest(&mut self) -> bool {
        let open = self.selected.is_some();
        self.select(None);
        open
    }

    /// Opens quest `n`'s window, or closes it.
    fn select(&mut self, selected: Option<usize>) {
        if self.selected != selected {
            self.selected = selected;
            self.popup_scroll = 0.0;
            // A panel of the quest that was open goes with its window.
            if self.panel.as_ref().is_some_and(|p| inline::subject_quest(p).is_some()) {
                self.panel = None;
            }
        }
    }

    /// A left click at the pointer.
    pub fn click(&mut self, s: &Screen<'_>) -> Reaction {
        let action = self.scene(s).action_at(s.mouse).cloned();
        // Link picking lasts while quests are clicked.
        if !matches!(action, Some(Action::Quest(_))) {
            self.linking = None;
        }
        // Typing ends with a click elsewhere: the text is kept, and that
        // click does nothing more.
        if self.typing.is_some() && !matches!(action, Some(Action::Caret(..))) {
            return self.finish_typing(s);
        }
        match action {
            Some(Action::Chapter(c)) => self.show_chapter(s, c),
            Some(Action::Group(id)) => {
                if let Some(i) = self.folded.iter().position(|g| *g == id) {
                    self.folded.remove(i);
                } else {
                    self.folded.push(id);
                }
            }
            Some(Action::Quest(n)) if self.linking.is_some_and(|(from, _)| from != n) => {
                let Some((from, needs)) = self.linking else {
                    return Reaction::Nothing;
                };
                let (quest, other) = if needs { (from, n) } else { (n, from) };
                let on = !s.book.nodes()[quest].dependencies.contains(&other);
                let (quest, needs) = (s.book.nodes()[quest].id.clone(), s.book.nodes()[other].id.clone());
                return Reaction::Edit(Edit::Link { quest, needs, on });
            }
            Some(Action::Quest(_)) if self.linking.is_some() => {}
            Some(Action::Quest(n)) if self.editing => {
                // Taken by the pointer: a drag moves it, a release in place opens it.
                let quest = s.book.quest(n);
                let at = self.cell_under(s);
                let grab = [at[0] - quest.x, at[1] - quest.y];
                self.moving = Some(Moving { node: n, grab, at: [quest.x, quest.y], pressed: s.mouse, moved: false });
            }
            Some(Action::Quest(n)) => self.select(Some(n)),
            Some(Action::Deselect) => self.select(None),
            Some(Action::Check(task)) => return Reaction::Send(ToServer::Check { task }),
            Some(Action::Submit(task)) => return Reaction::Send(ToServer::Submit { task }),
            Some(Action::Claim(reward)) => return Reaction::Send(ToServer::Claim { reward, choice: None }),
            Some(Action::Choose(reward, entry)) => {
                self.choosing = None;
                return Reaction::Send(ToServer::Claim { reward, choice: Some(entry) });
            }
            Some(Action::OpenChoice(reward)) => self.choosing = Some(reward),
            Some(Action::CloseChoice) => self.choosing = None,
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
            Some(Action::ToggleEdit) => {
                self.editing = self.editor && !self.editing;
                self.menu = None;
                self.close_editors();
                self.moving = None;
                self.linking = None;
            }
            Some(Action::Link(n, needs)) => {
                self.menu = None;
                self.select(None);
                self.linking = Some((n, needs));
            }
            Some(Action::Edit(change)) => {
                self.menu = None;
                return Reaction::Edit(change);
            }
            Some(Action::AskDelete(quest)) => {
                let title = s.book.find(&quest).map_or_else(|| quest.clone(), |n| quest_title(s.book.quest(n)).to_owned());
                let entries = vec![("Delete".to_owned(), Action::Edit(Edit::DeleteQuest { quest })), ("Cancel".to_owned(), Action::CloseMenu)];
                self.menu = Some(Menu { at: s.mouse, title: Some(format!("Delete \"{title}\"?")), entries, icons: Vec::new() });
            }
            Some(Action::SaveToModpack) => {
                self.menu = None;
                return Reaction::Send(ToServer::SaveToModpack);
            }
            Some(Action::AskUseModpack) => {
                let entries = vec![("Use the modpack's quests".to_owned(), Action::UseModpack), ("Cancel".to_owned(), Action::CloseMenu)];
                let title = "Put this world's quests aside?".to_owned();
                self.menu = Some(Menu { at: s.mouse, title: Some(title), entries, icons: Vec::new() });
            }
            Some(Action::UseModpack) => {
                self.menu = None;
                return Reaction::Send(ToServer::UseModpack);
            }
            Some(Action::CloseMenu) => self.menu = None,
            Some(Action::OpenPanel(subject)) => {
                self.menu = None;
                self.typing = None;
                self.panel = Some(subject);
            }
            Some(Action::Panel(action)) => return self.panel_click(s, action),
            Some(Action::Pick(PickAction::Choose(id))) => {
                if let Some((_, purpose)) = self.picker.take()
                    && let Some(change) = inline::picked(s.book, &purpose, self.panel.as_ref(), &id)
                {
                    return Reaction::Edit(change);
                }
            }
            Some(Action::Pick(PickAction::Close)) => self.picker = None,
            Some(Action::Pick(_)) => {}
            Some(Action::PickFor(purpose, mobs)) => {
                self.menu = None;
                let tags = matches!(&purpose, Purpose::NewTask { kind, .. } if *kind != inline::NewTask::Kill);
                self.picker = Some((Picker::new(mobs, tags), purpose));
            }
            Some(Action::Type(target)) => {
                let field = match &target {
                    Typed::Title(q) => TextField::new(&s.book.find(q).map_or(String::new(), |n| s.book.quest(n).title.clone()), 100),
                    Typed::Subtitle(q) => {
                        TextField::new(&s.book.find(q).and_then(|n| s.book.quest(n).subtitle.clone()).unwrap_or_default(), 150)
                    }
                    Typed::Description(q) => TextField::lines(&s.book.find(q).map_or(String::new(), |n| s.book.quest(n).description.join("\n")), 4000),
                    Typed::Panel(key) => match &self.panel {
                        Some(subject) => TextField::new(&inline::subject_value(s.book, subject, key), 300),
                        None => return Reaction::Nothing,
                    },
                };
                self.typing = Some((target, field));
            }
            Some(Action::Caret(line, left)) => {
                let text_w = popup_text_width(s.rect);
                if let Some((target, field)) = &mut self.typing {
                    let x = s.mouse[0] - left;
                    if matches!(target, Typed::Description(_)) {
                        let lines = field.wrapped(text_w, s.measure);
                        field.place_in(&lines, line, x, s.measure);
                    } else {
                        let scroll = field.shown(0, text_w, s.measure).1;
                        field.click(0, x, scroll, s.measure);
                    }
                }
            }
            Some(Action::AddTask(quest)) => self.add_menu(s, "Add a task", inline::add_task_menu(&quest)),
            Some(Action::AddReward(quest)) => self.add_menu(s, "Add a reward", inline::add_reward_menu(s.book, &quest)),
            Some(Action::AskDeleteChapter(id)) => {
                let name = s.book.file().chapters.iter().find(|c| c.id == id).map_or(id.clone(), |c| c.title.clone());
                let entries = vec![("Delete it and its quests".to_owned(), Action::Edit(Edit::DeleteChapter { chapter: id })), ("Cancel".to_owned(), Action::CloseMenu)];
                self.menu = Some(Menu { at: s.mouse, title: Some(format!("Delete \"{name}\"?")), entries, icons: Vec::new() });
            }
            Some(Action::AskDeleteTable(id)) => {
                let entries = vec![("Delete the table".to_owned(), Action::Edit(Edit::DeleteTable { table: id.clone() })), ("Cancel".to_owned(), Action::CloseMenu)];
                self.menu = Some(Menu { at: s.mouse, title: Some(format!("Delete table {id}?")), entries, icons: Vec::new() });
            }
            Some(Action::Tables) => {
                let mut entries: Vec<(String, Action)> = s
                    .book
                    .file()
                    .reward_tables
                    .iter()
                    .map(|t| (format!("{} ({} entries)", t.id, t.entries.len()), Action::OpenTable(t.id.clone())))
                    .collect();
                entries.push(("+ New table".to_owned(), Action::Edit(Edit::NewTable)));
                self.menu = Some(Menu { at: s.mouse, title: Some("Reward tables".to_owned()), entries, icons: Vec::new() });
            }
            Some(Action::OpenTable(id)) => {
                if let Some(table) = s.book.table(&id) {
                    let mut entries: Vec<(String, Action)> = table
                        .entries
                        .iter()
                        .enumerate()
                        .map(|(i, e)| (format!("{} (weight {})", entry_label(e), e.weight), Action::OpenPanel(Subject::Entry(id.clone(), i))))
                        .collect();
                    let bread = TableEntry { weight: 1, title: None, kind: RewardKind::Item { item: "minecraft:bread".into(), count: 1, max_count: None } };
                    entries.push(("+ Add an entry".to_owned(), Action::Edit(Edit::SaveEntry { table: id.clone(), index: None, entry: bread })));
                    entries.push(("Delete this table".to_owned(), Action::AskDeleteTable(id.clone())));
                    self.menu = Some(Menu { at: s.mouse, title: Some(format!("Table {id}")), entries, icons: Vec::new() });
                }
            }
            Some(Action::Stay) | None => {}
        }
        Reaction::Nothing
    }

    /// The left button came up: a quest dragged in edit mode is dropped on
    /// the nearest half cell, one pressed and released in place opens.
    pub fn release(&mut self, s: &Screen<'_>) -> Reaction {
        let Some(moving) = self.moving.take() else {
            return Reaction::Nothing;
        };
        if moving.node >= s.book.nodes().len() {
            return Reaction::Nothing;
        }
        if !moving.moved {
            self.select(Some(moving.node));
            return Reaction::Nothing;
        }
        let quest = s.book.quest(moving.node);
        let [x, y] = edit::snap(moving.at);
        if (x, y) == (quest.x, quest.y) {
            return Reaction::Nothing;
        }
        Reaction::Edit(Edit::MoveQuest { quest: s.book.nodes()[moving.node].id.clone(), x, y })
    }

    /// A right click: in edit mode, the menu for the quest under the
    /// pointer, or for the empty spot of the canvas.
    pub fn right_click(&mut self, s: &Screen<'_>) -> Reaction {
        if !self.editing || self.menu.take().is_some() || self.recipe.is_some() || self.choosing.is_some() || self.picker.is_some() || self.typing.is_some() {
            return Reaction::Nothing;
        }
        let scene = self.scene(s);
        if self.over_list(s) {
            let menu = match scene.action_at(s.mouse) {
                Some(Action::Chapter(c)) => {
                    let chapter = s.book.chapter(*c);
                    let id = chapter.id.clone();
                    let entries = vec![
                        ("Edit...".to_owned(), Action::OpenPanel(Subject::Chapter(id.clone()))),
                        ("Move up".to_owned(), Action::Edit(Edit::MoveChapter { chapter: id.clone(), by: -1 })),
                        ("Move down".to_owned(), Action::Edit(Edit::MoveChapter { chapter: id.clone(), by: 1 })),
                        ("Delete".to_owned(), Action::AskDeleteChapter(id)),
                    ];
                    Some(Menu { at: s.mouse, title: Some(if chapter.title.is_empty() { chapter.id.clone() } else { chapter.title.clone() }), entries, icons: Vec::new() })
                }
                Some(Action::Group(id)) => {
                    let entries = vec![
                        ("Rename...".to_owned(), Action::OpenPanel(Subject::Group(id.clone()))),
                        ("Delete the group (its chapters stay)".to_owned(), Action::Edit(Edit::DeleteGroup { group: id.clone() })),
                    ];
                    Some(Menu { at: s.mouse, title: None, entries, icons: Vec::new() })
                }
                _ => None,
            };
            self.menu = menu;
            return Reaction::Nothing;
        }
        let on_canvas = canvas::inside(scene.regions.canvas, s.mouse) && !self.over_list(s);
        let Some(chapter) = s.book.file().chapters.get(self.chapter).filter(|_| on_canvas) else {
            return Reaction::Nothing;
        };
        let menu = match scene.action_at(s.mouse) {
            Some(Action::Quest(n)) => {
                let (n, quest) = (*n, s.book.quest(*n));
                let id = s.book.nodes()[n].id.clone();
                let mut entries = vec![("Settings...".to_owned(), Action::OpenPanel(Subject::Quest(id.clone()))), (
                    "Duplicate".to_owned(),
                    Action::Edit(Edit::DuplicateQuest { quest: id.clone(), x: quest.x + 1.0, y: quest.y }),
                )];
                entries.push(("Pick what this quest needs...".to_owned(), Action::Link(n, true)));
                entries.push(("Pick what this quest opens...".to_owned(), Action::Link(n, false)));
                entries.push(("Delete".to_owned(), Action::AskDelete(id)));
                Menu { at: s.mouse, title: Some(quest_title(quest).to_owned()), entries, icons: Vec::new() }
            }
            Some(Action::Deselect) | None => {
                let [x, y] = edit::snap(self.cell_under(s));
                let mut entries = vec![("New quest here".to_owned(), Action::Edit(Edit::NewQuest { chapter: chapter.id.clone(), x, y }))];
                entries.push(("Reward tables...".to_owned(), Action::Tables));
                entries.push(("Pack settings...".to_owned(), Action::OpenPanel(Subject::Settings)));
                entries.push(("Save these quests for the modpack".to_owned(), Action::SaveToModpack));
                if self.source == edit::SOURCE_WORLD {
                    entries.push(("Use the modpack's quests".to_owned(), Action::AskUseModpack));
                }
                Menu { at: s.mouse, title: None, entries, icons: Vec::new() }
            }
            Some(_) => return Reaction::Nothing,
        };
        self.menu = Some(menu);
        Reaction::Nothing
    }

    /// The grid place (cells) under the pointer, on the canvas shown.
    fn cell_under(&self, s: &Screen<'_>) -> [f32; 2] {
        let world = self.view.to_world(canvas::centre(regions(s.rect).canvas), s.mouse);
        [world[0] / canvas::GRID, world[1] / canvas::GRID]
    }

    /// Where quest `n` is drawn: where it is being dragged to, else its place.
    fn place_of(&self, book: &Book, n: usize) -> [f32; 2] {
        match &self.moving {
            Some(moving) if moving.node == n && moving.moved => edit::snap(moving.at),
            _ => {
                let quest = book.quest(n);
                [quest.x, quest.y]
            }
        }
    }

    /// A new pack replaced `old`: node numbers may have changed. The open
    /// quest stays open if it is still there, and link picking goes on;
    /// `select` (a full id) opens that quest instead. A drag and the menu stop.
    pub fn pack_changed(&mut self, old: Option<&Book>, book: &Book, select: Option<&str>) {
        self.moving = None;
        self.menu = None;
        let file = book.file();
        if self.panel.as_ref().is_some_and(|subject| !inline::subject_exists(book, subject)) {
            self.panel = None;
        }
        if let Some((Typed::Title(q) | Typed::Subtitle(q) | Typed::Description(q), _)) = &self.typing
            && book.find(q).is_none()
        {
            self.typing = None;
        }
        // A task or reward just made: its panel opens, its quest stays open.
        let select = match select {
            Some(id) if book.find_task(id).is_some() => {
                self.panel = Some(Subject::Task(id.to_owned()));
                id.rsplit_once('/').map(|(quest, _)| quest)
            }
            Some(id) if book.find_reward(id).is_some() => {
                self.panel = Some(Subject::Reward(id.to_owned()));
                id.rsplit_once('/').map(|(quest, _)| quest)
            }
            other => other,
        };
        // The chapter shown stays shown when chapters move; a new chapter is turned to.
        let shown = old.and_then(|old| old.file().chapters.get(self.chapter)).map(|c| c.id.clone());
        let at = |id: &str| file.chapters.iter().position(|c| c.id == id);
        if let Some(c) = select.and_then(at) {
            self.chapter = c;
            self.view = View::default();
            self.select(None);
            return;
        }
        self.chapter = shown.as_deref().and_then(at).unwrap_or(if self.chapter < file.chapters.len() { self.chapter } else { 0 });
        let again = |n: usize| old.and_then(|old| old.nodes().get(n)).and_then(|node| book.find(&node.id));
        self.linking = self.linking.and_then(|(n, needs)| Some((again(n)?, needs)));
        let open = self.selected.and_then(|n| old?.nodes().get(n)).map(|node| node.id.clone());
        let wanted = select.map(str::to_owned).or(open);
        match wanted.and_then(|id| book.find(&id)) {
            Some(n) => {
                self.chapter = book.nodes()[n].chapter;
                self.select(Some(n));
            }
            None => self.select(None),
        }
    }

    /// Closes the panel and the picker; text being typed is dropped.
    fn close_editors(&mut self) {
        self.panel = None;
        self.picker = None;
        self.typing = None;
    }

    /// An "add" menu at the pointer: each entry with its icon.
    fn add_menu(&mut self, s: &Screen<'_>, title: &str, adds: Vec<(String, String, Add)>) {
        let mut entries = Vec::new();
        let mut icons = Vec::new();
        for (label, icon, add) in adds {
            let action = match add {
                Add::Pick(purpose, mobs) => Action::PickFor(purpose, mobs),
                Add::Edit(change) => Action::Edit(change),
            };
            entries.push((label, action));
            icons.push(icon);
        }
        self.menu = Some(Menu { at: s.mouse, title: Some(title.to_owned()), entries, icons });
    }

    /// A click on the open panel: most change what it edits at once.
    fn panel_click(&mut self, s: &Screen<'_>, action: PanelAction) -> Reaction {
        let Some(subject) = self.panel.clone() else {
            return Reaction::Nothing;
        };
        match action {
            PanelAction::Close => self.panel = None,
            PanelAction::Stay => {}
            // The number itself: the wheel steps it.
            PanelAction::Step(_, by) if by == 0.0 => {}
            PanelAction::Type(key) => {
                let value = inline::subject_value(s.book, &subject, key);
                self.typing = Some((Typed::Panel(key), TextField::new(&value, 300)));
            }
            PanelAction::Pick(key) => {
                let (mobs, tags) = inline::subject_pick_kind(s.book, &subject, key);
                self.picker = Some((Picker::new(mobs, tags), Purpose::Panel(key)));
            }
            _ => {
                if let Some(change) = inline::subject_action(s.book, &subject, &action) {
                    if matches!(action, PanelAction::Press(_)) {
                        self.panel = None;
                    }
                    return Reaction::Edit(change);
                }
            }
        }
        Reaction::Nothing
    }

    /// The text being typed is kept: the edit it makes, if it changes anything.
    fn finish_typing(&mut self, s: &Screen<'_>) -> Reaction {
        let Some((target, field)) = self.typing.take() else {
            return Reaction::Nothing;
        };
        let text = field.value;
        let result = match &target {
            Typed::Title(q) | Typed::Subtitle(q) | Typed::Description(q) => {
                let Some(n) = s.book.find(q) else {
                    return Reaction::Nothing;
                };
                let before = QuestInfo::of(s.book.quest(n));
                let mut info = before.clone();
                match &target {
                    Typed::Title(_) => info.title = text.trim().to_owned(),
                    Typed::Subtitle(_) => info.subtitle = Some(text.trim().to_owned()).filter(|t| !t.is_empty()),
                    // Each line a paragraph; the book puts the space between them.
                    _ => info.description = text.split('\n').map(|line| line.trim().to_owned()).filter(|line| !line.is_empty()).collect(),
                }
                Ok((info != before).then(|| Edit::SetQuestInfo { quest: q.clone(), info }))
            }
            Typed::Panel(key) => match &self.panel {
                Some(subject) => inline::subject_text(s.book, subject, key, &text),
                None => Ok(None),
            },
        };
        match result {
            Ok(Some(change)) => Reaction::Edit(change),
            Ok(None) => Reaction::Nothing,
            Err(e) => {
                self.notice = e;
                Reaction::Nothing
            }
        }
    }

    /// A key pressed over the book: what it does, or None when the book
    /// leaves it to the game. While the picker is up or text is typed, the
    /// book takes every key: letters must not close the book.
    pub fn key(&mut self, s: &Screen<'_>, name: &str) -> Option<Reaction> {
        if let Some((picker, _)) = &mut self.picker {
            if name == "key.keyboard.escape" {
                self.picker = None;
            } else if let Some(key) = Key::named(name) {
                picker.query.key(key);
                picker.scroll = 0;
            }
            return Some(Reaction::Nothing);
        }
        if let Some((target, field)) = &mut self.typing {
            let key = Key::named(name);
            let description = matches!(target, Typed::Description(_));
            if matches!(name, "key.keyboard.escape" | "key.keyboard.tab") || (key == Some(Key::Enter) && !description) {
                return Some(self.finish_typing(s));
            }
            match key {
                Some(Key::Up | Key::Down) if description => {
                    let lines = field.wrapped(popup_text_width(s.rect), s.measure);
                    field.step_line(&lines, key == Some(Key::Down), s.measure);
                }
                Some(key) => {
                    field.key(key);
                }
                None => {}
            }
            return Some(Reaction::Nothing);
        }
        if name == "key.keyboard.escape" && (self.close_menu() || self.panel.take().is_some()) {
            return Some(Reaction::Nothing);
        }
        None
    }

    /// A character typed over the book: into the picker's search, or the
    /// text being typed.
    pub fn typed(&mut self, c: char) -> bool {
        if let Some((picker, _)) = &mut self.picker {
            picker.query.type_char(c);
            picker.scroll = 0;
            return true;
        }
        match &mut self.typing {
            Some((_, field)) => {
                field.type_char(c);
                true
            }
            None => false,
        }
    }

    /// Escape closes the edit menu, or ends link picking, first: whether
    /// one of them was on.
    pub fn close_menu(&mut self) -> bool {
        self.moving = None;
        self.menu.take().is_some() || self.linking.take().is_some()
    }

    /// The pointer dragged by `delta`: over the canvas, it pans.
    /// Each frame, with the pointer where it is: the chapter list opens when
    /// the pointer comes onto its strip, no click needed, and closes when
    /// the pointer leaves it.
    pub fn hover(&mut self, s: &Screen<'_>) {
        self.rect = s.rect;
        let regions = regions(s.rect);
        if canvas::inside(regions.strip, s.mouse) && self.recipe.is_none() && self.choosing.is_none() && self.picker.is_none() {
            self.sidebar_open = true;
        } else if !canvas::inside(regions.sidebar, s.mouse) {
            self.sidebar_open = false;
        }
    }

    /// Whether the open chapter list is under the pointer.
    fn over_list(&self, s: &Screen<'_>) -> bool {
        self.sidebar_open && canvas::inside(regions(s.rect).sidebar, s.mouse)
    }

    pub fn drag(&mut self, s: &Screen<'_>, delta: [f32; 2]) {
        if self.moving.is_some() {
            let at = self.cell_under(s);
            if let Some(moving) = &mut self.moving {
                let far = (s.mouse[0] - moving.pressed[0]).abs().max((s.mouse[1] - moving.pressed[1]).abs());
                moving.moved |= far > 2.0;
                moving.at = [at[0] - moving.grab[0], at[1] - moving.grab[1]];
            }
            return;
        }
        let scene = self.scene(s);
        let over_popup = scene.popup.is_some_and(|popup| canvas::inside(popup, s.mouse));
        if self.picker.is_none()
            && self.recipe.is_none()
            && self.choosing.is_none()
            && !over_popup
            && !self.over_list(s)
            && canvas::inside(scene.regions.canvas, s.mouse)
        {
            self.view.drag(delta);
        }
    }

    /// The wheel turned `steps` notches (positive: away from the player):
    /// it zooms the canvas, and scrolls the chapter list and the quest
    /// window's text.
    pub fn scroll(&mut self, s: &Screen<'_>, steps: f64) -> Reaction {
        let scene = self.scene(s);
        let regions = scene.regions;
        let moved = |scroll: f32, by: f32, max: f32| (scroll - steps as f32 * by).clamp(0.0, max.max(0.0));
        if !steps.is_finite() || steps == 0.0 {
            return Reaction::Nothing;
        }
        if let Some((picker, _)) = &mut self.picker {
            let max = picker.max_scroll(&self.item_names) as i64;
            picker.scroll = (picker.scroll as i64 - steps.signum() as i64).clamp(0, max) as usize;
            return Reaction::Nothing;
        }
        // Over a panel's number: the wheel steps it.
        if let Some(Action::Panel(PanelAction::Step(key, _))) = scene.action_at(s.mouse).cloned() {
            return self.panel_click(s, PanelAction::Step(key, steps.signum() as f32));
        }
        if scene.action_at(s.mouse).is_some_and(|a| matches!(a, Action::Panel(_))) {
        } else if !steps.is_finite() || self.recipe.is_some() || self.choosing.is_some() {
        } else if self.over_list(s) {
            self.sidebar_scroll = moved(self.sidebar_scroll, LIST_ROW, scene.sidebar_max_scroll);
        } else if scene.popup.is_some_and(|popup| canvas::inside(popup, s.mouse)) {
            self.popup_scroll = moved(self.popup_scroll, 3.0 * TEXT, scene.popup_max_scroll);
        } else if canvas::inside(regions.canvas, s.mouse) {
            self.view.zoom_at(canvas::centre(regions.canvas), s.mouse, steps);
        }
        Reaction::Nothing
    }

    /// Everything to draw this frame, and where clicks land.
    pub fn scene(&self, s: &Screen<'_>) -> Scene {
        let statuses = s.book.statuses(s.progress, s.player);
        let selected = self.shown_selection(s.book, &statuses);
        let regions = regions(s.rect);
        let popup = selected.map(|n| popup_layout(s, &self.item_names, self.clock, &statuses, n, regions.canvas, self.editing, self.typing.as_ref(), self.panel.is_some()).0);
        let mut scene = Scene {
            regions,
            draws: Vec::new(),
            zones: Vec::new(),
            sidebar_max_scroll: 0.0,
            popup_max_scroll: 0.0,
            popup,
        };
        scene.fill(s.rect, SHADE);
        let hovered = self.draw_canvas(&mut scene, s, &statuses, selected).filter(|_| !self.over_list(s));
        // The open list replaces its strip (the panel is see-through).
        if self.sidebar_open {
            self.draw_sidebar(&mut scene, s, &statuses);
        } else {
            draw_strip(&mut scene, s);
        }
        self.draw_bands(&mut scene, s, &statuses);
        let mut tip = hovered.map(|n| vec![(quest_title(s.book.quest(n)).to_owned(), INK)]);
        if let (Some(n), Some(popup)) = (selected, popup) {
            tip = self.draw_popup(&mut scene, s, &statuses, n, popup);
        }
        if let Some(reward) = self.choosing.as_deref().filter(|_| self.recipe.is_none()) {
            tip = draw_choice(&mut scene, s, &self.item_names, reward);
        }
        // Edit mode's panel beside the quest window, the picker over everything.
        let mut over_panel = false;
        if let Some(subject) = &self.panel
            && let Some((title, controls)) = inline::subject_panel(s.book, subject)
        {
            let anchor = popup.unwrap_or([regions.canvas[0], regions.canvas[1] + PAD, 0.0, 0.0]);
            let typing = match &self.typing {
                Some((Typed::Panel(key), field)) => Some((*key, field)),
                _ => None,
            };
            let (rect, draws, zones) = inline::panel(&title, &controls, anchor, regions.canvas, typing, s.mouse, self.clock, s.measure);
            over_panel = canvas::inside(rect, s.mouse);
            scene.draws.extend(draws);
            scene.zones.extend(zones.into_iter().map(|(r, a)| (r, Action::Panel(a))));
        }
        if over_panel {
            tip = None;
        }
        if let Some((picker, _)) = &self.picker {
            let (_, draws, zones, picked) = picker.layout(regions.canvas, &self.item_names, true, s.mouse, self.clock, s.measure);
            scene.draws.extend(draws);
            scene.zones.extend(zones.into_iter().map(|(r, a)| (r, Action::Pick(a))));
            tip = picked.map(|lines| lines.into_iter().zip([INK, MOD_INK]).collect());
        }
        if let Some(recipe) = &self.recipe {
            draw_recipe(&mut scene, s, recipe);
        } else if let Some(menu) = &self.menu {
            draw_menu(&mut scene, s, menu);
        } else if let Some(lines) = tip.filter(|_| self.moving.as_ref().is_none_or(|m| !m.moved)) {
            draw_tooltip(&mut scene, s, &lines);
        }
        scene
    }

    /// The chapter's lines and nodes. Returns the quest under the pointer.
    fn draw_canvas(
        &self,
        scene: &mut Scene,
        s: &Screen<'_>,
        statuses: &[Status],
        selected: Option<usize>,
    ) -> Option<usize> {
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
        let centre_of = |n: usize| self.view.to_screen(origin, canvas::cell_centre(self.place_of(book, n)));
        if self.editing {
            draw_grid(scene, &self.view, origin, area);
        }

        // The quest the lines are about: the one under the pointer, else
        // the selected one. What it needs comes in cyan, what it opens goes
        // out in yellow, over the other lines.
        let hovered = shown
            .iter()
            .copied()
            .filter(|&n| {
                let b = node_box(&self.view, origin, self.place_of(book, n), book.nodes()[n].shape, book.nodes()[n].size);
                intersect(b, area).is_some_and(|part| canvas::inside(part, s.mouse))
            })
            .last();
        let over_popup = selected.is_some_and(|n| canvas::inside(popup_layout(s, &self.item_names, self.clock, statuses, n, area, self.editing, self.typing.as_ref(), self.panel.is_some()).0, s.mouse));
        let hovered = hovered.filter(|_| !over_popup);
        let focus = self.linking.map(|(n, _)| n).or(hovered).or(selected);

        // Dependency lines first, under the nodes: thin chevrons pointing
        // from the quest needed to the quest it opens, a bead between each
        // two, green once both quests are done, darker and half see-through
        // into a quest that cannot be started yet. Those of the quest in focus
        // (cyan: what it needs, yellow: what it opens) slide along; the
        // others stay still. Lines to another chapter's quests are not drawn.
        let zoom = self.view.zoom;
        let step = (CHEVRON_STEP * zoom).max(7.0);
        let (chevron, bead) = (CHEVRON_SIZE * zoom, BEAD_SIZE * zoom);
        let partial = self.partial_tick.clamp(0.0, 1.0);
        let flow = ((self.clock % FLOW_TICKS) as f32 + partial) / FLOW_TICKS as f32;
        let mut lines = Vec::new();
        for &n in &shown {
            for &d in &book.nodes()[n].dependencies {
                if !shown.contains(&d) {
                    continue;
                }
                let (colour, on_top) = if focus == Some(n) {
                    (LINE_NEEDS, true)
                } else if focus == Some(d) {
                    (LINE_OPENS, true)
                } else if s.progress.is_completed(&book.nodes()[d].id) && s.progress.is_completed(&book.nodes()[n].id) {
                    (LINE_DONE, false)
                } else if statuses[n].state == QuestState::Locked {
                    (LINE_LOCKED, false)
                } else {
                    (LINE, false)
                };
                lines.push((on_top, d, n, colour));
            }
        }
        // The lines of the quest in focus go last, over the others.
        lines.sort_by_key(|(on_top, ..)| *on_top);
        for (focused, d, n, colour) in lines {
            let (from, to) = (centre_of(d), centre_of(n));
            let len = ((to[0] - from[0]).powi(2) + (to[1] - from[1]).powi(2)).sqrt();
            if !len.is_finite() || len < 0.001 {
                continue;
            }
            let half = chevron / 2.0;
            let bounds = [from[0].min(to[0]) - half, from[1].min(to[1]) - half, (to[0] - from[0]).abs() + chevron, (to[1] - from[1]).abs() + chevron];
            if !canvas::overlaps(area, bounds) {
                continue;
            }
            let (ux, uy) = ((to[0] - from[0]) / len, (to[1] - from[1]) / len);
            let (chevron_src, bead_src) = (chevron_cell(colour, [ux, uy]), bead_cell(colour));
            for (at, is_chevron) in line_marks(len, step, if focused { flow } else { 0.0 }) {
                let (size, src) = if is_chevron { (chevron, chevron_src) } else { (bead, bead_src) };
                let dst = [from[0] + ux * at - size / 2.0, from[1] + uy * at - size / 2.0, size, size];
                scene.draws.push(Draw::Sprite { name: CHEVRON_SPRITE.to_owned(), src, dst, fallback: Vec::new() });
            }
        }

        for &n in &shown {
            let node = &book.nodes()[n];
            let quest = book.quest(n);
            let at_rest = node_box(&self.view, origin, self.place_of(book, n), node.shape, node.size);
            // Only the part of the node inside the canvas takes clicks.
            let Some(part) = intersect(at_rest, area) else {
                continue;
            };
            // The node under the pointer, and the selected one, stand out a little.
            let raised = hovered == Some(n) || selected == Some(n) || self.linking.is_some_and(|(from, _)| from == n);
            let b = if raised { grow(at_rest, HOVER_GROW) } else { at_rest };
            let centre = canvas::centre(b);
            let radius = b[2] / 2.0;
            // The frame's texture, or the same shape as polygons without it.
            let state = statuses[n].state;
            let mut fallback = polygon_draws(node.shape, centre, radius, state_look(state).0);
            fallback.extend(polygon_draws(node.shape, centre, (radius - 1.5).max(0.0), NODE_FILL));
            scene.draws.push(Draw::Sprite { name: NODE_SPRITE.to_owned(), src: node_cell(node.shape, state), dst: b, fallback });
            // Each shape has its own room inside.
            let share = shape_look(node.shape).1;
            let icon = (b[2] * share).clamp(4.0, 64.0);
            let at = [centre[0] - icon / 2.0, centre[1] - icon / 2.0];
            if contains(area, [at[0], at[1], icon, icon]) {
                scene.draws.push(Draw::Item { item: quest_icon(quest).to_owned(), at, size: icon });
                // A locked quest: its icon and the frame's face greyed out
                // under a veil of the face's own shape (no square in it).
                if state == QuestState::Locked {
                    scene.draws.extend(polygon_draws(node.shape, centre, radius * face_share(node.shape), LOCKED_VEIL));
                }
            }
            // And a small padlock on the bottom-right corner of the box a
            // square of the same size would have: the same padlock for all shapes.
            if state == QuestState::Locked {
                if let lock @ Draw::Sprite { dst, .. } = lock_draws(grow(b, 1.0 / shape_look(node.shape).0))
                    && contains(area, dst)
                {
                    scene.draws.push(lock);
                }
            }
            // Rewards to claim: a red "!" badge on the top-right corner of
            // the same box, a little outside the frame.
            if state == QuestState::RewardsWaiting {
                if let badge @ Draw::Sprite { dst, .. } = alert_badge_draws(grow(b, 1.0 / shape_look(node.shape).0))
                    && contains(area, dst)
                {
                    scene.draws.push(badge);
                }
            }
            scene.zones.push((part, Action::Quest(n)));
        }
        hovered.filter(|_| canvas::inside(area, s.mouse))
    }

    /// The open chapter list, over the canvas: group headings with their
    /// triangle, chapters with their icon (indented under a group), the
    /// current one outlined, names coloured by `chapter_ink`, a red "!" at
    /// the end of the row of a chapter with rewards to claim.
    fn draw_sidebar(&self, scene: &mut Scene, s: &Screen<'_>, statuses: &[Status]) {
        let area = scene.regions.sidebar;
        scene.fill(area, LIST_BACK);
        scene.fill([area[0] + area[2] - 1.0, area[1], 1.0, area[3]], LIST_EDGE);
        // The list's background takes clicks: none fall through to the canvas.
        scene.zones.push((area, Action::Stay));
        let rows = sidebar_rows(s.book, &self.folded);
        // In edit mode, two more rows at the end make a chapter or a group.
        let extra = if self.editing { 2 } else { 0 };
        scene.sidebar_max_scroll = ((rows.len() + extra) as f32 * LIST_ROW + 2.0 * PAD - area[3]).max(0.0);
        if self.editing {
            let adds = [
                ("+ New chapter", Edit::NewChapter { title: "New chapter".to_owned() }),
                ("+ New group", Edit::NewGroup { title: "New group".to_owned() }),
            ];
            for (k, (label, change)) in adds.into_iter().enumerate() {
                let rect = [area[0] + 2.0, area[1] + PAD + (rows.len() + k) as f32 * LIST_ROW - self.sidebar_scroll, area[2] - 5.0, LIST_ROW];
                if contains(area, rect) {
                    if canvas::inside(rect, s.mouse) {
                        scene.fill(rect, LIST_HOVER);
                    }
                    scene.text(label, [rect[0] + 3.0, rect[1] + (LIST_ROW - 8.0) / 2.0], EDIT_INK);
                    scene.zones.push((rect, Action::Edit(change)));
                }
            }
        }
        let (x, w) = (area[0] + 2.0, area[2] - 5.0);
        let mut in_group = false;
        for (i, row) in rows.iter().enumerate() {
            in_group |= matches!(row, Row::Group { .. });
            let rect = [x, area[1] + PAD + i as f32 * LIST_ROW - self.sidebar_scroll, w, LIST_ROW];
            if !contains(area, rect) {
                continue;
            }
            let text_y = rect[1] + (LIST_ROW - 8.0) / 2.0;
            match row {
                Row::Group { id, title } => {
                    if canvas::inside(rect, s.mouse) {
                        scene.fill(rect, LIST_HOVER);
                    }
                    let open = !self.folded.contains(id);
                    scene.draws.push(Draw::Quad { corners: arrow([x + 3.0, rect[1] + LIST_ROW / 2.0], 5.0, open), colour: LIST_ARROW });
                    let left = x + 12.0;
                    scene.text(fit_text(title, x + w - 2.0 - left, s.measure), [left, text_y], GROUP_INK);
                    scene.zones.push((rect, Action::Group(id.clone())));
                }
                Row::Chapter(c) => {
                    let chapter = s.book.chapter(*c);
                    if *c == self.chapter {
                        scene.fill(rect, LIST_CURRENT);
                        outline(scene, rect, LIST_OUTLINE);
                    } else if canvas::inside(rect, s.mouse) {
                        scene.fill(rect, LIST_HOVER);
                    }
                    let mut left = x + if in_group { 8.0 } else { 3.0 };
                    if let Some(icon) = &chapter.icon {
                        let at = [left, rect[1] + (LIST_ROW - LIST_ICON) / 2.0];
                        scene.draws.push(Draw::Item { item: icon.clone(), at, size: LIST_ICON });
                    }
                    left += LIST_ICON + 3.0;
                    let states: Vec<QuestState> = s.book.chapter_nodes(*c).map(|n| statuses[n].state).collect();
                    let waiting = states.contains(&QuestState::RewardsWaiting);
                    let mut right = x + w - 2.0;
                    if waiting {
                        let size = LIST_ROW - 5.0;
                        right -= size + 2.0;
                        scene.draws.push(alert_sign([right + 1.0, rect[1] + (LIST_ROW - size) / 2.0, size, size]));
                    }
                    let name = if chapter.title.is_empty() { &chapter.id } else { &chapter.title };
                    scene.text(fit_text(name, right - left, s.measure), [left, text_y], chapter_ink(s.book.chapter_completed(*c, s.progress), &states));
                    scene.zones.push((rect, Action::Chapter(*c)));
                }
            }
        }
    }

    /// The window of quest `n`, centred over the canvas: its title (with
    /// Track and close), its tasks and its rewards as icons side by side,
    /// then what it is (state, what it needs, its description). A click on
    /// the canvas outside it closes it; another quest opens that one.
    /// Returns the tooltip of the icon under the pointer.
    fn draw_popup(&self, scene: &mut Scene, s: &Screen<'_>, statuses: &[Status], n: usize, b: [f32; 4]) -> Option<Vec<(String, u32)>> {
        let book = s.book;
        let node = &book.nodes()[n];
        let quest = book.quest(n);
        let [x, y, w, h] = b;
        // Under every other zone: an empty spot of the canvas closes it.
        scene.zones.insert(0, (scene.regions.canvas, Action::Deselect));
        scene.fill(b, POPUP_BACK);
        scene.fill([x, y, w, 1.0], POPUP_EDGE);
        scene.fill([x, y + h - 1.0, w, 1.0], POPUP_EDGE);
        scene.fill([x, y, 1.0, h], POPUP_EDGE);
        scene.fill([x + w - 1.0, y, 1.0, h], POPUP_EDGE);
        scene.zones.push((b, Action::Stay));

        // The title bar: icon, title, Track and close.
        let bar = POPUP_BAR;
        scene.fill([x + 1.0, y + 1.0, w - 2.0, bar], BAND);
        let icon_box = [x + PAD - 1.0, y + 2.0, 14.0, 14.0];
        if self.editing {
            // In edit mode the icon opens the item picker.
            if canvas::inside(icon_box, s.mouse) {
                scene.fill(icon_box, ROW_HOVER);
            }
            scene.zones.push((icon_box, Action::PickFor(Purpose::QuestIcon(node.id.clone()), false)));
        }
        scene.draws.push(Draw::Item { item: quest_icon(quest).to_owned(), at: [x + PAD, y + 3.0], size: 12.0 });
        let close = scene.button(s, "x", x + w - PAD, y + 4.0, Action::Deselect);
        // The pin: tracks the quest on the HUD, or stops.
        let tracking = self.tracked.as_deref() == Some(node.id.as_str());
        let pin = [close - PAD - PIN_SIZE[0] - 4.0, y + 2.0, PIN_SIZE[0] + 4.0, PIN_SIZE[1] + 4.0];
        let over_pin = canvas::inside(pin, s.mouse);
        scene.draws.extend(pin_draws([pin[0] + 2.0, pin[1] + 2.0], tracking, over_pin));
        let action = if tracking { Action::Untrack } else { Action::Track(node.id.clone()) };
        scene.zones.push((pin, action));
        let pin_tip = if tracking { "Unpin: stop showing it while you play" } else { "Pin: show it while you play" };
        let mut tip_gear = false;
        let mut track = pin[0];
        if self.editing {
            // The gear: the quest's settings panel.
            let gear = [track - 13.0, y + 3.0, 11.0, 11.0];
            let over = canvas::inside(gear, s.mouse);
            scene.draws.extend(pixel_art(&GEAR_PIXELS, [gear[0] + 1.0, gear[1] + 1.0], 1.0, &|c| (c == 'g').then_some(if over { 0xFFFF_FFFF } else { 0xFFB8_B8C8 })));
            scene.zones.push((gear, Action::OpenPanel(Subject::Quest(node.id.clone()))));
            if over {
                tip_gear = true;
            }
            track = gear[0];
        }
        let title_left = x + PAD + 16.0;
        let room = track - PAD - title_left;
        let title_box = [title_left - 2.0, y + 2.0, room + 4.0, bar - 2.0];
        match &self.typing {
            Some((Typed::Title(q), field)) if *q == node.id => {
                scene.fill(title_box, 0xFF0C_0E14);
                outline(scene, title_box, 0xFF80_90C0);
                let (shown, _, cursor) = field.shown(0, room, s.measure);
                scene.text(shown, [title_left, y + 5.0], INK);
                if let Some(cx) = cursor.filter(|_| (self.clock / 10) % 2 == 0) {
                    scene.fill([title_left + cx, y + 4.0, 1.0, 9.0], CURSOR_INK);
                }
                scene.zones.push((title_box, Action::Caret(0, title_left)));
            }
            _ => {
                let title = if quest.title.is_empty() { &quest.id } else { &quest.title };
                let title = fit_text(title, room, s.measure);
                let title_x = title_left + ((room - (s.measure)(&title)) / 2.0).max(0.0);
                if self.editing {
                    if canvas::inside(title_box, s.mouse) {
                        outline(scene, title_box, POPUP_EDGE);
                    }
                    scene.zones.push((title_box, Action::Type(Typed::Title(node.id.clone()))));
                }
                scene.text(title, [title_x, y + 5.0], INK);
            }
        }

        // Tasks on the left, rewards on the right.
        let content = popup_layout(s, &self.item_names, self.clock, statuses, n, scene.regions.canvas, self.editing, self.typing.as_ref(), self.panel.is_some()).1;
        let top = y + 1.0 + bar + 1.0;
        let half = (w - 2.0) / 2.0;
        let columns_h = content.columns_h;
        let left = [x + 1.0, top, half, columns_h];
        let right = [x + 1.0 + half, top, half, columns_h];
        scene.fill([x + 1.0 + half, top, 1.0, columns_h], POPUP_EDGE);
        scene.fill([x + 1.0, top + columns_h, w - 2.0, 1.0], POPUP_EDGE);
        let heading = if quest.sequential_tasks { "Tasks, in order" } else { "Tasks" };
        let mut tip = over_pin.then(|| vec![(pin_tip.to_owned(), INK)]);
        if tip_gear {
            tip = Some(vec![("Quest settings".to_owned(), INK)]);
        }
        tip = draw_icons(scene, s, left, heading, TASK_INK, &content.tasks).or(tip);
        tip = draw_icons(scene, s, right, "Rewards", GOLD_INK, &content.rewards).or(tip);

        // What it is, under them: it scrolls with the wheel when it is long.
        let body = [x + 1.0, top + columns_h + 1.0, w - 2.0, (h - bar - columns_h - 4.0).max(0.0)];
        scene.popup_max_scroll = (content.lines.len() as f32 * TEXT + 2.0 * PAD - body[3]).max(0.0);
        for (i, (text, colour)) in content.lines.into_iter().enumerate() {
            let line_y = body[1] + PAD + i as f32 * TEXT - self.popup_scroll;
            if contains(body, [body[0], line_y, body[2], TEXT]) {
                scene.text(text, [x + PAD, line_y + 1.0], colour);
                if let Some(action) = content.actions.get(i).cloned().flatten() {
                    scene.zones.push(([x + 1.0, line_y, w - 2.0, TEXT], action));
                }
                if let Some((_, cx)) = content.cursor.filter(|(line, _)| *line == i && (self.clock / 10) % 2 == 0) {
                    scene.fill([x + PAD + cx, line_y, 1.0, 9.0], CURSOR_INK);
                }
            }
        }
        tip
    }

    /// The header (chapter title, tally, close) and the footer (claim all,
    /// notice, help), over anything under them.
    fn draw_bands(&self, scene: &mut Scene, s: &Screen<'_>, statuses: &[Status]) {
        // One band at the bottom: the chapter's name and tally, "Claim all"
        // and the last notice on the left; close, the party and the help on
        // the right.
        let footer = scene.regions.footer;
        let [x, y, w, _] = footer;
        scene.fill(footer, if self.editing { EDIT_BAND } else { BAND });
        let mut right = scene.button(s, "x", x + w - 4.0, y + 2.0, Action::Close) - 6.0;
        if self.editor {
            right = pencil_button(scene, s, right, y + 2.0, self.editing) - 6.0;
        }
        let mut left = x + 6.0;
        if self.editing {
            let label = format!("Editing ({})", source_name(&self.source));
            scene.text(label.clone(), [left, y + 3.0], EDIT_INK);
            left += (s.measure)(&label) + 10.0;
        }
        if let Some(chapter) = s.book.file().chapters.get(self.chapter) {
            let name = if chapter.title.is_empty() { &chapter.id } else { &chapter.title };
            let title = fit_text(name, w / 3.0, s.measure);
            let title_w = (s.measure)(&title);
            scene.text(title, [left, y + 3.0], INK);
            left += title_w + 6.0;
            let (done, total) = s.book.chapter_tally(self.chapter, s.progress);
            let tally = format!("{done}/{total}");
            let tally_w = (s.measure)(&tally);
            scene.text(tally, [left, y + 3.0], DIM_INK);
            left += tally_w + 10.0;
        }
        let waiting = statuses.iter().filter(|status| status.state == QuestState::RewardsWaiting).count();
        if waiting > 0 {
            let label = format!("Claim all ({waiting})");
            let width = (s.measure)(&label) + 6.0;
            scene.button(s, &label, left + width, y + 2.0, Action::ClaimAll);
            left += width + 6.0;
        }
        if let Some(party) = &self.party {
            let label = format!("Party {party}");
            let width = (s.measure)(&label);
            // Only when the rest keeps some room.
            if width < (right - left) / 3.0 {
                right -= width;
                scene.text(label, [right, y + 3.0], GOLD_INK);
                right -= 10.0;
            }
        }
        let help = match self.linking {
            _ if self.picker.is_some() => "type to search | wheel: more | Esc: close",
            _ if self.typing.is_some() => "click outside, Enter or Esc: done",
            Some((_, true)) => "click the quests this one needs | Esc: done",
            Some((_, false)) => "click the quests this one opens | Esc: done",
            None if self.editing => "right-click: menu | drag a quest: move it",
            None => "drag: move | wheel: zoom",
        };
        let help_w = (s.measure)(help);
        let notice = fit_text(&self.notice, (right - left).max(0.0), s.measure);
        let notice_end = left + (s.measure)(&notice);
        scene.text(notice, [left, y + 3.0], GOLD_INK);
        if right - help_w >= notice_end + 6.0 {
            scene.text(help, [right - help_w, y + 3.0], DIM_INK);
        }
    }
}

/// A gear: the quest settings button in edit mode.
const GEAR_PIXELS: [&str; 9] = ["...g.g...", "..ggggg..", ".gg...gg.", "ggg...ggg", ".g..g..g.", "ggg...ggg", ".gg...gg.", "..ggggg..", "...g.g..."];
/// A hint in place of something to write; the typing cursor.
const HINT_INK: u32 = 0x78_7888;
const CURSOR_INK: u32 = 0xFFE0_E0E8;

/// Edit mode's band, the "Editing" line, the menu's look.
const EDIT_BAND: u32 = 0xFF3C_2C14;
const EDIT_INK: u32 = 0xFF_B040;
const DANGER_INK: u32 = 0xFF_6A5A;
const MENU_BACK: u32 = 0xF814_1820;
const MENU_ROW: f32 = 12.0;
/// A dot on each grid cell in edit mode.
const GRID_DOT: u32 = 0x40FF_FFFF;

/// `source` for the player.
fn source_name(source: &str) -> &'static str {
    match source {
        edit::SOURCE_WORLD => "this world's quests",
        edit::SOURCE_MODPACK => "the modpack's quests",
        edit::SOURCE_DATA_PACKS => "the data packs' quests",
        edit::SOURCE_BUILT_IN => "the built-in quests",
        _ => "no quests",
    }
}

/// A quest's title, or its id when it has none.
fn quest_title(quest: &Quest) -> &str {
    if quest.title.is_empty() { &quest.id } else { &quest.title }
}

/// A pencil, point down-left: `w` its wood, `p` its eraser, `g` its lead.
const PENCIL_PIXELS: [&str; 8] = ["......pp", ".....ppp", "....wwp.", "...www..", "..www...", ".www....", "gww.....", "gg......"];

/// The pencil button that turns edit mode on and off, its right edge at
/// `right`: lit while edit mode is on. Returns its left edge.
fn pencil_button(scene: &mut Scene, s: &Screen<'_>, right: f32, y: f32, on: bool) -> f32 {
    let rect = [right - 12.0, y, 12.0, TEXT];
    let hovered = canvas::inside(rect, s.mouse);
    scene.fill(rect, if on { 0xFFB0_7020 } else if hovered { BUTTON_HOVER } else { BUTTON });
    let wood = if on || hovered { 0xFFFF_E080 } else { 0xFFE8_C050 };
    scene.draws.extend(pixel_art(&PENCIL_PIXELS, [rect[0] + 2.0, y + 1.0], 1.0, &|c| match c {
        'w' => Some(wood),
        'p' => Some(0xFFF0_8090),
        'g' => Some(0xFF40_4048),
        _ => None,
    }));
    scene.zones.push((rect, Action::ToggleEdit));
    rect[0]
}

/// A faint dot on every grid cell of the canvas shown, while the zoom
/// leaves room between them.
fn draw_grid(scene: &mut Scene, view: &View, origin: [f32; 2], area: [f32; 4]) {
    let step = canvas::GRID * view.zoom;
    if step < 8.0 {
        return;
    }
    let first = view.to_world(origin, [area[0], area[1]]);
    let last = view.to_world(origin, [area[0] + area[2], area[1] + area[3]]);
    let (x0, x1) = ((first[0] / canvas::GRID).ceil() as i32, (last[0] / canvas::GRID).floor() as i32);
    let (y0, y1) = ((first[1] / canvas::GRID).ceil() as i32, (last[1] / canvas::GRID).floor() as i32);
    for cy in y0..=y1 {
        for cx in x0..=x1 {
            let [sx, sy] = view.to_screen(origin, canvas::cell_centre([cx as f32, cy as f32]));
            scene.fill([sx.floor(), sy.floor(), 1.0, 1.0], GRID_DOT);
        }
    }
}

/// Edit mode's menu at its place, kept on screen: a dim title, then a row
/// per entry. A click anywhere else closes it.
fn draw_menu(scene: &mut Scene, s: &Screen<'_>, menu: &Menu) {
    scene.zones.push((s.rect, Action::CloseMenu));
    let rows = menu.entries.len() + usize::from(menu.title.is_some());
    let indent = if menu.icons.is_empty() { 0.0 } else { 15.0 };
    let widest = menu.title.iter().chain(menu.entries.iter().map(|(label, _)| label)).map(|text| (s.measure)(text)).fold(0.0, f32::max);
    let row_h = if menu.icons.is_empty() { MENU_ROW } else { 15.0 };
    let (w, h) = (widest + indent + 2.0 * PAD + 2.0, rows as f32 * row_h + 2.0);
    let [rx, ry, rw, rh] = s.rect;
    let x = menu.at[0].min(rx + rw - w).max(rx);
    let y = menu.at[1].min(ry + rh - h).max(ry);
    scene.fill([x, y, w, h], MENU_BACK);
    outline(scene, [x, y, w, h], POPUP_EDGE);
    let mut row_y = y + 1.0;
    if let Some(title) = &menu.title {
        scene.text(title.clone(), [x + PAD + 1.0, row_y + (row_h - 8.0) / 2.0], DIM_INK);
        scene.zones.push(([x, row_y, w, row_h], Action::Stay));
        row_y += row_h;
    }
    for (i, (label, action)) in menu.entries.iter().enumerate() {
        let rect = [x + 1.0, row_y, w - 2.0, row_h];
        if canvas::inside(rect, s.mouse) {
            scene.fill(rect, ROW_HOVER);
        }
        let danger = matches!(
            action,
            Action::Edit(Edit::DeleteQuest { .. } | Edit::DeleteChapter { .. } | Edit::DeleteTable { .. })
                | Action::UseModpack
                | Action::AskDelete(_)
                | Action::AskDeleteChapter(_)
                | Action::AskDeleteTable(_)
        );
        if let Some(icon) = menu.icons.get(i) {
            scene.draws.push(Draw::Item { item: icon.clone(), at: [x + PAD, row_y + 1.5], size: 12.0 });
        }
        scene.text(label.clone(), [x + PAD + 1.0 + indent, row_y + (row_h - 8.0) / 2.0], if danger { DANGER_INK } else { INK });
        scene.zones.push((rect, action.clone()));
        row_y += row_h;
    }
}

/// The quest window: at most this big, centred over the canvas.
const POPUP_SIZE: [f32; 2] = [300.0, 200.0];
const POPUP_BAR: f32 = 16.0;
const POPUP_BACK: u32 = 0xF41C_2230;
/// The choice window, opaque: it sits over the quest window's text.
const CHOICE_BACK: u32 = 0xFF1C_2230;
const POPUP_EDGE: u32 = 0xFF3A_4458;
/// The "Tasks" heading's colour.
const TASK_INK: u32 = 0x7C7C_FF;
/// An icon's cell in the quest window, and the most icons a column shows.
const ICON_CELL: f32 = 20.0;
const ICONS_MAX: usize = 24;
/// Over a task that waits for the ones before it: a dark veil.
const DIM_VEIL: u32 = 0xA010_1018;

/// What the quest window holds, laid out for its width: its icons, how tall
/// their columns are, and the lines of text under them.
struct PopupContent {
    tasks: Vec<Icon>,
    rewards: Vec<Icon>,
    columns_h: f32,
    lines: Vec<(String, u32)>,
    /// In edit mode: what a click on each line does, and where the cursor
    /// of the text typed is (line, x).
    actions: Vec<Option<Action>>,
    cursor: Option<(usize, f32)>,
}

/// The width the quest window's text wraps at, in a book filling `rect`.
fn popup_text_width(rect: [f32; 4]) -> f32 {
    (popup_width(regions(rect).canvas) - 2.0 * PAD).max(0.0)
}

/// The quest window's text in edit mode: its subtitle and description, or
/// hints to write them; a click on them types there. While one is typed,
/// its field's lines and cursor. `left` is where the text starts on screen.
#[allow(clippy::type_complexity)]
fn edit_lines(
    s: &Screen<'_>,
    n: usize,
    text_w: f32,
    left: f32,
    typing: Option<&(Typed, TextField)>,
) -> (Vec<(String, u32)>, Vec<Option<Action>>, Option<(usize, f32)>) {
    let quest = s.book.quest(n);
    let id = s.book.nodes()[n].id.clone();
    let (mut lines, mut actions, mut cursor) = (Vec::new(), Vec::new(), None);
    match typing {
        Some((Typed::Subtitle(q), field)) if *q == id => {
            let (shown, _, cx) = field.shown(0, text_w, s.measure);
            cursor = cx.map(|cx| (0, cx));
            lines.push((shown, INK));
            actions.push(Some(Action::Caret(0, left)));
        }
        _ => {
            match &quest.subtitle {
                Some(subtitle) => lines.extend(wrap(subtitle, text_w, s.measure).into_iter().map(|line| (line, DIM_INK))),
                None => lines.push(("Click to add a subtitle".to_owned(), HINT_INK)),
            }
            actions.resize(lines.len(), Some(Action::Type(Typed::Subtitle(id.clone()))));
        }
    }
    lines.push((String::new(), INK));
    actions.push(None);
    match typing {
        Some((Typed::Description(q), field)) if *q == id => {
            let wrapped = field.wrapped(text_w, s.measure);
            let (at, cx) = field.cursor_in(&wrapped, s.measure);
            cursor = Some((lines.len() + at, cx));
            let chars: Vec<char> = field.value.chars().collect();
            for (i, (start, end)) in wrapped.into_iter().enumerate() {
                lines.push((chars[start..end].iter().collect(), INK));
                actions.push(Some(Action::Caret(i, left)));
            }
        }
        _ => {
            let first = lines.len();
            for (p, paragraph) in quest.description.iter().enumerate() {
                if p > 0 {
                    lines.push((String::new(), INK));
                }
                lines.extend(wrap(paragraph, text_w, s.measure).into_iter().map(|line| (line, INK)));
            }
            if lines.len() == first {
                lines.push(("Click to write a description".to_owned(), HINT_INK));
            }
            actions.resize(lines.len(), Some(Action::Type(Typed::Description(id))));
        }
    }
    (lines, actions, cursor)
}

/// The quest window's width over the canvas `area`.
fn popup_width(area: [f32; 4]) -> f32 {
    POPUP_SIZE[0].min(area[2] - 2.0 * PAD).max(0.0)
}

/// Quest `n`'s window: its place over the canvas `area`, as tall as what it
/// holds (at most `POPUP_SIZE`, and the canvas), and its content.
fn popup_layout(
    s: &Screen<'_>,
    names: &HashMap<String, String>,
    clock: u32,
    statuses: &[Status],
    n: usize,
    area: [f32; 4],
    editing: bool,
    typing: Option<&(Typed, TextField)>,
    beside: bool,
) -> ([f32; 4], PopupContent) {
    let [x, y, w, h] = area;
    let pw = popup_width(area);
    let half = (pw - 2.0) / 2.0;
    let (mut tasks, mut rewards) = (task_icons(s, names, n), reward_icons(s, names, clock, n));
    if editing {
        // In edit mode a click edits a task or reward, and a "+" adds one.
        let quest = s.book.quest(n);
        let id = &s.book.nodes()[n].id;
        let to_edit = |icon: &mut Icon, action: Action| {
            // A click opens its panel beside the window.
            icon.action = action;
            icon.tip.retain(|(_, colour)| *colour != CLICK_INK);
            icon.tip.push(("Click to edit".to_owned(), CLICK_INK));
        };
        for (icon, task) in tasks.iter_mut().zip(&quest.tasks) {
            to_edit(icon, Action::OpenPanel(Subject::Task(child_id(id, &task.id))));
        }
        for (icon, reward) in rewards.iter_mut().zip(&quest.rewards) {
            to_edit(icon, Action::OpenPanel(Subject::Reward(child_id(id, &reward.id))));
        }
        tasks.push(plus_icon("Add a task", Action::AddTask(id.clone())));
        rewards.push(plus_icon("Add a reward", Action::AddReward(id.clone())));
    }
    let per_row = ((half - 2.0 * PAD) / ICON_CELL).floor().max(1.0) as usize;
    let rows = tasks.len().max(rewards.len()).min(ICONS_MAX).div_ceil(per_row).max(1);
    let max_h = POPUP_SIZE[1].min(h - 2.0 * PAD).max(0.0);
    let columns_h = (3.0 * PAD + TEXT + rows as f32 * ICON_CELL).min((max_h - POPUP_BAR) * 0.6);
    let text_w = (pw - 2.0 * PAD).max(0.0);
    // With a panel open, the window moves left so both fit side by side.
    let both = pw + inline::GAP + inline::PANEL_W;
    let popup_x = if beside && both <= w - 2.0 * PAD { x + (w - both) / 2.0 } else { x + (w - pw) / 2.0 };
    let left = popup_x + PAD;
    let (lines, actions, cursor) = if editing {
        edit_lines(s, n, text_w, left, typing)
    } else {
        (popup_lines(s, statuses, n, text_w), Vec::new(), None)
    };
    let wanted = POPUP_BAR + columns_h + 4.0 + lines.len() as f32 * TEXT + 2.0 * PAD;
    let ph = wanted.min(max_h);
    ([popup_x, y + (h - ph) / 2.0, pw, ph], PopupContent { tasks, rewards, columns_h, lines, actions, cursor })
}

/// The quest window's text: its state, what it needs, its subtitle and
/// description, wrapped to `text_w`.
fn popup_lines(s: &Screen<'_>, statuses: &[Status], n: usize, text_w: f32) -> Vec<(String, u32)> {
    let book = s.book;
    let node = &book.nodes()[n];
    let quest = book.quest(n);
    let mut lines: Vec<(String, u32)> = Vec::new();
    let (colour, state) = state_look(statuses[n].state);
    let mut state = state.to_owned();
    if quest.optional {
        state.push_str(", optional");
    }
    if quest.repeatable {
        state.push_str(", repeatable");
        match s.progress.repeats.get(&node.id).copied().unwrap_or(0) {
            0 => {}
            1 => state.push_str(", done once"),
            times => state.push_str(&format!(", done {times} times")),
        }
    }
    lines.extend(wrap(&state, text_w, s.measure).into_iter().map(|line| (line, colour & 0xFF_FFFF)));
    if statuses[n].state == QuestState::Locked {
        lines.push(("Needs:".to_owned(), GOLD_INK));
        for &d in &node.dependencies {
            let dependency = book.quest(d);
            let name = if !statuses[d].visible {
                "???"
            } else if dependency.title.is_empty() {
                &dependency.id
            } else {
                &dependency.title
            };
            let mark = if s.progress.is_completed(&book.nodes()[d].id) { "[x]" } else { "[ ]" };
            lines.push((fit_text(&format!("{mark} {name}"), text_w, s.measure), DIM_INK));
        }
    }
    if let Some(subtitle) = &quest.subtitle {
        lines.extend(wrap(subtitle, text_w, s.measure).into_iter().map(|line| (line, DIM_INK)));
    }
    for paragraph in &quest.description {
        lines.push((String::new(), INK));
        lines.extend(wrap(paragraph, text_w, s.measure).into_iter().map(|line| (line, INK)));
    }
    lines
}

/// One icon of the quest window: the item shown (an id, or `#tag`), its
/// count, what a click does, its tooltip's lines, a veil over it (a task
/// that waits) and a mark at its top-right corner.
struct Icon {
    item: String,
    count: String,
    action: Action,
    tip: Vec<(String, u32)>,
    veil: Option<u32>,
    mark: Option<Mark>,
}

/// A small sign on an icon's top-right corner.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mark {
    /// Done: a task completed, a reward claimed.
    Check,
    /// A reward waits to be claimed.
    Alert,
    /// A random reward (not claimable yet): what it gives is not known.
    Question,
}

/// The "+" that ends a column of the quest window in edit mode.
const PLUS_ICON: &str = "horizon_quests:plus";

fn plus_icon(tip: &str, action: Action) -> Icon {
    Icon { item: PLUS_ICON.to_owned(), count: String::new(), action, tip: vec![(tip.to_owned(), CLICK_INK)], veil: None, mark: None }
}

/// The item that stands for a task.
fn task_item(kind: &TaskKind) -> String {
    match kind {
        TaskKind::Item { item, .. } | TaskKind::Craft { item, .. } => String::from(item.clone()),
        TaskKind::Checkmark => CHECK_TASK_ICON.to_owned(),
        TaskKind::Kill { entity, .. } => format!("{entity}_spawn_egg"),
        TaskKind::XpLevels { .. } => "minecraft:experience_bottle".to_owned(),
        TaskKind::Dimension { dimension } if dimension.contains("nether") => "minecraft:netherrack".to_owned(),
        TaskKind::Dimension { dimension } if dimension.contains("end") => "minecraft:end_stone".to_owned(),
        TaskKind::Dimension { .. } => "minecraft:grass_block".to_owned(),
        TaskKind::Location { .. } => "minecraft:compass".to_owned(),
        TaskKind::Advancement { .. } => "minecraft:knowledge_book".to_owned(),
    }
}

/// The item that stands for something given, and its small count.
fn reward_item(kind: &RewardKind) -> (String, String) {
    let count = |n: u32| if n > 1 { n.to_string() } else { String::new() };
    match kind {
        RewardKind::Item { item, count: n, max_count: Some(max) } if max > n => (item.clone(), format!("{n}-{max}")),
        RewardKind::Item { item, count: n, .. } => (item.clone(), count(*n)),
        RewardKind::Xp { points } => ("minecraft:experience_bottle".to_owned(), points.to_string()),
        RewardKind::XpLevels { levels } => ("minecraft:experience_bottle".to_owned(), levels.to_string()),
        RewardKind::Command { .. } => ("minecraft:command_block".to_owned(), String::new()),
        RewardKind::Toast { .. } => ("minecraft:paper".to_owned(), String::new()),
        RewardKind::Random { .. } | RewardKind::Choice { .. } => ("minecraft:chest".to_owned(), String::new()),
    }
}

/// A click on an item's icon that does nothing else shows its recipes.
fn recipe_of(kind_item: Option<String>) -> Action {
    kind_item.map_or(Action::Stay, Action::ShowRecipe)
}

/// The colours of an item's tooltip, as Minecraft's: the mod's name in
/// blue, what a click does in yellow.
const MOD_INK: u32 = 0x5555_FF;
const CLICK_INK: u32 = 0xFFFF_55;

/// What an item is called: its name in the player's language when the game
/// gave it, else one made from its id; a tag is "any" of it.
fn item_name(names: &HashMap<String, String>, item: &str) -> String {
    match item.strip_prefix('#') {
        Some(tag) => format!("Any {}", pretty(tag)),
        None => names.get(item).cloned().unwrap_or_else(|| pretty(item)),
    }
}

/// The mod an item comes from, as Minecraft's tooltips name it.
fn mod_name(item: &str) -> String {
    let namespace = item.trim_start_matches('#').split_once(':').map_or("minecraft", |(ns, _)| ns);
    if namespace == "minecraft" { "Minecraft".to_owned() } else { pretty(namespace) }
}

/// A tooltip like an item's in Minecraft: its name, its mod in blue, then
/// `more` lines, then what a click does in yellow. Something that is not an
/// item (`item` None) is named by `label`.
fn item_tip(names: &HashMap<String, String>, item: Option<&str>, label: &str, more: Vec<(String, u32)>, click: &str) -> Vec<(String, u32)> {
    let mut lines = match item {
        Some(item) => vec![(item_name(names, item), INK), (mod_name(item), MOD_INK)],
        None => vec![(label.to_owned(), INK)],
    };
    lines.extend(more);
    if !click.is_empty() {
        lines.push((click.to_owned(), CLICK_INK));
    }
    lines
}

/// The item a task names, when it is an item or a craft task.
fn task_link(kind: &TaskKind) -> Option<String> {
    match kind {
        TaskKind::Item { item, .. } | TaskKind::Craft { item, .. } => Some(String::from(item.clone())),
        _ => None,
    }
}

/// The quest's tasks as icons: a click ticks a checkmark, hands items in, or
/// shows an item's recipes.
fn task_icons(s: &Screen<'_>, names: &HashMap<String, String>, n: usize) -> Vec<Icon> {
    let book = s.book;
    let node = &book.nodes()[n];
    let quest = book.quest(n);
    let completed = s.progress.is_completed(&node.id);
    quest
        .tasks
        .iter()
        .enumerate()
        .map(|(t, task)| {
            let id = book.task_id(n, t);
            let goal = task.kind.goal();
            let done = s.progress.task(&id).min(goal);
            let finished = done >= goal || completed;
            let counts = book.task_counts(n, t, s.progress);
            let link = task_link(&task.kind);
            let (action, click) = match task.kind {
                _ if !counts || finished => (recipe_of(link.clone()), ""),
                TaskKind::Checkmark => (Action::Check(id.clone()), "Click to tick it"),
                TaskKind::Item { consume: true, .. } => (Action::Submit(id.clone()), "Click to hand in"),
                _ => (recipe_of(link.clone()), ""),
            };
            let click = if click.is_empty() && matches!(action, Action::ShowRecipe(_)) { "Click to view recipe(s)" } else { click };
            let mut more = Vec::new();
            if link.is_some() {
                // The item's name says what; the task says what to do with it.
                more.push((task_label(task), DIM_INK));
            }
            if finished {
                more.push(("Done".to_owned(), LINE_DONE & 0xFF_FFFF));
            } else if goal > 1 {
                more.push((format!("Progress: {done}/{goal}"), DIM_INK));
            }
            if !counts && !finished && quest.sequential_tasks {
                more.push(("Waiting for the tasks before it".to_owned(), DIM_INK));
            }
            Icon {
                item: if finished && task.kind == TaskKind::Checkmark { CHECK_TASK_DONE.to_owned() } else { task_item(&task.kind) },
                count: if goal > 1 { goal.to_string() } else { String::new() },
                action,
                tip: item_tip(names, link.as_deref(), &task_label(task), more, click),
                veil: (!counts && !finished).then_some(DIM_VEIL),
                // A checkmark task shows it is done by its green check.
                mark: (finished && task.kind != TaskKind::Checkmark).then_some(Mark::Check),
            }
        })
        .collect()
}

/// Window ticks each entry of a random or choice reward is shown (0.8 s).
const TABLE_TURN: u32 = 16;

/// A random or choice reward's icon: its entries' items in turn, one every
/// `TABLE_TURN` window ticks, with the shown entry's count.
fn table_icon(table: Option<&RewardTable>, clock: u32) -> (String, String) {
    match table.filter(|table| !table.entries.is_empty()) {
        Some(table) => reward_item(&table.entries[(clock / TABLE_TURN) as usize % table.entries.len()].kind),
        None => ("minecraft:chest".to_owned(), String::new()),
    }
}

/// An entry's chance in a random reward, as a percentage: "33.33%", "50%".
fn chance(weight: u32, total: u32) -> String {
    let percent = format!("{:.2}", f64::from(weight) * 100.0 / f64::from(total.max(1)));
    format!("{}%", percent.trim_end_matches('0').trim_end_matches('.'))
}

/// The lines of a random or choice reward's tooltip: "- 4x Iron ingot",
/// with each entry's chance for a random reward.
fn table_lines(table: &RewardTable, chances: bool) -> Vec<(String, u32)> {
    let total: u32 = table.entries.iter().map(|entry| entry.weight).sum();
    let mut lines: Vec<(String, u32)> = table
        .entries
        .iter()
        .take(TABLE_ROWS)
        .map(|entry| {
            let line = if chances {
                format!("- {} [{}]", entry_label(entry), chance(entry.weight, total))
            } else {
                format!("- {}", entry_label(entry))
            };
            (line, TABLE_INK)
        })
        .collect();
    if table.entries.len() > TABLE_ROWS {
        lines.push((format!("and {} more", table.entries.len() - TABLE_ROWS), DIM_INK));
    }
    lines
}
const TABLE_INK: u32 = 0xD0D0D8;

/// The quest's rewards as icons: a click claims one, or picks an entry of a
/// choice reward; a random or choice reward shows its entries' items in
/// turn and lists them in its tooltip.
fn reward_icons(s: &Screen<'_>, names: &HashMap<String, String>, clock: u32, n: usize) -> Vec<Icon> {
    let book = s.book;
    let node = &book.nodes()[n];
    let quest = book.quest(n);
    let completed = s.progress.is_completed(&node.id);
    let link_of = |kind: &RewardKind| match kind {
        RewardKind::Item { item, .. } => Some(item.clone()),
        _ => None,
    };
    let mut icons = Vec::new();
    for (r, reward) in quest.rewards.iter().enumerate() {
        let id = book.reward_id(n, r);
        let claimed = s.progress.claimed_for(&id, s.player, reward.team);
        let team = reward.team.then(|| ("Team reward: one claim for the party".to_owned(), DIM_INK));
        let table = reward.kind.table().and_then(|table| book.table(table));
        // A random or choice reward claimed: only what it gave.
        let given = table.filter(|_| claimed).and_then(|table| {
            let index = s.progress.picked_for(&id, s.player, reward.team)?;
            table.entries.get(index as usize)
        });
        if let Some(entry) = given {
            let (item, count) = reward_item(&entry.kind);
            let link = link_of(&entry.kind);
            let mut more = team.into_iter().collect::<Vec<_>>();
            more.push(("Claimed".to_owned(), LINE_DONE & 0xFF_FFFF));
            let click = if link.is_some() { "Click to view recipe(s)" } else { "" };
            let tip = item_tip(names, link.as_deref(), &entry_label(entry), more, click);
            icons.push(Icon { item, count, action: recipe_of(link), tip, veil: None, mark: Some(Mark::Check) });
            continue;
        }
        if let (RewardKind::Choice { .. }, Some(table), false) = (&reward.kind, table, claimed) {
            // A choice: one icon; a click opens the window to pick from.
            let mut more = table_lines(table, false);
            more.extend(team.clone());
            let click = if completed { "Click to choose" } else { "Click to see the choices" };
            let tip = item_tip(names, None, &reward_label(reward), more, click);
            let (item, count) = table_icon(Some(table), clock);
            icons.push(Icon {
                item,
                count,
                action: Action::OpenChoice(id.clone()),
                tip,
                veil: None,
                mark: completed.then_some(Mark::Alert),
            });
            continue;
        }
        let (item, count) = if reward.kind.table().is_some() { table_icon(table, clock) } else { reward_item(&reward.kind) };
        let link = link_of(&reward.kind);
        let claimable = completed && !claimed;
        let action = if claimable { Action::Claim(id.clone()) } else { recipe_of(link.clone()) };
        let mut more = Vec::new();
        if let (RewardKind::Random { .. }, Some(table)) = (&reward.kind, table) {
            more.extend(table_lines(table, true));
        }
        more.extend(team);
        if claimed {
            more.push(("Claimed".to_owned(), LINE_DONE & 0xFF_FFFF));
        }
        let click = if claimable { "Click to claim" } else if link.is_some() { "Click to view recipe(s)" } else { "" };
        let tip = item_tip(names, link.as_deref(), &reward_label(reward), more, click);
        // A random reward not claimable yet: a "?", what it gives is not known.
        let random = matches!(reward.kind, RewardKind::Random { .. });
        let mark = if claimed {
            Some(Mark::Check)
        } else if claimable {
            Some(Mark::Alert)
        } else {
            random.then_some(Mark::Question)
        };
        icons.push(Icon { item, count, action, tip, veil: None, mark });
    }
    icons
}

/// A green check, and a red disc with a white "!", as pixel maps (one
/// character a pixel: `g`/`r` the sign, `w` white, `k` its dark edge).
const CHECK_PIXELS: [&str; 7] = ["........gg", ".......gg.", "......gg..", "gg...gg...", ".gg.gg....", "..ggg.....", "..gg......"];
const ALERT_PIXELS: [&str; 7] = [".rrrrr.", "rrrwrrr", "rrrwrrr", "rrrwrrr", "rrrrrrr", "rrrwrrr", ".rrrrr."];
/// A checkmark task's icon: no item, a round grey button with a check,
/// drawn by `check_task_draws` (white, or green once done).
const CHECK_TASK_ICON: &str = "horizon_quests:check_task";
const CHECK_TASK_DONE: &str = "horizon_quests:check_task_done";

/// Half a GUI pixel: the step of the finer pixel art (the counts, the
/// checkmark button), drawn at the screen's resolution as FTB-style books
/// draw their small text.
const FINE: f32 = 0.5;

/// Distance from `p` to the segment `a`-`b`.
fn segment_distance(p: [f32; 2], a: [f32; 2], b: [f32; 2]) -> f32 {
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let t = (((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / (dx * dx + dy * dy)).clamp(0.0, 1.0);
    let (x, y) = (a[0] + t * dx - p[0], a[1] + t * dy - p[1]);
    (x * x + y * y).sqrt()
}

/// The round grey button of a checkmark task, 16 pixels wide with its
/// top-left corner at `at`, drawn on a grid of half pixels: a dark rim, a
/// lighter top, a thick check with a soft shadow, white (green once done).
fn check_task_draws(at: [f32; 2], done: bool) -> Vec<Draw> {
    const N: usize = 32;
    let (short, corner, long) = ([8.5, 16.5], [13.5, 21.5], [23.5, 9.5]);
    let on_check = |x: f32, y: f32| segment_distance([x, y], short, corner).min(segment_distance([x, y], corner, long)) < 1.9;
    let rows: Vec<String> = (0..N)
        .map(|y| {
            (0..N)
                .map(|x| {
                    let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
                    let (dx, dy) = (px - 16.0, py - 16.0);
                    let d = (dx * dx + dy * dy).sqrt();
                    if d > 15.5 {
                        '.'
                    } else if d > 13.8 {
                        'k'
                    } else if on_check(px, py) {
                        'w'
                    } else if on_check(px - 1.0, py - 1.5) {
                        'd'
                    } else if d > 12.4 && dy < 0.0 {
                        'l'
                    } else if dy < -3.0 {
                        'h'
                    } else if dy > 6.0 {
                        's'
                    } else {
                        'g'
                    }
                })
                .collect()
        })
        .collect();
    let map: Vec<&str> = rows.iter().map(String::as_str).collect();
    // Grey, or green once done; the check stays white.
    let [rim, light, high, body, shade, shadow] = if done {
        [0xFF0E_3A0C, 0xFF9C_F488, 0xFF6C_DC58, 0xFF48_C03C, 0xFF2E_9E2A, 0xFF1E_6A1A]
    } else {
        [0xFF26_262C, 0xFFC8_C8D0, 0xFFA2_A2AA, 0xFF8C_8C94, 0xFF74_747C, 0xFF4E_4E56]
    };
    let pixels = pixel_art(&map, at, FINE, &|c| match c {
        'k' => Some(rim),
        'l' => Some(light),
        'h' => Some(high),
        'g' => Some(body),
        's' => Some(shade),
        'd' => Some(shadow),
        'w' => Some(CHECK_WHITE),
        _ => None,
    });
    let cell = if done { MarkCell::ButtonDone } else { MarkCell::Button };
    vec![mark_sprite(cell, [at[0], at[1], 16.0, 16.0], pixels)]
}
const CHECK_WHITE: u32 = 0xFFFA_FAFA;

/// Digits 5 half-pixels wide and 7 tall, then "-", for the counts on icons.
const DIGIT_PIXELS: [[&str; 7]; DIGIT_CELLS] = [
    [".www.", "w...w", "w..ww", "w.w.w", "ww..w", "w...w", ".www."],
    ["..w..", ".ww..", "..w..", "..w..", "..w..", "..w..", ".www."],
    [".www.", "w...w", "....w", "...w.", "..w..", ".w...", "wwwww"],
    [".www.", "w...w", "....w", "..ww.", "....w", "w...w", ".www."],
    ["...w.", "..ww.", ".w.w.", "w..w.", "wwwww", "...w.", "...w."],
    ["wwwww", "w....", "wwww.", "....w", "....w", "w...w", ".www."],
    ["..ww.", ".w...", "w....", "wwww.", "w...w", "w...w", ".www."],
    ["wwwww", "....w", "...w.", "..w..", ".w...", ".w...", ".w..."],
    [".www.", "w...w", "w...w", ".www.", "w...w", "w...w", ".www."],
    [".www.", "w...w", "w...w", ".wwww", "....w", "...w.", ".ww.."],
    [".....", ".....", ".....", ".www.", ".....", ".....", "....."],
];

/// The smooth digits of the counts: `digits.png`, the digits 0 to 9 then
/// "-" (a count range: "3-5") in cells of `DIGIT_CELL` pixels (made by `tools/make-digit-textures.py`).
pub const DIGIT_SPRITE: &str = "horizon_quests:gui/digits";
pub const DIGIT_CELL: [f32; 2] = [24.0, 32.0];
pub const DIGIT_CELLS: usize = 11;

/// A count's characters as cells of the digit sheet: 0 to 9, and 10 for "-".
fn count_cells(count: &str) -> Vec<usize> {
    count
        .chars()
        .filter_map(|c| if c == '-' { Some(10) } else { c.to_digit(10).map(|d| d as usize) })
        .collect()
}
/// A digit's cell on screen, in GUI pixels, and the step from one digit to
/// the next (the cells overlap where they are clear).
const DIGIT_SIZE: [f32; 2] = [4.2, 5.6];
const DIGIT_STEP: f32 = 3.2;

/// A count in small smooth white digits with a dark edge (the digit
/// sprite), in the bottom-right corner of the 16-pixel icon at `icon`, as
/// FTB-style books draw them: small, in the corner, readable on any item.
/// Without the sprite, the same in half-pixel art. Anything but digits and
/// "-" is left out.
fn count_draws(count: &str, icon: [f32; 2]) -> Vec<Draw> {
    let digits = count_cells(count);
    let n = digits.len() as f32;
    // The last cell's right edge: its digit ends about at the icon's edge.
    let right = icon[0] + 17.0;
    let top = icon[1] + 16.6 - DIGIT_SIZE[1];
    let left = right - DIGIT_SIZE[0] - (n - 1.0) * DIGIT_STEP;
    let mut fallback = Some(count_pixel_draws(count, icon));
    digits
        .iter()
        .enumerate()
        .map(|(i, &d)| Draw::Sprite {
            name: DIGIT_SPRITE.to_owned(),
            src: [d as f32 * DIGIT_CELL[0], 0.0, DIGIT_CELL[0], DIGIT_CELL[1]],
            dst: [left + i as f32 * DIGIT_STEP, top, DIGIT_SIZE[0], DIGIT_SIZE[1]],
            // The whole count once, under the first digit: the sprite is
            // there for all of them or for none.
            fallback: fallback.take().unwrap_or_default(),
        })
        .collect()
}

/// `count_draws` without the digit sprite: digits in half-pixel art with a
/// dark edge all round.
fn count_pixel_draws(count: &str, icon: [f32; 2]) -> Vec<Draw> {
    let digits = count_cells(count);
    if digits.is_empty() {
        return Vec::new();
    }
    let rows: Vec<String> = (0..7)
        .map(|r| digits.iter().map(|&d| DIGIT_PIXELS[d][r]).collect::<Vec<_>>().join("."))
        .collect();
    let rows: Vec<&str> = rows.iter().map(String::as_str).collect();
    let map = outlined(&rows, 'w', 'k');
    let map: Vec<&str> = map.iter().map(String::as_str).collect();
    let (w, h) = (map[0].len() as f32 * FINE, map.len() as f32 * FINE);
    // The edge may stick out half a pixel past the icon.
    let at = [icon[0] + 16.5 - w, icon[1] + 16.5 - h];
    pixel_art(&map, at, FINE, &|c| match c {
        'w' => Some(COUNT_INK),
        'k' => Some(COUNT_EDGE),
        _ => None,
    })
}
const COUNT_INK: u32 = 0xFFF6_F6F6;
const COUNT_EDGE: u32 = 0xFF20_2020;

/// The smooth signs: `marks.png`, cells of `MARK_CELL` pixels (made by
/// `tools/make-mark-textures.py`), in `MarkCell` order. Each is drawn as a
/// sprite with the pixel art below as its fallback.
pub const MARK_SPRITE: &str = "horizon_quests:gui/marks";
pub const MARK_CELL: f32 = 64.0;
pub const MARK_CELLS: usize = 6;

/// A cell of the sign sheet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MarkCell {
    Check,
    Question,
    Alert,
    Lock,
    Button,
    ButtonDone,
}

// The sheet holds one cell per sign.
const _: () = assert!(MARK_CELLS == MarkCell::ButtonDone as usize + 1);

/// Cell `cell` of the sign sheet stretched into `dst`, `fallback` without it.
fn mark_sprite(cell: MarkCell, dst: [f32; 4], fallback: Vec<Draw>) -> Draw {
    Draw::Sprite { name: MARK_SPRITE.to_owned(), src: [cell as u8 as f32 * MARK_CELL, 0.0, MARK_CELL, MARK_CELL], dst, fallback }
}

/// A padlock: `k` its dark edge and keyhole, `w` its metal, `s` the
/// metal's shaded bottom.
const LOCK_PIXELS: [&str; 10] = [
    "..kkkkk..",
    ".kwwwwwk.",
    ".kwkkkwk.",
    ".kwk.kwk.",
    "kkkkkkkkk",
    "kwwwwwwwk",
    "kwwwkwwwk",
    "kwwwkwwwk",
    "ksssssssk",
    "kkkkkkkkk",
];
/// Over a locked quest's icon on the canvas: a grey veil.
const LOCKED_VEIL: u32 = 0xB038_383E;
const LOCK_METAL: u32 = 0xFFD4_D4DA;
const LOCK_SHADE: u32 = 0xFF9C_9CA6;
const LOCK_EDGE: u32 = 0xFF1C_1C22;

/// The padlock on a locked node's bottom-right corner (the node's box at
/// `node`), bigger as the node grows.
fn lock_draws(node: [f32; 4]) -> Draw {
    // A third of the node, however far the canvas is zoomed out.
    let scale = node[2] * 0.034;
    let (w, h) = (LOCK_PIXELS[0].len() as f32 * scale, LOCK_PIXELS.len() as f32 * scale);
    let inset = node[2] * 0.06;
    let at = [node[0] + node[2] - w - inset, node[1] + node[3] - h - inset];
    let pixels = pixel_art(&LOCK_PIXELS, at, scale, &|c| match c {
        'k' => Some(LOCK_EDGE),
        'w' => Some(LOCK_METAL),
        's' => Some(LOCK_SHADE),
        _ => None,
    });
    // The smooth padlock: a square a little bigger than the pixel one.
    let size = h * 1.1;
    let dst = [node[0] + node[2] - size - inset + scale, node[1] + node[3] - size - inset + scale, size, size];
    mark_sprite(MarkCell::Lock, dst, pixels)
}

/// The colour of a chapter's name: green once it is `completed` (every
/// quest that counts, `Book::chapter_completed`; rewards still to claim are
/// the row's "!"), else blue once one of its quests (`states`) is started
/// or done, else grey.
pub fn chapter_ink(completed: bool, states: &[QuestState]) -> u32 {
    if completed {
        CHAPTER_DONE
    } else if states.iter().any(|s| matches!(s, QuestState::InProgress | QuestState::Done | QuestState::RewardsWaiting)) {
        CHAPTER_STARTED
    } else {
        CHAPTER_INK
    }
}

/// The red "!" badge on the top-right corner of a node whose rewards wait
/// (the node's box at `node`), bigger as the node grows: the sign sheet's
/// alert, or its pixel art without the sheet.
fn alert_badge_draws(node: [f32; 4]) -> Draw {
    let size = node[2] * 0.4;
    alert_sign([node[0] + node[2] - size * 0.7, node[1] - size * 0.3, size, size])
}

/// The red "!" sign stretched into `dst`: the sign sheet's alert, or its
/// pixel art without the sheet.
fn alert_sign(dst: [f32; 4]) -> Draw {
    let size = dst[2];
    let scale = size / ALERT_PIXELS.len() as f32;
    let pixels = pixel_art(&ALERT_PIXELS, [dst[0], dst[1]], scale, &|c| match c {
        'r' => Some(0xFFE0_3434),
        'w' => Some(0xFFFF_FFFF),
        _ => None,
    });
    mark_sprite(MarkCell::Alert, dst, pixels)
}

/// A "?" for a reward to choose.
const QUESTION_PIXELS: [&str; 7] = [".yyy.", "yy.yy", "...yy", "..yy.", "..yy.", ".....", "..yy."];

/// `map` with a one-pixel `edge` round every `fill` pixel (sides and
/// corners), the map grown by one pixel on each side.
fn outlined(map: &[&str], fill: char, edge: char) -> Vec<String> {
    let (h, w) = (map.len() + 2, map.iter().map(|row| row.len()).max().unwrap_or(0) + 2);
    let at = |r: isize, c: isize| -> bool {
        r >= 1 && c >= 1 && map.get((r - 1) as usize).and_then(|row| row.chars().nth((c - 1) as usize)) == Some(fill)
    };
    (0..h as isize)
        .map(|r| {
            (0..w as isize)
                .map(|c| {
                    if at(r, c) {
                        fill
                    } else if (-1..=1).any(|dr| (-1..=1).any(|dc| at(r + dr, c + dc))) {
                        edge
                    } else {
                        '.'
                    }
                })
                .collect()
        })
        .collect()
}

/// A push pin, upright: `p` its colour, `k` its needle.
const PIN_PIXELS: [&str; 9] = [".ppppp.", ".ppppp.", "..ppp..", "..ppp..", ".ppppp.", "ppppppp", "...k...", "...k...", "...k..."];

/// A pixel map drawn with its top-left corner at `at`, `scale` pixels a
/// character; `colour` gives each character's colour (None: nothing).
fn pixel_art(map: &[&str], at: [f32; 2], scale: f32, colour: &dyn Fn(char) -> Option<u32>) -> Vec<Draw> {
    let mut draws = Vec::new();
    for (row, line) in map.iter().enumerate() {
        let chars: Vec<char> = line.chars().collect();
        let mut column = 0;
        // A run of one colour is one fill.
        while column < chars.len() {
            let start = column;
            let here = colour(chars[column]);
            while column < chars.len() && colour(chars[column]) == here {
                column += 1;
            }
            if let Some(c) = here {
                let rect = [at[0] + start as f32 * scale, at[1] + row as f32 * scale, (column - start) as f32 * scale, scale];
                draws.push(Draw::Fill { rect, colour: c });
            }
        }
    }
    draws
}

/// A mark's pixels at an icon's top-right corner (the icon at `icon`, 16
/// pixels wide).
fn mark_draws(mark: Mark, icon: [f32; 2]) -> Vec<Draw> {
    let (cell, dst) = match mark {
        Mark::Check => (MarkCell::Check, [icon[0] + 8.0, icon[1] - 4.0, 11.0, 11.0]),
        Mark::Question => (MarkCell::Question, [icon[0] + 9.0, icon[1] - 3.5, 10.0, 10.0]),
        Mark::Alert => (MarkCell::Alert, [icon[0] + 9.5, icon[1] - 3.0, 9.0, 9.0]),
    };
    vec![mark_sprite(cell, dst, mark_pixel_draws(mark, icon))]
}

/// `mark_draws` without the sign sheet: the same signs in pixel art.
fn mark_pixel_draws(mark: Mark, icon: [f32; 2]) -> Vec<Draw> {
    match mark {
        Mark::Check => {
            // A thick green check with a dark edge all round.
            let map = outlined(&CHECK_PIXELS, 'g', 'k');
            let rows: Vec<&str> = map.iter().map(String::as_str).collect();
            pixel_art(&rows, [icon[0] + 8.0, icon[1] - 4.0], 1.0, &|c| match c {
                'g' => Some(0xFF5C_E83C),
                'k' => Some(0xFF0C_2208),
                _ => None,
            })
        }
        Mark::Question => {
            let map = outlined(&QUESTION_PIXELS, 'y', 'k');
            let rows: Vec<&str> = map.iter().map(String::as_str).collect();
            pixel_art(&rows, [icon[0] + 10.0, icon[1] - 3.0], 1.0, &|c| match c {
                'y' => Some(0xFFFF_D040),
                'k' => Some(0xFF2A_1C04),
                _ => None,
            })
        }
        Mark::Alert => pixel_art(&ALERT_PIXELS, [icon[0] + 10.0, icon[1] - 2.0], 1.0, &|c| match c {
            'r' => Some(0xFFE0_3434),
            'w' => Some(0xFFFF_FFFF),
            _ => None,
        }),
    }
}

/// The pin that tracks a quest: gold while it is tracked, grey otherwise,
/// white under the pointer.
const PIN_SIZE: [f32; 2] = [7.0, 9.0];
fn pin_draws(at: [f32; 2], tracked: bool, hovered: bool) -> Vec<Draw> {
    let body = if hovered {
        0xFFFF_FFFF
    } else if tracked {
        0xFFFF_C040
    } else {
        0xFF9A_9AAA
    };
    pixel_art(&PIN_PIXELS, at, 1.0, &|c| match c {
        'p' => Some(body),
        'k' => Some(0xFFC8_C8D0),
        _ => None,
    })
}

/// A column of the quest window: its heading, then its icons in rows,
/// centred. Returns the tooltip of the icon under the pointer.
fn draw_icons(scene: &mut Scene, s: &Screen<'_>, area: [f32; 4], heading: &str, ink: u32, icons: &[Icon]) -> Option<Vec<(String, u32)>> {
    let [x, y, w, h] = area;
    let heading = fit_text(heading, w - 2.0 * PAD, s.measure);
    scene.text(heading.clone(), [x + (w - (s.measure)(&heading)) / 2.0, y + PAD], ink);
    let top = y + PAD + TEXT + PAD;
    let per_row = ((w - 2.0 * PAD) / ICON_CELL).floor().max(1.0) as usize;
    let rows_room = ((y + h - top) / ICON_CELL).floor().max(0.0) as usize;
    let shown = icons.len().min(ICONS_MAX).min(per_row * rows_room);
    let mut tip = None;
    for (i, icon) in icons.iter().take(shown).enumerate() {
        let row = i / per_row;
        let in_row = (shown - row * per_row).min(per_row);
        let row_left = x + (w - in_row as f32 * ICON_CELL) / 2.0;
        let cell = [row_left + (i % per_row) as f32 * ICON_CELL, top + row as f32 * ICON_CELL, ICON_CELL, ICON_CELL];
        let at = [cell[0] + 2.0, cell[1] + 2.0];
        if canvas::inside(cell, s.mouse) {
            scene.fill(cell, ROW_HOVER);
            tip = Some(icon.tip.clone());
        }
        if icon.item == PLUS_ICON {
            scene.fill([at[0] + 2.0, at[1] + 7.0, 12.0, 2.0], 0xFF60_D060);
            scene.fill([at[0] + 7.0, at[1] + 2.0, 2.0, 12.0], 0xFF60_D060);
        } else if icon.item == CHECK_TASK_ICON || icon.item == CHECK_TASK_DONE {
            scene.draws.extend(check_task_draws(at, icon.item == CHECK_TASK_DONE));
        } else {
            scene.draws.push(Draw::Item { item: icon.item.clone(), at, size: 16.0 });
        }
        if let Some(veil) = icon.veil {
            scene.fill([at[0], at[1], 16.0, 16.0], veil);
        }
        // The count in small digits, in the icon's bottom-right corner.
        scene.draws.extend(count_draws(&icon.count, at));
        if let Some(mark) = icon.mark {
            scene.draws.extend(mark_draws(mark, at));
        }
        scene.zones.push((cell, icon.action.clone()));
    }
    if icons.len() > shown {
        let more = format!("+{}", icons.len() - shown);
        scene.text(more.clone(), [x + w - PAD - (s.measure)(&more), y + h - TEXT], DIM_INK);
    }
    if icons.is_empty() {
        let none = "None";
        scene.text(none, [x + (w - (s.measure)(none)) / 2.0, top + 4.0], DIM_INK);
    }
    tip
}

/// The window to pick an entry of choice reward `reward`, over the book:
/// each entry with its icon and name; a click takes it once the quest is
/// done. A click outside closes it. Returns the tooltip under the pointer.
fn draw_choice(scene: &mut Scene, s: &Screen<'_>, names: &HashMap<String, String>, reward: &str) -> Option<Vec<(String, u32)>> {
    let book = s.book;
    let (n, r) = book.find_reward(reward)?;
    let found = &book.quest(n).rewards[r];
    let table = found.kind.table().and_then(|table| book.table(table))?;
    let completed = s.progress.is_completed(&book.nodes()[n].id);
    scene.zones.push((s.rect, Action::CloseChoice));
    let entries = table.entries.len().min(TABLE_ROWS);
    let b = over_book(s, scene, [CHOICE_WIDTH, POPUP_BAR + 2.0 * PAD + TEXT + entries as f32 * ICON_CELL + PAD]);
    let [_, _, bw, bh] = b;
    scene.fill(b, CHOICE_BACK);
    scene.fill([b[0], b[1], bw, 1.0], POPUP_EDGE);
    scene.fill([b[0], b[1] + bh - 1.0, bw, 1.0], POPUP_EDGE);
    scene.fill([b[0], b[1], 1.0, bh], POPUP_EDGE);
    scene.fill([b[0] + bw - 1.0, b[1], 1.0, bh], POPUP_EDGE);
    scene.zones.push((b, Action::Stay));
    scene.fill([b[0] + 1.0, b[1] + 1.0, bw - 2.0, POPUP_BAR], BAND);
    let close = scene.button(s, "x", b[0] + bw - PAD, b[1] + 4.0, Action::CloseChoice);
    let title = fit_text("Choose one reward", close - PAD - b[0] - PAD, s.measure);
    scene.text(title, [b[0] + PAD, b[1] + 5.0], GOLD_INK);
    let say = if completed { "Click the one you want." } else { "Finish the quest, then pick one." };
    let top = b[1] + 1.0 + POPUP_BAR + PAD;
    scene.text(fit_text(say, bw - 2.0 * PAD, s.measure), [b[0] + PAD, top], DIM_INK);
    let mut tip = None;
    for (e, entry) in table.entries.iter().take(entries).enumerate() {
        let row = [b[0] + 1.0, top + TEXT + PAD + e as f32 * ICON_CELL, bw - 2.0, ICON_CELL];
        if !contains(b, row) {
            continue;
        }
        let (item, count) = reward_item(&entry.kind);
        let link = match &entry.kind {
            RewardKind::Item { item, .. } => Some(item.as_str()),
            _ => None,
        };
        if canvas::inside(row, s.mouse) {
            if completed {
                scene.fill(row, ROW_HOVER);
            }
            let click = if completed { "Click to take this one" } else { "" };
            tip = Some(item_tip(names, link, &entry_label(entry), Vec::new(), click));
        }
        let at = [row[0] + PAD, row[1] + 2.0];
        scene.draws.push(Draw::Item { item, at, size: 16.0 });
        scene.draws.extend(count_draws(&count, at));
        let label = fit_text(&entry_label(entry), bw - 2.0 * PAD - 24.0, s.measure);
        scene.text(label, [at[0] + 22.0, row[1] + 6.0], if completed { INK } else { DIM_INK });
        let action = if completed { Action::Choose(reward.to_owned(), e as u32) } else { Action::Stay };
        scene.zones.push((row, action));
    }
    tip
}

/// Where a window over the book goes (the choice window, the recipe view):
/// centred on the quest window when one is open, else on the canvas, and
/// kept on screen.
fn over_book(s: &Screen<'_>, scene: &Scene, size: [f32; 2]) -> [f32; 4] {
    let [cx, cy] = canvas::centre(scene.popup.unwrap_or(scene.regions.canvas));
    let [x, y, w, h] = s.rect;
    let (bw, bh) = (size[0].min(w), size[1].min(h));
    [(cx - bw / 2.0).clamp(x, x + w - bw), (cy - bh / 2.0).clamp(y, y + h - bh), bw, bh]
}

/// The choice window's width.
const CHOICE_WIDTH: f32 = 200.0;

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

/// A recipe cell: an 18-pixel square holding a 16-pixel icon.
const CELL: f32 = 18.0;
const RECIPE_BOX: [f32; 2] = [200.0, 120.0];
const CELL_BACK: u32 = 0xFF2E_2E3C;

/// The recipe view, centred over the book. A click outside closes it.
fn draw_recipe(scene: &mut Scene, s: &Screen<'_>, recipe: &RecipeOverlay) {
    scene.zones.push((s.rect, Action::CloseRecipe));
    let b = over_book(s, scene, RECIPE_BOX);
    let bw = b[2];
    scene.fill(b, PANEL);
    scene.zones.push((b, Action::Stay));
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

/// How much bigger the node under the pointer is drawn.
const HOVER_GROW: f32 = 1.12;

/// `b` grown by `factor` round its centre.
fn grow(b: [f32; 4], factor: f32) -> [f32; 4] {
    let [cx, cy] = canvas::centre(b);
    let (w, h) = (b[2] * factor, b[3] * factor);
    [cx - w / 2.0, cy - h / 2.0, w, h]
}

/// Lines by the pointer (an item's tooltip, a quest's title), in a dark box
/// with a violet frame, kept on screen.
fn draw_tooltip(scene: &mut Scene, s: &Screen<'_>, lines: &[(String, u32)]) {
    let [x, y, w, h] = s.rect;
    let widest = lines.iter().map(|(text, _)| (s.measure)(text)).fold(0.0, f32::max);
    let (width, height) = (widest + 8.0, lines.len() as f32 * TEXT + 6.0);
    let left = (s.mouse[0] + 10.0).min(x + w - width).max(x);
    let top = (s.mouse[1] - 16.0).min(y + h - height).max(y);
    scene.fill([left, top, width, height], TOOLTIP);
    scene.fill([left + 1.0, top + 1.0, width - 2.0, 1.0], TOOLTIP_EDGE_TOP);
    scene.fill([left + 1.0, top + height - 2.0, width - 2.0, 1.0], TOOLTIP_EDGE_BOTTOM);
    scene.fill([left + 1.0, top + 2.0, 1.0, height - 4.0], TOOLTIP_EDGE_TOP);
    scene.fill([left + width - 2.0, top + 2.0, 1.0, height - 4.0], TOOLTIP_EDGE_BOTTOM);
    for (i, (text, colour)) in lines.iter().enumerate() {
        scene.text(text.clone(), [left + 4.0, top + 3.0 + i as f32 * TEXT], *colour);
    }
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

    /// The middle of quest `id`'s node on screen.
    fn node_at(view: &BookView, book: &Book, progress: &Progress, id: &str) -> [f32; 2] {
        let n = book.find(id).unwrap();
        zone(&view.scene(&screen(book, progress, NOWHERE)), &Action::Quest(n)).unwrap()
    }

    #[test]
    fn the_pencil_shows_for_editors_only_and_turns_edit_mode_on() {
        let (book, progress) = (book(), Progress::default());
        let mut view = opened(&book, &progress);
        assert!(zone(&view.scene(&screen(&book, &progress, NOWHERE)), &Action::ToggleEdit).is_none());
        // Not an editor: right clicks do nothing.
        assert_eq!(view.right_click(&screen(&book, &progress, node_at(&view, &book, &progress, "a/start"))), Reaction::Nothing);
        assert!(view.menu.is_none());
        view.editor = true;
        click(&mut view, &book, &progress, &Action::ToggleEdit);
        assert!(view.editing);
        assert!(texts(&view.scene(&screen(&book, &progress, NOWHERE))).iter().any(|t| t.starts_with("Editing")));
        click(&mut view, &book, &progress, &Action::ToggleEdit);
        assert!(!view.editing);
    }

    #[test]
    fn a_right_click_on_the_canvas_offers_a_new_quest_there() {
        let (book, progress) = (book(), Progress::default());
        let mut view = opened(&book, &progress);
        (view.editor, view.editing, view.source) = (true, true, edit::SOURCE_WORLD.to_owned());
        // Two cells right of the second quest, an empty spot.
        let next = node_at(&view, &book, &progress, "a/next");
        let spot = [next[0], next[1] + 2.0 * canvas::GRID * view.view.zoom];
        view.right_click(&screen(&book, &progress, spot));
        let menu = view.menu.clone().unwrap();
        assert!(menu.entries.iter().any(|(_, a)| *a == Action::AskUseModpack));
        let reaction = click(&mut view, &book, &progress, &menu.entries[0].1);
        assert_eq!(reaction, Reaction::Edit(Edit::NewQuest { chapter: "a".into(), x: 2.0, y: 2.0 }));
        assert!(view.menu.is_none());
    }

    #[test]
    fn a_right_click_on_a_quest_offers_link_picking_and_asks_before_deleting() {
        let (book, progress) = (book(), Progress::default());
        let mut view = opened(&book, &progress);
        (view.editor, view.editing) = (true, true);
        let (start, next) = (book.find("a/start").unwrap(), book.find("a/next").unwrap());
        let at_next = node_at(&view, &book, &progress, "a/next");
        view.right_click(&screen(&book, &progress, at_next));
        let menu = view.menu.clone().unwrap();
        assert_eq!(menu.title.as_deref(), Some("Then this"));
        let labels: Vec<&str> = menu.entries.iter().map(|(l, _)| l.as_str()).collect();
        assert_eq!(labels, ["Settings...", "Duplicate", "Pick what this quest needs...", "Pick what this quest opens...", "Delete"]);
        // Picking what "next" needs: it needs "start" already, so a click unlinks it.
        click(&mut view, &book, &progress, &Action::Link(next, true));
        let reaction = click(&mut view, &book, &progress, &Action::Quest(start));
        assert_eq!(reaction, Reaction::Edit(Edit::Link { quest: "a/next".into(), needs: "a/start".into(), on: false }));
        // Picking what "far" opens: a click on "start" makes "start" need "far".
        let far = book.find("a/far").unwrap();
        view.linking = Some((far, false));
        let reaction = click(&mut view, &book, &progress, &Action::Quest(start));
        assert_eq!(reaction, Reaction::Edit(Edit::Link { quest: "a/start".into(), needs: "a/far".into(), on: true }));
        // Escape ends the picking.
        assert!(view.close_menu());
        assert!(!view.close_menu());

        view.right_click(&screen(&book, &progress, at_next));
        click(&mut view, &book, &progress, &Action::AskDelete("a/next".into()));
        let confirm = view.menu.clone().unwrap();
        assert_eq!(confirm.title.as_deref(), Some("Delete \"Then this\"?"));
        let reaction = click(&mut view, &book, &progress, &Action::Edit(Edit::DeleteQuest { quest: "a/next".into() }));
        assert_eq!(reaction, Reaction::Edit(Edit::DeleteQuest { quest: "a/next".into() }));
        // A second right click closes a menu instead of opening another.
        view.right_click(&screen(&book, &progress, at_next));
        view.right_click(&screen(&book, &progress, at_next));
        assert!(view.menu.is_none());
    }

    #[test]
    fn in_edit_mode_a_quest_drags_to_a_half_cell_and_a_click_still_opens_it() {
        let (book, progress) = (book(), Progress::default());
        let mut view = opened(&book, &progress);
        (view.editor, view.editing) = (true, true);
        let start = node_at(&view, &book, &progress, "a/start");
        let cell = canvas::GRID * view.view.zoom;
        // Pressed and released in place: the window opens.
        view.click(&screen(&book, &progress, start));
        assert_eq!(view.selected, None);
        assert_eq!(view.release(&screen(&book, &progress, start)), Reaction::Nothing);
        assert_eq!(view.selected, book.find("a/start"));
        view.selected = None;
        // Dragged one and a half cells down (and a little more): dropped there.
        view.click(&screen(&book, &progress, start));
        let to = [start[0] + 0.1 * cell, start[1] + 1.6 * cell];
        view.drag(&screen(&book, &progress, to), [0.1 * cell, 1.6 * cell]);
        let before = view.view;
        let reaction = view.release(&screen(&book, &progress, to));
        assert_eq!(reaction, Reaction::Edit(Edit::MoveQuest { quest: "a/start".into(), x: 0.0, y: 1.5 }));
        assert_eq!(view.view, before, "the canvas did not pan");
        assert_eq!(view.selected, None);
    }

    #[test]
    fn the_open_quest_stays_open_when_the_pack_changes_under_it() {
        let (book, progress) = (book(), Progress::default());
        let mut view = opened(&book, &progress);
        view.selected = book.find("a/next");
        let mut file = book.file().clone();
        edit::apply(&mut file, &Edit::DeleteQuest { quest: "a/start".into() }).unwrap();
        let smaller = Book::new(file.clone());
        view.pack_changed(Some(&book), &smaller, None);
        assert_eq!(view.selected, smaller.find("a/next"));
        edit::apply(&mut file, &Edit::DeleteQuest { quest: "a/next".into() }).unwrap();
        let smallest = Book::new(file);
        view.pack_changed(Some(&smaller), &smallest, None);
        assert_eq!(view.selected, None);
        view.pack_changed(Some(&smallest), &smallest, Some("a/far"));
        assert_eq!(view.selected, smallest.find("a/far"));
        // Link picking goes on in the new pack.
        view.linking = Some((book.find("a/far").unwrap(), true));
        view.pack_changed(Some(&book), &smallest, None);
        assert_eq!(view.linking, Some((smallest.find("a/far").unwrap(), true)));
    }

    #[test]
    fn in_edit_mode_the_title_and_description_are_typed_in_place() {
        let (book, progress) = (book(), Progress::default());
        let mut view = opened(&book, &progress);
        (view.editor, view.editing) = (true, true);
        view.selected = book.find("a/start");
        assert!(view.key(&screen(&book, &progress, NOWHERE), "key.keyboard.k").is_none());
        click(&mut view, &book, &progress, &Action::Type(Typed::Title("a/start".into())));
        // Every key is the book's while typing: "k" must not close it.
        assert_eq!(view.key(&screen(&book, &progress, NOWHERE), "key.keyboard.k"), Some(Reaction::Nothing));
        assert!(view.typed('!'));
        // A click elsewhere keeps the text.
        let Reaction::Edit(Edit::SetQuestInfo { quest, info }) = view.click(&screen(&book, &progress, NOWHERE)) else {
            panic!("the title was not kept");
        };
        assert_eq!((quest.as_str(), info.title.as_str()), ("a/start", "Start here!"));
        assert!(view.typing.is_none());
        // The description: Enter starts a paragraph, Escape keeps it all.
        view.selected = book.find("a/start");
        click(&mut view, &book, &progress, &Action::Type(Typed::Description("a/start".into())));
        let s = screen(&book, &progress, NOWHERE);
        view.key(&s, "key.keyboard.enter");
        "Two.".chars().for_each(|c| assert!(view.typed(c)));
        let Some(Reaction::Edit(Edit::SetQuestInfo { info, .. })) = view.key(&s, "key.keyboard.escape") else {
            panic!("the description was not kept");
        };
        assert_eq!(info.description.len(), 2);
        assert_eq!(info.description[1], "Two.");
        // Nothing changed: nothing is sent.
        click(&mut view, &book, &progress, &Action::Type(Typed::Subtitle("a/start".into())));
        assert_eq!(view.key(&s, "key.keyboard.enter"), Some(Reaction::Nothing));
    }

    #[test]
    fn the_icon_opens_the_picker_and_a_choice_sets_it() {
        let (book, progress) = (book(), Progress::default());
        let mut view = opened(&book, &progress);
        (view.editor, view.editing) = (true, true);
        view.item_names = [("minecraft:diamond", "Diamond"), ("minecraft:dirt", "Dirt")].into_iter().map(|(a, b)| (a.to_owned(), b.to_owned())).collect();
        view.selected = book.find("a/start");
        click(&mut view, &book, &progress, &Action::PickFor(Purpose::QuestIcon("a/start".into()), false));
        "dia".chars().for_each(|c| assert!(view.typed(c)));
        let scene = view.scene(&screen(&book, &progress, NOWHERE));
        assert!(zone(&scene, &Action::Pick(PickAction::Choose("minecraft:dirt".into()))).is_none());
        let reaction = click(&mut view, &book, &progress, &Action::Pick(PickAction::Choose("minecraft:diamond".into())));
        let Reaction::Edit(Edit::SetQuestInfo { info, .. }) = reaction else { panic!("{reaction:?}") };
        assert_eq!(info.icon.as_deref(), Some("minecraft:diamond"));
        assert!(view.picker.is_none());
    }

    #[test]
    fn in_edit_mode_the_quest_window_adds_tasks_and_opens_their_panels() {
        let (book, progress) = (book(), Progress::default());
        let mut view = opened(&book, &progress);
        view.selected = book.find("a/start");
        let scene = view.scene(&screen(&book, &progress, NOWHERE));
        assert!(zone(&scene, &Action::Check("a/start/ok".into())).is_some());
        assert!(zone(&scene, &Action::AddTask("a/start".into())).is_none());
        (view.editor, view.editing) = (true, true);
        let scene = view.scene(&screen(&book, &progress, NOWHERE));
        assert!(zone(&scene, &Action::Check("a/start/ok".into())).is_none());
        for action in [
            Action::OpenPanel(Subject::Task("a/start/ok".into())),
            Action::OpenPanel(Subject::Reward("a/start/xp".into())),
            Action::AddReward("a/start".into()),
            Action::OpenPanel(Subject::Quest("a/start".into())),
        ] {
            assert!(zone(&scene, &action).is_some(), "{action:?}");
        }
        // "+": a menu with an icon a row; "Tick a box" makes the task at once.
        click(&mut view, &book, &progress, &Action::AddTask("a/start".into()));
        let menu = view.menu.clone().unwrap();
        assert_eq!(menu.icons.len(), menu.entries.len());
        let tick = menu.entries.iter().find(|(label, _)| label == "Tick a box").unwrap().1.clone();
        let Reaction::Edit(change) = click(&mut view, &book, &progress, &tick) else { panic!("no edit") };
        let mut file = book.file().clone();
        let made = edit::apply(&mut file, &change).unwrap();
        assert_eq!(made.as_deref(), Some("a/start/checkmark"));
        // The pack comes back: the new task's panel is open, its quest too.
        let bigger = Book::new(file);
        view.pack_changed(Some(&book), &bigger, made.as_deref());
        assert_eq!(view.panel, Some(Subject::Task("a/start/checkmark".into())));
        assert_eq!(view.selected, bigger.find("a/start"));
        // Its panel's red button deletes it.
        let reaction = click(&mut view, &bigger, &progress, &Action::Panel(PanelAction::Press("delete")));
        assert_eq!(reaction, Reaction::Edit(Edit::RemoveTask { quest: "a/start".into(), task: "checkmark".into() }));
        assert!(view.panel.is_none());
        // Closing the quest's window closes a panel of its quest.
        view.panel = Some(Subject::Quest("a/start".into()));
        view.close_quest();
        assert!(view.panel.is_none());
    }

    #[test]
    fn the_wheel_over_a_panel_number_steps_it() {
        let (book, progress) = (book(), Progress::default());
        let mut view = opened(&book, &progress);
        (view.editor, view.editing) = (true, true);
        view.selected = book.find("a/start");
        view.panel = Some(Subject::Quest("a/start".into()));
        let at = zone(&view.scene(&screen(&book, &progress, NOWHERE)), &Action::Panel(PanelAction::Step("cooldown", 0.0))).unwrap();
        let reaction = view.scroll(&screen(&book, &progress, at), 1.0);
        let Reaction::Edit(Edit::SetQuestInfo { info, .. }) = reaction else { panic!("{reaction:?}") };
        assert_eq!(info.repeat_cooldown, 60);
        // A click on the number itself does nothing.
        assert_eq!(click(&mut view, &book, &progress, &Action::Panel(PanelAction::Step("cooldown", 0.0))), Reaction::Nothing);
    }

    #[test]
    fn in_edit_mode_the_chapter_list_makes_moves_and_deletes_chapters() {
        let (book, progress) = (book(), Progress::default());
        let mut view = opened(&book, &progress);
        view.sidebar_open = true;
        let scene = view.scene(&screen(&book, &progress, NOWHERE));
        assert!(zone(&scene, &Action::Edit(Edit::NewChapter { title: "New chapter".into() })).is_none());
        (view.editor, view.editing) = (true, true);
        let new_chapter = Action::Edit(Edit::NewChapter { title: "New chapter".into() });
        let reaction = click(&mut view, &book, &progress, &new_chapter);
        assert_eq!(reaction, Reaction::Edit(Edit::NewChapter { title: "New chapter".into() }));
        // A right click on a chapter row: its menu.
        view.sidebar_open = true;
        let row = zone(&view.scene(&screen(&book, &progress, NOWHERE)), &Action::Chapter(1)).unwrap();
        view.right_click(&screen(&book, &progress, row));
        let menu = view.menu.clone().unwrap();
        assert_eq!(menu.title.as_deref(), Some("Beta"));
        assert!(menu.entries.contains(&("Move up".to_owned(), Action::Edit(Edit::MoveChapter { chapter: "b".into(), by: -1 }))));
        click(&mut view, &book, &progress, &Action::AskDeleteChapter("b".into()));
        assert!(view.menu.as_ref().unwrap().entries[0].1 == Action::Edit(Edit::DeleteChapter { chapter: "b".into() }));

        // A new chapter made: the book turns to it; a pack where chapters moved keeps the one shown.
        let mut file = book.file().clone();
        edit::apply(&mut file, &Edit::NewChapter { title: "Gamma".into() }).unwrap();
        let bigger = Book::new(file.clone());
        view.pack_changed(Some(&book), &bigger, Some("gamma"));
        assert_eq!(bigger.file().chapters[view.chapter].id, "gamma");
        edit::apply(&mut file, &Edit::DeleteChapter { chapter: "a".into() }).unwrap();
        let smaller = Book::new(file);
        view.pack_changed(Some(&bigger), &smaller, None);
        assert_eq!(smaller.file().chapters[view.chapter].id, "gamma");
    }

    #[test]
    fn the_canvas_menu_opens_tables_and_settings() {
        let settings = r#"{ }"#;
        let chapter = r#"{ "id": "c", "title": "C", "quests": [ { "id": "q", "title": "Q", "rewards": [ { "id": "r", "type": "random", "table": "loot" } ] } ] }"#;
        let table = r#"{ "id": "loot", "entries": [ { "type": "xp", "points": 5, "weight": 2 } ] }"#;
        let loaded = super::super::hq_validate::load_pack(Some(settings), &[("c.json", chapter)], &[("loot.json", table)]);
        assert_eq!(loaded.problems, []);
        let (book, progress) = (Book::new(loaded.file), Progress::default());
        let mut view = opened(&book, &progress);
        (view.editor, view.editing) = (true, true);
        view.menu = Some(Menu { at: [10.0, 10.0], title: None, entries: vec![("Reward tables...".into(), Action::Tables)], icons: Vec::new() });
        click(&mut view, &book, &progress, &Action::Tables);
        assert_eq!(view.menu.as_ref().unwrap().entries[0].1, Action::OpenTable("loot".into()));
        click(&mut view, &book, &progress, &Action::OpenTable("loot".into()));
        let labels: Vec<String> = view.menu.as_ref().unwrap().entries.iter().map(|(l, _)| l.clone()).collect();
        assert_eq!(labels[1..], ["+ Add an entry".to_owned(), "Delete this table".to_owned()]);
        assert!(labels[0].ends_with("(weight 2)"), "{labels:?}");
        click(&mut view, &book, &progress, &Action::OpenPanel(Subject::Entry("loot".into(), 0)));
        let reaction = click(&mut view, &book, &progress, &Action::Panel(PanelAction::Step("weight", 1.0)));
        assert!(matches!(&reaction, Reaction::Edit(Edit::SaveEntry { index: Some(0), entry, .. }) if entry.weight == 3), "{reaction:?}");
        view.panel = None;
        view.menu = Some(Menu { at: [10.0, 10.0], title: None, entries: vec![("Pack settings...".into(), Action::OpenPanel(Subject::Settings))], icons: Vec::new() });
        click(&mut view, &book, &progress, &Action::OpenPanel(Subject::Settings));
        assert_eq!(view.panel, Some(Subject::Settings));
    }

    #[test]
    fn regions_split_the_window() {
        let closed = regions(RECT);
        assert_eq!(closed.footer, [0.0, 270.0 - canvas::FOOTER, 480.0, canvas::FOOTER]);
        assert_eq!(closed.strip[2] + closed.canvas[2], 480.0);
        // The open chapter list goes over the canvas's left side.
        assert_eq!(closed.sidebar[0], closed.strip[0]);
        assert!(closed.sidebar[2] > closed.strip[2] && closed.sidebar[2] < closed.canvas[2]);
        // No band at the top: the canvas starts at the window's top.
        assert_eq!(closed.canvas[1], 0.0);
        assert_eq!(closed.canvas[3], 270.0 - canvas::FOOTER);

        // A window too small for anything: nothing goes negative.
        for rect in [[0.0, 0.0, 40.0, 20.0], [0.0, 0.0, 0.0, 0.0]] {
            let tiny = regions(rect);
            for part in [tiny.sidebar, tiny.strip, tiny.canvas, tiny.footer] {
                assert!(part[2] >= 0.0 && part[3] >= 0.0, "{part:?}");
            }
        }
    }

    #[test]
    fn shapes_are_fans_of_quads_inside_their_radius() {
        for (shape, quads) in
            [(Shape::Square, 1), (Shape::Diamond, 1), (Shape::Hexagon, 2), (Shape::Circle, 7)]
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
        let main = || Row::Group { id: "main".into(), title: "Main story".into() };
        assert_eq!(sidebar_rows(&book(), &[]), [Row::Chapter(0), main(), Row::Chapter(1)]);
        // A folded group keeps its heading, without its chapters.
        assert_eq!(sidebar_rows(&book(), &["main".into()]), [Row::Chapter(0), main()]);
    }

    #[test]
    fn a_click_on_a_group_heading_folds_and_unfolds_it() {
        let (book, progress) = (book(), Progress::default());
        let mut view = opened(&book, &progress);
        view.sidebar_open = true;
        let group = Action::Group("main".into());
        assert_eq!(click(&mut view, &book, &progress, &group), Reaction::Nothing);
        assert_eq!(view.folded, ["main"]);
        let scene = view.scene(&screen(&book, &progress, NOWHERE));
        assert!(zone(&scene, &group).is_some());
        assert_eq!(zone(&scene, &Action::Chapter(1)), None);
        click(&mut view, &book, &progress, &group);
        assert!(view.folded.is_empty());
        assert!(zone(&view.scene(&screen(&book, &progress, NOWHERE)), &Action::Chapter(1)).is_some());
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
        // The chapter list is closed: only its strip shows.
        assert_eq!(zone(&scene, &Action::Chapter(0)), None);
        assert!(zone(&scene, &Action::Close).is_some());
        assert_eq!(zone(&scene, &Action::ClaimAll), None);
        let shown = texts(&scene);
        // The chapter's name and tally in the bottom band, no subtitle.
        assert!(!shown.contains(&"The first one"));
        for text in ["Alpha", "0/3"] {
            assert!(shown.contains(&text), "{text} in {shown:?}");
        }

        // Open, it lists the chapters under their group.
        let mut view = view;
        view.sidebar_open = true;
        let scene = view.scene(&screen(&book, &progress, NOWHERE));
        assert!(zone(&scene, &Action::Chapter(0)).is_some() && zone(&scene, &Action::Chapter(1)).is_some());
        assert_eq!(zone(&scene, &Action::Chapter(2)), None);
        let shown = texts(&scene);
        for text in ["Main story", "Beta"] {
            assert!(shown.contains(&text), "{text} in {shown:?}");
        }
        // A chapter's name is grey, blue once started, green once all its
        // quests are done.
        let ink = |scene: &Scene, name: &str| {
            scene.draws.iter().find_map(|d| match d {
                Draw::Text { text, colour, .. } if text == name => Some(*colour),
                _ => None,
            })
        };
        assert_eq!(ink(&scene, "Beta"), Some(CHAPTER_INK));
        // Beta's only quest has no task: once the engine has run, it is done.
        let done = progress_after(&book, &[QuestEvent::Refresh]);
        assert_eq!(ink(&view.scene(&screen(&book, &done, NOWHERE)), "Beta"), Some(CHAPTER_DONE));
        use QuestState::*;
        assert_eq!(chapter_ink(false, &[Available, Locked]), CHAPTER_INK);
        assert_eq!(chapter_ink(false, &[InProgress, Locked]), CHAPTER_STARTED);
        assert_eq!(chapter_ink(false, &[Done, Available]), CHAPTER_STARTED);
        assert_eq!(chapter_ink(false, &[RewardsWaiting, Locked]), CHAPTER_STARTED);
        assert_eq!(chapter_ink(true, &[Done, RewardsWaiting]), CHAPTER_DONE);
        // One icon per visible quest, and the chapter's in the list.
        let icons = scene.draws.iter().filter(|draw| matches!(draw, Draw::Item { .. })).count();
        assert_eq!(icons, 3);
        // One line, start to next, which cannot be started yet: dimmed, its
        // chevrons all pointing the same way.
        let chevrons: Vec<[f32; 4]> = scene
            .draws
            .iter()
            .filter_map(|draw| match draw {
                Draw::Sprite { name, src, .. } if name == CHEVRON_SPRITE && *src != bead_cell(LINE_LOCKED) => Some(*src),
                _ => None,
            })
            .collect();
        assert!(!chevrons.is_empty() && chevrons.iter().all(|src| *src == chevron_cell(LINE_LOCKED, [1.0, 0.0])));
    }

    #[test]
    fn hovering_a_quest_shows_its_title_in_a_framed_box() {
        let (book, progress) = (book(), Progress::default());
        let view = opened(&book, &progress);
        let at = zone(&view.scene(&screen(&book, &progress, NOWHERE)), &Action::Quest(1)).unwrap();
        let scene = view.scene(&screen(&book, &progress, at));
        assert_eq!(texts(&scene).last(), Some(&"Then this"));
        assert!(scene.draws.iter().any(|d| matches!(d, Draw::Fill { colour, .. } if *colour == TOOLTIP_EDGE_TOP)));
        assert!(!texts(&view.scene(&screen(&book, &progress, NOWHERE))).contains(&"Then this"));
    }

    #[test]
    fn the_quest_under_the_pointer_grows_and_the_selected_one_gets_corners() {
        let (book, progress) = (book(), Progress::default());
        let mut view = opened(&book, &progress);
        let frame = |scene: &Scene, n: usize| -> [f32; 4] {
            let cell = node_cell(book.nodes()[n].shape, scene_state(&book, &progress, n));
            scene
                .draws
                .iter()
                .find_map(|d| match d {
                    Draw::Sprite { src, dst, .. } if *src == cell => Some(*dst),
                    _ => None,
                })
                .unwrap()
        };
        let rest = view.scene(&screen(&book, &progress, NOWHERE));
        let at = zone(&rest, &Action::Quest(0)).unwrap();
        let over = view.scene(&screen(&book, &progress, at));
        assert!(frame(&over, 0)[2] > frame(&rest, 0)[2], "drawn bigger under the pointer");
        // The click zone does not grow with it.
        assert_eq!(zone(&over, &Action::Quest(0)), Some(at));

        // Selected, it stays raised with the pointer away.
        click(&mut view, &book, &progress, &Action::Quest(0));
        assert_eq!(frame(&view.scene(&screen(&book, &progress, NOWHERE)), 0)[2], frame(&over, 0)[2]);
    }

    /// How many signs of the sheet's `cell` a scene draws.
    fn signs(scene: &Scene, cell: MarkCell) -> usize {
        let src = cell as u8 as f32 * MARK_CELL;
        scene.draws.iter().filter(|d| matches!(d, Draw::Sprite { name, src: s, .. } if name == MARK_SPRITE && s[0] == src)).count()
    }

    fn scene_state(book: &Book, progress: &Progress, n: usize) -> QuestState {
        book.statuses(progress, "steve")[n].state
    }

    #[test]
    fn selecting_a_quest_opens_its_window_over_the_canvas() {
        let (book, progress) = (book(), Progress::default());
        let mut view = opened(&book, &progress);
        let before = zone(&view.scene(&screen(&book, &progress, NOWHERE)), &Action::Quest(0)).unwrap();
        assert_eq!(click(&mut view, &book, &progress, &Action::Quest(0)), Reaction::Nothing);
        assert_eq!(view.selected, Some(0));

        let scene = view.scene(&screen(&book, &progress, NOWHERE));
        let popup = scene.popup.unwrap();
        // Centred over the canvas, which does not move.
        let canvas = scene.regions.canvas;
        assert!(contains(canvas, popup));
        assert!((canvas::centre(popup)[0] - canvas::centre(canvas)[0]).abs() < 0.01);
        let shown = texts(&scene);
        for text in ["Start here", "Tasks", "Rewards", "Available", "A beginning", "x"] {
            assert!(shown.contains(&text), "{text} in {shown:?}");
        }
        // The description is wrapped to the window.
        for draw in &scene.draws {
            if let Draw::Text { text, at, .. } = draw
                && canvas::inside(popup, *at)
            {
                assert!(at[0] + measure(text) <= popup[0] + popup[2], "{text}");
            }
        }
        assert!(canvas::inside(popup, zone(&scene, &Action::Check("a/start/ok".into())).unwrap()));
        // Not completed: nothing to claim yet.
        assert_eq!(zone(&scene, &Action::Claim("a/start/xp".into())), None);
        // An empty spot of the canvas closes it; so does its x.
        assert_eq!(scene.action_at([canvas[0] + 1.0, canvas[1] + 1.0]), Some(&Action::Deselect));
        assert_eq!(scene.action_at(canvas::centre(popup)), Some(&Action::Stay));
        assert_eq!(click(&mut view, &book, &progress, &Action::Deselect), Reaction::Nothing);
        assert_eq!(view.selected, None);
        let back = zone(&view.scene(&screen(&book, &progress, NOWHERE)), &Action::Quest(0)).unwrap();
        assert!((back[0] - before[0]).abs() < 0.01);
    }

    #[test]
    fn the_window_shows_a_task_and_a_reward_as_icons() {
        let (book, progress) = (book(), Progress::default());
        let mut view = opened(&book, &progress);
        click(&mut view, &book, &progress, &Action::Quest(0));
        let scene = view.scene(&screen(&book, &progress, NOWHERE));
        let icons: Vec<&str> = scene
            .draws
            .iter()
            .filter_map(|d| match d {
                Draw::Item { item, size, .. } if *size == 16.0 => Some(item.as_str()),
                _ => None,
            })
            .collect();
        // A checkmark task (a drawn button, no item), and 5 XP with its count.
        assert_eq!(icons, ["minecraft:experience_bottle"]);
        assert_eq!(signs(&scene, MarkCell::Button), 1);
        assert!(scene.draws.iter().any(|d| matches!(d, Draw::Sprite { name, .. } if name == DIGIT_SPRITE)));
        // The title bar has the quest's icon.
        assert!(scene.draws.iter().any(|d| matches!(d, Draw::Item { size, .. } if *size == 12.0)));
    }

    #[test]
    fn icons_send_requests_and_say_what_a_click_does() {
        let book = book();
        let mut progress = Progress::default();
        let mut view = opened(&book, &progress);
        click(&mut view, &book, &progress, &Action::Quest(0));
        let check = Action::Check("a/start/ok".into());
        assert_eq!(tip(&view, &book, &progress, &check).unwrap(), ["Confirm", "Click to tick it"]);
        assert_eq!(
            click(&mut view, &book, &progress, &check),
            Reaction::Send(ToServer::Check { task: "a/start/ok".into() })
        );

        // The server did it: the quest is completed, its reward waits.
        progress = progress_after(&book, &[QuestEvent::Check { task: "a/start/ok".into() }]);
        let scene = view.scene(&screen(&book, &progress, NOWHERE));
        assert_eq!(zone(&scene, &check), None);
        let shown = texts(&scene);
        for text in ["Rewards waiting", "Claim all (1)", "1/3"] {
            assert!(shown.contains(&text), "{text} in {shown:?}");
        }
        let claim = Action::Claim("a/start/xp".into());
        assert_eq!(tip(&view, &book, &progress, &claim).unwrap(), ["5 XP", "Click to claim"]);
        // A reward waiting has its "!", and so has the quest's node, in the
        // done green.
        let waiting = view.scene(&screen(&book, &progress, NOWHERE));
        assert_eq!(signs(&waiting, MarkCell::Alert), 2);
        assert_eq!(state_look(QuestState::RewardsWaiting).0, state_look(QuestState::Done).0);
        assert_eq!(click(&mut view, &book, &progress, &claim), Reaction::Send(ToServer::Claim { reward: "a/start/xp".into(), choice: None }));
        assert_eq!(click(&mut view, &book, &progress, &Action::ClaimAll), Reaction::Send(ToServer::ClaimAll));

        // Claimed: the reward says so and takes no more claims.
        progress = progress_after(&book, &[QuestEvent::Check { task: "a/start/ok".into() }, QuestEvent::ClaimAll]);
        let scene = view.scene(&screen(&book, &progress, NOWHERE));
        assert_eq!(zone(&scene, &claim), None);
        assert_eq!(tip(&view, &book, &progress, &Action::Stay).unwrap(), ["5 XP", "Claimed"]);
        // Claimed: a check on it, and on the done task.
        // (The checkmark task's own button turns green.)
        let scene_now = view.scene(&screen(&book, &progress, NOWHERE));
        assert_eq!((signs(&scene_now, MarkCell::Check), signs(&scene_now, MarkCell::ButtonDone)), (1, 1));
        // No more "!" anywhere, the node's badge gone too.
        assert_eq!(signs(&scene_now, MarkCell::Alert), 0);
        assert_eq!(zone(&scene, &Action::ClaimAll), None);

        // The next quest is open now, and takes a hand-in.
        click(&mut view, &book, &progress, &Action::Deselect);
        click(&mut view, &book, &progress, &Action::Quest(1));
        let submit = Action::Submit("a/next/planks".into());
        assert_eq!(
            tip(&view, &book, &progress, &submit).unwrap(),
            ["Oak planks", "Minecraft", "Hand in 4x Oak planks", "Progress: 0/4", "Click to hand in"]
        );
        // The count is what the task asks, as in an inventory.
        let scene = view.scene(&screen(&book, &progress, NOWHERE));
        assert!(scene.draws.iter().any(|d| matches!(d, Draw::Sprite { name, .. } if name == DIGIT_SPRITE)));
        // With the game's names, the item is named as in the game.
        view.item_names.insert("minecraft:oak_planks".into(), "Oak Planks".into());
        assert_eq!(tip(&view, &book, &progress, &submit).unwrap()[0], "Oak Planks");
        assert_eq!(click(&mut view, &book, &progress, &submit), Reaction::Send(ToServer::Submit { task: "a/next/planks".into() }));
        assert_eq!(click(&mut view, &book, &progress, &Action::Close), Reaction::Close);
    }

    /// The tooltip's lines with the pointer on the window's zone that does
    /// `action`: the texts drawn after those of the scene without it.
    fn tip(view: &BookView, book: &Book, progress: &Progress, action: &Action) -> Option<Vec<String>> {
        let scene = view.scene(&screen(book, progress, NOWHERE));
        let popup = scene.popup?;
        let at = scene.zones.iter().rev().find(|(rect, a)| a == action && contains(popup, *rect)).map(|(rect, _)| canvas::centre(*rect))?;
        let before = texts(&scene).len();
        Some(texts(&view.scene(&screen(book, progress, at)))[before..].iter().map(|t| (*t).to_owned()).collect())
    }

    #[test]
    fn nodes_are_drawn_from_the_frame_sheet_with_polygons_to_fall_back_on() {
        let (book, progress) = (book(), Progress::default());
        let view = opened(&book, &progress);
        let scene = view.scene(&screen(&book, &progress, NOWHERE));
        let frames: Vec<(&[f32; 4], &Vec<Draw>)> = scene
            .draws
            .iter()
            .filter_map(|d| match d {
                Draw::Sprite { name, src, fallback, .. } if name == NODE_SPRITE => Some((src, fallback)),
                _ => None,
            })
            .collect();
        assert_eq!(frames.len(), 2, "one per node shown");
        // The open quest is "available", the other "locked", each in its shape's row.
        let shape = book.nodes()[0].shape;
        assert_eq!(*frames[0].0, node_cell(shape, QuestState::Available));
        assert_eq!(node_cell(Shape::Hexagon, QuestState::Done), [192.0, 192.0, 64.0, 64.0]);
        assert_eq!(node_cell(Shape::Hexagon, QuestState::RewardsWaiting), node_cell(Shape::Hexagon, QuestState::Done));
        assert!(frames.iter().all(|(_, fallback)| fallback.iter().all(|d| matches!(d, Draw::Quad { .. })) && !fallback.is_empty()));
    }

    #[test]
    fn lines_carry_chevrons_and_beads_and_light_up_round_the_quest_under_the_pointer() {
        let (book, progress) = (book(), Progress::default());
        let view = opened(&book, &progress);
        // The marks of the lines: (cell, place) of each line sprite.
        let marks = |view: &BookView, progress: &Progress, mouse: [f32; 2]| -> Vec<([f32; 4], [f32; 4])> {
            view.scene(&screen(&book, progress, mouse))
                .draws
                .iter()
                .filter_map(|d| match d {
                    Draw::Sprite { name, src, dst, .. } if name == CHEVRON_SPRITE => Some((*src, *dst)),
                    _ => None,
                })
                .collect()
        };
        let only = |marks: &[([f32; 4], [f32; 4])], colour: u32| {
            !marks.is_empty() && marks.iter().all(|(src, _)| *src == chevron_cell(colour, [1.0, 0.0]) || *src == bead_cell(colour))
        };
        // At rest the line into the next quest, which cannot be started
        // yet, is dimmed.
        let resting = marks(&view, &progress, NOWHERE);
        assert!(only(&resting, LINE_LOCKED));
        assert!(resting.iter().any(|(src, _)| *src == bead_cell(LINE_LOCKED)));
        // Over the first quest: the line to the quest it opens is yellow.
        let first = zone(&view.scene(&screen(&book, &progress, NOWHERE)), &Action::Quest(0)).unwrap();
        assert!(only(&marks(&view, &progress, first), LINE_OPENS));
        // Over the second: the line from what it needs is cyan.
        let second = zone(&view.scene(&screen(&book, &progress, NOWHERE)), &Action::Quest(1)).unwrap();
        assert!(only(&marks(&view, &progress, second), LINE_NEEDS));
        // Grey once the next quest can be started; green while nothing is in
        // focus only once both its quests are done.
        let start = QuestEvent::Check { task: "a/start/ok".into() };
        assert!(only(&marks(&view, &progress_after(&book, std::slice::from_ref(&start)), NOWHERE), LINE));
        let planks = QuestEvent::Submitted { task: "a/next/planks".into(), count: 4 };
        // (Once done, a quest further on shows, its line grey.)
        let done = marks(&view, &progress_after(&book, &[start, planks]), NOWHERE);
        assert!(done.iter().any(|(src, _)| *src == chevron_cell(LINE_DONE, [1.0, 0.0])));
        // The marks of the lines in focus move with time, the others stay.
        let mut later = view.clone();
        later.clock += 3;
        later.partial_tick = 0.5;
        assert_eq!(resting, marks(&later, &progress, NOWHERE));
        assert_ne!(marks(&view, &progress, first), marks(&later, &progress, first));
        // The nearest of the drawn directions, in its colour's block; the
        // bead under the chevrons.
        assert_eq!(chevron_cell(LINE, [1.0, 0.0]), [0.0, 0.0, 32.0, 32.0]);
        assert_eq!(chevron_cell(LINE, [0.0, 1.0]), [256.0, 0.0, 32.0, 32.0]);
        assert_eq!(chevron_cell(LINE_OPENS, [-1.0, 0.0]), [0.0, 7.0 * 32.0, 32.0, 32.0]);
        assert_eq!(chevron_cell(LINE_DONE, [0.7071, -0.7071]), [12.0 * 32.0, 3.0 * 32.0, 32.0, 32.0]);
        assert_eq!(bead_cell(LINE), [0.0, 320.0, 32.0, 32.0]);
        assert_eq!(bead_cell(LINE_OPENS), [96.0, 320.0, 32.0, 32.0]);
        assert_eq!(chevron_cell(LINE_LOCKED, [1.0, 0.0]), [0.0, 256.0, 32.0, 32.0]);
    }

    #[test]
    fn chevrons_and_beads_take_turns_along_a_line() {
        let marks = line_marks(40.0, 10.0, 0.0);
        assert_eq!(marks, [(5.0, true), (10.0, false), (15.0, true), (20.0, false), (25.0, true), (30.0, false), (35.0, true)]);
        // Half a step on: everything moved 5 towards the end, and a bead
        // comes in at the start.
        let moved = line_marks(40.0, 10.0, 0.5);
        assert_eq!(moved[..3], [(5.0, false), (10.0, true), (15.0, false)]);
        assert!(moved.iter().all(|&(at, _)| at > 0.0 && at < 40.0));
        assert!(line_marks(40.0, 0.0, 0.0).is_empty());
    }

    #[test]
    fn a_quest_is_pinned_and_unpinned_from_its_window() {
        let (book, progress) = (book(), Progress::default());
        let mut view = opened(&book, &progress);
        click(&mut view, &book, &progress, &Action::Quest(0));
        let pinned = |view: &BookView| {
            view.scene(&screen(&book, &progress, NOWHERE)).draws.iter().any(|d| matches!(d, Draw::Fill { colour, .. } if *colour == 0xFFFF_C040))
        };
        assert!(!pinned(&view));
        let track = Action::Track("a/start".into());
        assert_eq!(tip(&view, &book, &progress, &track).unwrap(), ["Pin: show it while you play"]);
        assert_eq!(click(&mut view, &book, &progress, &track), Reaction::Tracked(Some("a/start".into())));
        // The pin turns gold, and the window has no Track button text.
        assert!(pinned(&view));
        assert!(!texts(&view.scene(&screen(&book, &progress, NOWHERE))).iter().any(|t| t.contains("Track")));
        assert_eq!(click(&mut view, &book, &progress, &Action::Untrack), Reaction::Tracked(None));
        assert_eq!(view.tracked, None);
    }

    #[test]
    fn windows_over_the_book_sit_on_the_quest_window_and_escape_closes_it_first() {
        let book = gamma();
        let progress = Progress::default();
        let mut view = opened(&book, &progress);
        click(&mut view, &book, &progress, &Action::Quest(0));
        click(&mut view, &book, &progress, &Action::OpenChoice("g/bake/pick".into()));
        let scene = view.scene(&screen(&book, &progress, NOWHERE));
        let popup = scene.popup.unwrap();
        // The choice window (the zone right after its full-screen "close")
        // is centred on the quest window, not on the whole book.
        let at = scene.zones.iter().position(|(r, a)| *a == Action::CloseChoice && *r == RECT).unwrap();
        let window = scene.zones[at + 1].0;
        assert!((canvas::centre(window)[0] - canvas::centre(popup)[0]).abs() < 0.5, "{window:?} on {popup:?}");
        assert!((canvas::centre(window)[1] - canvas::centre(popup)[1]).abs() < 0.5, "{window:?} on {popup:?}");
        // Escape: the quest window goes, then nothing more to close (the book's turn).
        view.choosing = None;
        assert!(view.close_quest());
        assert_eq!(view.selected, None);
        assert!(!view.close_quest());
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
    fn an_item_icon_opens_its_recipes_and_the_view_turns_and_closes() {
        // The hand-in quest is locked: a click on its item shows the recipes.
        let (book, progress) = (book(), Progress::default());
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
        // The quest window's task icon, then the recipe's slot and result.
        assert_eq!(items(&scene), ["minecraft:oak_planks", "minecraft:oak_log", "minecraft:oak_planks"]);
        // A slot with two items shows the second one a second later.
        view.recipe.as_mut().unwrap().clock = 20;
        assert_eq!(items(&view.scene(&screen(&book, &progress, NOWHERE)))[1], "minecraft:birch_log");
        // Turning goes round; the view keeps clicks on it, and closes on one outside.
        click(&mut view, &book, &progress, &Action::TurnRecipe(1));
        assert_eq!(view.recipe.as_ref().unwrap().page, 1);
        click(&mut view, &book, &progress, &Action::TurnRecipe(1));
        assert_eq!(view.recipe.as_ref().unwrap().page, 0);
        let scene = view.scene(&screen(&book, &progress, NOWHERE));
        // Under the view, the book's own zones do not take clicks.
        assert_eq!(scene.action_at(canvas::centre(RECT)), Some(&Action::Stay));
        assert_eq!(scene.action_at([RECT[0] + 1.0, RECT[1] + 1.0]), Some(&Action::CloseRecipe));
        click(&mut view, &book, &progress, &Action::CloseRecipe);
        assert!(view.recipe.is_none());

        // Nothing makes it: the view says so.
        view.recipe = Some(RecipeOverlay { title: "Bedrock".into(), ..RecipeOverlay::default() });
        assert!(texts(&view.scene(&screen(&book, &progress, NOWHERE))).contains(&"No recipe makes this item."));
    }

    #[test]
    fn hovering_an_icon_keeps_it_on_top() {
        let (book, progress) = (book(), Progress::default());
        let mut view = opened(&book, &progress);
        click(&mut view, &book, &progress, &Action::Quest(1));
        let show = Action::ShowRecipe("minecraft:oak_planks".into());
        let over = zone(&view.scene(&screen(&book, &progress, NOWHERE)), &show).unwrap();
        let scene = view.scene(&screen(&book, &progress, over));
        let shade = scene.draws.iter().position(|d| matches!(d, Draw::Fill { colour, .. } if *colour == ROW_HOVER)).unwrap();
        let icon = scene
            .draws
            .iter()
            .position(|d| matches!(d, Draw::Item { item, size, .. } if item == "minecraft:oak_planks" && *size == 16.0))
            .unwrap();
        assert!(shade < icon, "the hover shade ({shade}) is drawn before the icon ({icon})");
        // The waiting task is veiled.
        assert!(scene.draws.iter().any(|d| matches!(d, Draw::Fill { colour, .. } if *colour == DIM_VEIL)));
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
    fn counts_are_small_digits_in_the_corner() {
        assert!(count_draws("", [0.0, 0.0]).is_empty());
        assert_eq!(count_cells("3-5"), [3, 10, 5]);
        for count in ["3", "64", "1000", "3-5"] {
            let draws = count_draws(count, [100.0, 50.0]);
            assert_eq!(draws.len(), count.len(), "one sprite a digit");
            for (i, d) in draws.iter().enumerate() {
                let Draw::Sprite { name, src, dst, fallback } = d else { panic!("digits are sprites") };
                assert_eq!(name, DIGIT_SPRITE);
                let digit = count_cells(count)[i];
                assert_eq!(src[0], digit as f32 * DIGIT_CELL[0]);
                // In the icon's bottom-right part, never much past it.
                assert!(dst[1] >= 61.0 && dst[1] + dst[3] <= 67.0, "{count}: {dst:?}");
                assert!(dst[0] + dst[2] <= 117.5, "{count}: {dst:?}");
                // The pixel-art count is the first digit's fallback, once.
                assert_eq!(fallback.is_empty(), i > 0);
            }
        }
        // The pixel-art count: in the corner too, the edge at most half a pixel past.
        for d in count_pixel_draws("64", [100.0, 50.0]) {
            let Draw::Fill { rect, .. } = d else { panic!("digits are fills") };
            assert!(rect[1] >= 61.0 && rect[1] + rect[3] <= 66.5 && rect[0] + rect[2] <= 116.5, "{rect:?}");
        }
        let left = count_pixel_draws("64", [0.0, 0.0]).iter().map(|d| match d { Draw::Fill { rect, .. } => rect[0], _ => 99.0 }).fold(99.0, f32::min);
        assert_eq!(left, 16.5 - 13.0 * FINE);
    }

    #[test]
    fn a_checkmark_task_is_a_round_button() {
        for (done, cell) in [(false, MarkCell::Button), (true, MarkCell::ButtonDone)] {
            let draws = check_task_draws([0.0, 0.0], done);
            let [Draw::Sprite { name, src, dst, fallback }] = &draws[..] else { panic!("one sprite: {draws:?}") };
            assert_eq!((name.as_str(), src[0], *dst), (MARK_SPRITE, cell as u8 as f32 * MARK_CELL, [0.0, 0.0, 16.0, 16.0]));
            // Without the sheet: the button in half pixels, its check white
            // and, once done, its body green.
            let check = fallback.iter().filter(|d| matches!(d, Draw::Fill { colour: CHECK_WHITE, .. })).count();
            assert!(check >= 10, "a check: {check} runs");
            let green = fallback.iter().any(|d| matches!(d, Draw::Fill { colour: 0xFF48_C03C, .. }));
            assert_eq!(green, done);
            for d in fallback {
                let Draw::Fill { rect, .. } = d else { panic!("the button is fills") };
                assert!(contains([0.0, 0.0, 16.0, 16.0], *rect), "{rect:?}");
            }
        }
    }

    #[test]
    fn signs_are_sprites_with_pixel_art_behind() {
        for mark in [Mark::Check, Mark::Question, Mark::Alert] {
            let draws = mark_draws(mark, [100.0, 50.0]);
            let [Draw::Sprite { name, dst, fallback, .. }] = &draws[..] else { panic!("one sprite: {draws:?}") };
            assert_eq!(name, MARK_SPRITE);
            // On the icon's top-right corner, a little outside.
            assert!(dst[0] >= 108.0 && dst[0] + dst[2] <= 120.0 && dst[1] >= 46.0 && dst[1] <= 48.0, "{mark:?}: {dst:?}");
            assert!(!fallback.is_empty());
        }
    }

    #[test]
    fn random_and_choice_rewards_show_their_entries_in_turn() {
        let table: RewardTable = serde_json::from_str(
            r#"{ "id": "t", "entries": [
                { "weight": 2, "type": "item", "item": "minecraft:iron_ingot", "count": 4 },
                { "type": "item", "item": "minecraft:diamond" },
                { "type": "item", "item": "minecraft:emerald", "count": 3, "max_count": 5 } ] }"#,
        )
        .unwrap();
        assert_eq!(table_icon(Some(&table), 0), ("minecraft:iron_ingot".to_owned(), "4".to_owned()));
        assert_eq!(table_icon(Some(&table), TABLE_TURN - 1).0, "minecraft:iron_ingot");
        assert_eq!(table_icon(Some(&table), TABLE_TURN), ("minecraft:diamond".to_owned(), String::new()));
        assert_eq!(table_icon(Some(&table), 2 * TABLE_TURN), ("minecraft:emerald".to_owned(), "3-5".to_owned()));
        assert_eq!(table_icon(Some(&table), 3 * TABLE_TURN).0, "minecraft:iron_ingot");
        assert_eq!(table_icon(None, 0).0, "minecraft:chest");
        let lines: Vec<String> = table_lines(&table, true).into_iter().map(|(line, _)| line).collect();
        assert_eq!(lines, ["- 4x Iron ingot [50%]", "- Diamond [25%]", "- 3-5x Emerald [25%]"]);
        let lines: Vec<String> = table_lines(&table, false).into_iter().map(|(line, _)| line).collect();
        assert_eq!(lines, ["- 4x Iron ingot", "- Diamond", "- 3-5x Emerald"]);
        assert_eq!(chance(1, 3), "33.33%");
        assert_eq!(chance(1, 8), "12.5%");
    }

    #[test]
    fn a_locked_node_is_greyed_and_padlocked() {
        let has_veil = |scene: &Scene| scene.draws.iter().any(|d| matches!(d, Draw::Quad { colour: LOCKED_VEIL, .. }));
        // The veil is the face's shape, not a square over the icon.
        assert!((face_share(Shape::Square) - 0.891).abs() < 0.01 && face_share(Shape::Diamond) < face_share(Shape::Square));
        let lock_src = MarkCell::Lock as u8 as f32 * MARK_CELL;
        let has_lock = |scene: &Scene| scene.draws.iter().any(|d| matches!(d, Draw::Sprite { name, src, .. } if name == MARK_SPRITE && src[0] == lock_src));
        let (book, progress) = (book(), Progress::default());
        let view = opened(&book, &progress);
        let scene = view.scene(&screen(&book, &progress, NOWHERE));
        let locked = book.statuses(&progress, "steve").iter().filter(|s| s.state == QuestState::Locked).count();
        assert!(locked > 0 && has_veil(&scene) && has_lock(&scene), "{locked} quests locked at first");
        // The padlock grows with the node and stays inside it, smooth or in pixels.
        for size in [16.0, 40.0, 120.0] {
            let node = [10.0, 20.0, size, size];
            let Draw::Sprite { dst, fallback, .. } = lock_draws(node) else { panic!("a lock is a sprite") };
            assert!(contains(node, dst), "{dst:?} inside {node:?}");
            for d in fallback {
                let Draw::Fill { rect, .. } = d else { panic!("a pixel lock is fills") };
                assert!(contains(node, rect), "{rect:?} inside {node:?}");
            }
        }
    }

    #[test]
    fn turning_to_another_chapter_closes_the_quest_window() {
        let (book, progress) = (book(), Progress::default());
        let mut view = opened(&book, &progress);
        click(&mut view, &book, &progress, &Action::Quest(0));
        view.sidebar_open = true;
        assert_eq!(click(&mut view, &book, &progress, &Action::Chapter(1)), Reaction::Nothing);
        assert_eq!((view.chapter, view.selected), (1, None));
        let scene = view.scene(&screen(&book, &progress, NOWHERE));
        assert_eq!(scene.popup, None);
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

        // A short window: the quest window's text does not all fit, and
        // scrolls as far as its last line; the canvas does not zoom under it.
        let short = |mouse| Screen { rect: [0.0, 0.0, 480.0, 120.0], ..screen(&book, &progress, mouse) };
        view.show_chapter(&short(NOWHERE), 0);
        let at = zone(&view.scene(&short(NOWHERE)), &Action::Quest(0)).unwrap();
        view.click(&short(at));
        let scene = view.scene(&short(NOWHERE));
        assert!(scene.popup_max_scroll > 0.0);
        let over_popup = canvas::centre(scene.popup.unwrap());
        let zoom = view.view.zoom;
        view.scroll(&short(over_popup), -100.0);
        assert_eq!(view.popup_scroll, scene.popup_max_scroll);
        assert_eq!(view.view.zoom, zoom);
        view.scroll(&short(over_popup), 100.0);
        assert_eq!(view.popup_scroll, 0.0);
    }

    #[test]
    fn dragging_pans_only_over_the_canvas() {
        let (book, progress) = (book(), Progress::default());
        let mut view = opened(&book, &progress);
        let regions = view.scene(&screen(&book, &progress, NOWHERE)).regions;
        let pan = view.view.pan;
        view.drag(&screen(&book, &progress, canvas::centre(regions.strip)), [10.0, 10.0]);
        assert_eq!(view.view.pan, pan);
        // Nor over the open chapter list.
        view.sidebar_open = true;
        view.drag(&screen(&book, &progress, canvas::centre(regions.sidebar)), [10.0, 10.0]);
        assert_eq!(view.view.pan, pan);
        view.drag(&screen(&book, &progress, canvas::centre(regions.canvas)), [10.0, 0.0]);
        assert!(view.view.pan[0] < pan[0]);
    }

    #[test]
    fn the_chapter_list_opens_under_the_pointer_without_a_click() {
        let (book, progress) = (book(), Progress::default());
        let mut view = opened(&book, &progress);
        let regions = regions(RECT);
        let at = |view: &mut BookView, mouse: [f32; 2]| {
            view.hover(&screen(&book, &progress, mouse));
            view.sidebar_open
        };
        // Closed at first; the canvas where the list would be does not open it.
        assert!(!view.sidebar_open);
        assert!(!at(&mut view, [regions.sidebar[0] + regions.sidebar[2] - 2.0, regions.sidebar[1] + 20.0]));
        // The strip opens it; it stays open over the list, closes off it.
        let strip = canvas::centre(regions.strip);
        assert!(at(&mut view, strip));
        assert!(at(&mut view, [regions.sidebar[0] + regions.sidebar[2] - 2.0, strip[1]]));
        assert!(!at(&mut view, canvas::centre(regions.canvas)));
        // Open, it hides the quest under it from the pointer.
        view.sidebar_open = true;
        let scene = view.scene(&screen(&book, &progress, strip));
        assert_eq!(scene.action_at(strip), Some(&Action::Stay));
    }

    #[test]
    fn nodes_out_of_the_canvas_are_not_drawn() {
        let (book, progress) = (book(), Progress::default());
        let mut view = opened(&book, &progress);
        view.view.pan = [10_000.0, 0.0];
        let scene = view.scene(&screen(&book, &progress, NOWHERE));
        assert_eq!(zone(&scene, &Action::Quest(0)), None);
        // No icon left (the chapter list is closed).
        assert_eq!(scene.draws.iter().filter(|draw| matches!(draw, Draw::Item { .. })).count(), 0);
        // No quad but the list strip's arrow.
        assert!(!scene.draws.iter().any(|draw| matches!(draw, Draw::Quad { colour, .. } if *colour != LIST_ARROW)));
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
        // Half the node takes clicks; the list's strip, drawn after, covers the rest.
        assert!((rect[0] - area[0]).abs() < 0.01 && (rect[2] - canvas::NODE / 2.0).abs() < 0.01, "{rect:?}");
        let icons = scene.draws.iter().filter(|draw| matches!(draw, Draw::Item { .. })).count();
        assert_eq!(icons, 1);
        let strip_fill = scene.draws.iter().position(|draw| *draw == Draw::Fill { rect: scene.regions.strip, colour: STRIP_BACK });
        let last_quad = scene.draws.iter().rposition(|draw| matches!(draw, Draw::Quad { colour, .. } if *colour != LIST_ARROW));
        assert!(strip_fill > last_quad);
        // The pointer over the covered half is over the chapter list, not the quest.
        assert_eq!(scene.action_at([area[0] - 3.0, at[1]]), None);
    }

    #[test]
    fn a_hidden_quest_cannot_stay_selected() {
        let (book, progress) = (book(), Progress::default());
        let mut view = opened(&book, &progress);
        // `far` (node 2) is hidden: no window for it, even if something selected it.
        view.selected = Some(2);
        assert_eq!(view.scene(&screen(&book, &progress, NOWHERE)).popup, None);
        view.selected = Some(99);
        assert_eq!(view.scene(&screen(&book, &progress, NOWHERE)).popup, None);
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
        assert!(texts(&shown).contains(&"drag: move | wheel: zoom"));
    }

    #[test]
    fn an_empty_pack_says_so_and_clicks_do_nothing() {
        let book = Book::new(load(None, &[]).file);
        let progress = Progress::default();
        let mut view = opened(&book, &progress);
        let shown = view.scene(&screen(&book, &progress, NOWHERE));
        assert!(texts(&shown).contains(&NO_QUESTS));
        let centre = canvas::centre(regions(RECT).canvas);
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
    fn the_window_shows_order_repeats_and_what_a_table_holds() {
        let book = gamma();
        let mut progress = Progress::default();
        let mut view = opened(&book, &progress);
        click(&mut view, &book, &progress, &Action::Quest(0));
        let check = Action::Check("g/bake/ok".into());
        let claim = Action::Claim("g/bake/loot".into());
        let pick = |entry| Action::Choose("g/bake/pick".into(), entry);

        let scene = view.scene(&screen(&book, &progress, NOWHERE));
        let shown = texts(&scene);
        assert!(shown.contains(&"Tasks, in order"), "{shown:?}");
        assert!(shown.iter().any(|t| t.contains("optional") && t.contains("repeatable")), "{shown:?}");
        // The checkmark waits for the bread, and nothing is claimed before the quest is done.
        assert_eq!(zone(&scene, &check), None);
        assert_eq!((zone(&scene, &claim), zone(&scene, &pick(0))), (None, None));
        // A choice is one icon whose window shows the entries, not to take yet.
        let open = Action::OpenChoice("g/bake/pick".into());
        let said = tip(&view, &book, &progress, &open).unwrap();
        assert_eq!(said.first().map(String::as_str), Some("Your choice: Picks"));
        assert_eq!(said.last().map(String::as_str), Some("Click to see the choices"));
        click(&mut view, &book, &progress, &open);
        let scene = view.scene(&screen(&book, &progress, NOWHERE));
        let shown = texts(&scene);
        for text in ["Choose one reward", "Finish the quest, then pick one.", "4x Iron ingot", "2 levels"] {
            assert!(shown.contains(&text), "{text} in {shown:?}");
        }
        assert_eq!(zone(&scene, &pick(0)), None);
        // A click outside it closes it.
        assert_eq!(scene.action_at([RECT[0] + 1.0, RECT[1] + 1.0]), Some(&Action::CloseChoice));
        click(&mut view, &book, &progress, &Action::CloseChoice);
        assert_eq!(view.choosing, None);

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
        let random = tip(&view, &book, &progress, &claim).unwrap();
        assert_eq!(random[..3], ["Random: Loot", "- 4x Cookie [75%]", "- A cake! [25%]"], "{random:?}");
        assert_eq!(random.last().map(String::as_str), Some("Click to claim"));
        assert_eq!(click(&mut view, &book, &progress, &claim), Reaction::Send(ToServer::Claim { reward: "g/bake/loot".into(), choice: None }));
        // Done: the choice window takes a click on an entry, and closes.
        assert_eq!(tip(&view, &book, &progress, &open).unwrap().last().map(String::as_str), Some("Click to choose"));
        click(&mut view, &book, &progress, &open);
        assert!(texts(&view.scene(&screen(&book, &progress, NOWHERE))).contains(&"Click the one you want."));
        assert_eq!(click(&mut view, &book, &progress, &pick(1)), Reaction::Send(ToServer::Claim { reward: "g/bake/pick".into(), choice: Some(1) }));
        assert_eq!(view.choosing, None);
        // Claimed: each shows only what it gave, with a check.
        let claimed = progress_after(
            &book,
            &[
                QuestEvent::Crafted { task: "g/bake/bread".into(), count: 3 },
                QuestEvent::Check { task: "g/bake/ok".into() },
                QuestEvent::Claim { reward: "g/bake/pick".into(), choice: Some(1) },
            ],
        );
        let icons = reward_icons(&screen(&book, &claimed, NOWHERE), &HashMap::new(), 0, 0);
        let pick = icons.iter().find(|icon| icon.tip.iter().any(|(line, _)| line == "Claimed")).unwrap();
        assert_eq!((pick.item.as_str(), pick.count.as_str(), pick.mark), ("minecraft:experience_bottle", "2", Some(Mark::Check)));
        let lines: Vec<&str> = pick.tip.iter().map(|(line, _)| line.as_str()).collect();
        assert_eq!(lines, ["2 levels", "Claimed"]);
        // Levels count as a number alone.
        assert_eq!(reward_item(&RewardKind::XpLevels { levels: 2 }).1, "2");

        // Started over: the window says how often it was done.
        let mut again = Progress::default();
        again.repeats.insert("g/bake".into(), 1);
        let said = |p: &Progress| texts(&view.scene(&screen(&book, p, NOWHERE))).iter().any(|t| t.contains("done once"));
        assert!(said(&again));
        again.repeats.insert("g/bake".into(), 3);
        assert!(texts(&view.scene(&screen(&book, &again, NOWHERE))).iter().any(|t| t.contains("done 3 times")));
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
