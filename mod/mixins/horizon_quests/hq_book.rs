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
use super::hq_logic::{Book, QuestState, Status};
use super::hq_model::{ItemMatch, Progress, Quest, Reward, RewardKind, RewardTable, Shape, TableEntry, Task, TaskKind};
use super::hq_net::ToServer;
use std::collections::VecDeque;

const SHADE: u32 = 0xD010_1018;
const BAND: u32 = 0xFF20_2030;
const SIDEBAR: u32 = 0xFF18_1822;
const PANEL: u32 = 0xFF1C_1C28;
const ROW_CURRENT: u32 = 0xFF34_344A;
const ROW_HOVER: u32 = 0xFF2A_2A3A;
const BUTTON: u32 = 0xFF3A_5A8A;
const BUTTON_HOVER: u32 = 0xFF4A_74B0;
const TOOLTIP: u32 = 0xF010_0418;
/// The tooltip's frame: lighter at the top, darker at the bottom.
const TOOLTIP_EDGE_TOP: u32 = 0xFF5A_2AB8;
const TOOLTIP_EDGE_BOTTOM: u32 = 0xFF2C_1460;
const NODE_FILL: u32 = 0xFF26_2632;
const BRACKET: u32 = 0xFFFF_FFFF;
const LINE: u32 = 0xFF8C_7C88;
const LINE_DONE: u32 = 0xFF40_A050;
/// A line into the quest under the pointer: what it needs.
const LINE_NEEDS: u32 = 0xFF38_C8E8;
/// A line out of the quest under the pointer: what it opens.
const LINE_OPENS: u32 = 0xFFE8_C838;
/// A dependency line's thickness and the step between its chevrons, in
/// canvas units (a node is `canvas::NODE`).
const LINE_WIDTH: f32 = 5.0;
const CHEVRON_STEP: f32 = 9.0;
const BAR: u32 = 0xFF30_303C;
const INK: u32 = 0xFF_FFFF;
const DIM_INK: u32 = 0xA0_A0B0;
const GOLD_INK: u32 = 0xFF_C040;

/// A line of text's height, a chapter row's, and the space left round
/// things, in GUI pixels.
const TEXT: f32 = 10.0;
const ROW: f32 = 14.0;
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
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    Chapter(usize),
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
}

/// What the glue does after a click.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Reaction {
    Nothing,
    Send(ToServer),
    Close,
    /// The tracked quest changed: keep it in the player's settings.
    Tracked(Option<String>),
}

/// The book's parts on screen, as `[x, y, w, h]`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Regions {
    pub header: [f32; 4],
    pub sidebar: [f32; 4],
    pub canvas: [f32; 4],
    pub footer: [f32; 4],
}

/// The parts of a book filling `rect`. The quest window goes over the
/// canvas (`Scene::popup`).
pub fn regions(rect: [f32; 4]) -> Regions {
    let [x, y, w, h] = rect;
    let body_y = y + canvas::HEADER;
    let body_h = (h - canvas::HEADER - canvas::FOOTER).max(0.0);
    let sidebar_w = (w * 0.25).clamp(90.0, 150.0).min(w.max(0.0));
    Regions {
        header: [x, y, w, canvas::HEADER],
        sidebar: [x, body_y, sidebar_w, body_h],
        canvas: [x + sidebar_w, body_y, (w - sidebar_w).max(0.0), body_h],
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
    /// A group's heading.
    Group(String),
    Chapter(usize),
}

/// The chapter list: chapters without a group first, then each group of
/// quests.json with its chapters. Hidden chapters are not listed.
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

/// `colour` darker, for the chevrons on a line of that colour.
fn darker(colour: u32) -> u32 {
    let [a, r, g, b] = colour.to_be_bytes();
    let dim = |c: u8| (u32::from(c) * 3 / 5) as u8;
    u32::from_be_bytes([a, dim(r), dim(g), dim(b)])
}

/// A shape filled with `colour`, as quads.
fn polygon_draws(shape: Shape, centre: [f32; 2], radius: f32, colour: u32) -> Vec<Draw> {
    polygon_quads(&shape_points(shape, centre, radius)).into_iter().map(|corners| Draw::Quad { corners, colour }).collect()
}

/// The node frames' sprite sheet (`tools/make-node-textures.py`): a row per
/// shape, a column per state, each cell `NODE_CELL` pixels square.
pub const NODE_SPRITE: &str = "horizon_quests:gui/nodes";
const NODE_CELL: f32 = 32.0;

/// The cell of the node sprite sheet for a node of this shape and state.
pub fn node_cell(shape: Shape, state: QuestState) -> [f32; 4] {
    let row = match shape {
        Shape::Square => 0.0,
        Shape::Circle => 1.0,
        Shape::Diamond => 2.0,
        Shape::Hexagon => 3.0,
        Shape::Octagon => 4.0,
    };
    let column = match state {
        QuestState::Locked => 0.0,
        QuestState::Available => 1.0,
        QuestState::InProgress => 2.0,
        QuestState::RewardsWaiting => 3.0,
        QuestState::Done => 4.0,
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
        QuestState::RewardsWaiting => (0xFF50_B8FF, "Rewards waiting"),
        QuestState::Done => (0xFF40_B050, "Completed"),
    }
}

/// A node's box on screen.
fn node_box(view: &View, origin: [f32; 2], quest: &Quest, size: f32) -> [f32; 4] {
    let [sx, sy] = view.to_screen(origin, canvas::cell_centre([quest.x, quest.y]));
    let side = canvas::NODE * size * view.zoom;
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
    /// Whether a chapter was fitted to the canvas yet.
    fitted: bool,
}

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
        }
    }

    /// A left click at the pointer.
    pub fn click(&mut self, s: &Screen<'_>) -> Reaction {
        let action = self.scene(s).action_at(s.mouse).cloned();
        match action {
            Some(Action::Chapter(c)) => self.show_chapter(s, c),
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
            Some(Action::Stay) | None => {}
        }
        Reaction::Nothing
    }

    /// The pointer dragged by `delta`: over the canvas, it pans.
    pub fn drag(&mut self, s: &Screen<'_>, delta: [f32; 2]) {
        let scene = self.scene(s);
        let over_popup = scene.popup.is_some_and(|popup| canvas::inside(popup, s.mouse));
        if self.recipe.is_none() && self.choosing.is_none() && !over_popup && canvas::inside(scene.regions.canvas, s.mouse) {
            self.view.drag(delta);
        }
    }

    /// The wheel turned `steps` notches (positive: away from the player):
    /// it zooms the canvas, and scrolls the chapter list and the quest
    /// window's text.
    pub fn scroll(&mut self, s: &Screen<'_>, steps: f64) {
        let scene = self.scene(s);
        let regions = scene.regions;
        let moved = |scroll: f32, by: f32, max: f32| (scroll - steps as f32 * by).clamp(0.0, max.max(0.0));
        if !steps.is_finite() || self.recipe.is_some() || self.choosing.is_some() {
        } else if canvas::inside(regions.sidebar, s.mouse) {
            self.sidebar_scroll = moved(self.sidebar_scroll, ROW, scene.sidebar_max_scroll);
        } else if scene.popup.is_some_and(|popup| canvas::inside(popup, s.mouse)) {
            self.popup_scroll = moved(self.popup_scroll, 3.0 * TEXT, scene.popup_max_scroll);
        } else if canvas::inside(regions.canvas, s.mouse) {
            self.view.zoom_at(canvas::centre(regions.canvas), s.mouse, steps);
        }
    }

    /// Everything to draw this frame, and where clicks land.
    pub fn scene(&self, s: &Screen<'_>) -> Scene {
        let statuses = s.book.statuses(s.progress, s.player);
        let selected = self.shown_selection(s.book, &statuses);
        let regions = regions(s.rect);
        let popup = selected.map(|n| popup_layout(s, &self.item_names, self.clock, &statuses, n, regions.canvas).0);
        let mut scene = Scene {
            regions,
            draws: Vec::new(),
            zones: Vec::new(),
            sidebar_max_scroll: 0.0,
            popup_max_scroll: 0.0,
            popup,
        };
        scene.fill(s.rect, SHADE);
        let hovered = self.draw_canvas(&mut scene, s, &statuses, selected);
        self.draw_sidebar(&mut scene, s);
        self.draw_bands(&mut scene, s, &statuses);
        let mut tip = hovered.map(|n| {
            let quest = s.book.quest(n);
            vec![(if quest.title.is_empty() { quest.id.clone() } else { quest.title.clone() }, INK)]
        });
        if let (Some(n), Some(popup)) = (selected, popup) {
            tip = self.draw_popup(&mut scene, s, &statuses, n, popup);
        }
        if let Some(reward) = self.choosing.as_deref().filter(|_| self.recipe.is_none()) {
            tip = draw_choice(&mut scene, s, &self.item_names, reward);
        }
        if let Some(recipe) = &self.recipe {
            draw_recipe(&mut scene, s, recipe);
        } else if let Some(lines) = tip {
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
        let centre_of = |n: usize| {
            let quest = book.quest(n);
            self.view.to_screen(origin, canvas::cell_centre([quest.x, quest.y]))
        };

        // The quest the lines are about: the one under the pointer, else
        // the selected one. What it needs comes in cyan, what it opens goes
        // out in yellow, over the other lines.
        let hovered = shown
            .iter()
            .copied()
            .filter(|&n| {
                let b = node_box(&self.view, origin, book.quest(n), book.nodes()[n].size);
                intersect(b, area).is_some_and(|part| canvas::inside(part, s.mouse))
            })
            .last();
        let over_popup = selected.is_some_and(|n| canvas::inside(popup_layout(s, &self.item_names, self.clock, statuses, n, area).0, s.mouse));
        let hovered = hovered.filter(|_| !over_popup);
        let focus = hovered.or(selected);

        // Dependency lines first, under the nodes: a band with chevrons
        // pointing from the dependency to the quest it opens. Those to
        // another chapter's quests are not drawn.
        let width = (LINE_WIDTH * self.view.zoom).max(3.0);
        let spacing = (CHEVRON_STEP * self.view.zoom).max(8.0);
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
                } else if s.progress.is_completed(&book.nodes()[d].id) {
                    (LINE_DONE, false)
                } else {
                    (LINE, false)
                };
                lines.push((on_top, d, n, colour));
            }
        }
        // The lines of the quest in focus go last, over the others.
        lines.sort_by_key(|(on_top, ..)| *on_top);
        for (_, d, n, colour) in lines {
            let (from, to) = (centre_of(d), centre_of(n));
            if let Some(corners) = canvas::line_quad(from, to, width)
                && canvas::overlaps(area, canvas::quad_bounds(&corners))
            {
                scene.draws.push(Draw::Quad { corners, colour });
                let mark = darker(colour);
                for corners in canvas::chevrons(from, to, width, spacing) {
                    scene.draws.push(Draw::Quad { corners, colour: mark });
                }
            }
        }

        for &n in &shown {
            let node = &book.nodes()[n];
            let quest = book.quest(n);
            let at_rest = node_box(&self.view, origin, quest, node.size);
            // Only the part of the node inside the canvas takes clicks.
            let Some(part) = intersect(at_rest, area) else {
                continue;
            };
            // The node under the pointer stands out a little.
            let b = if hovered == Some(n) { grow(at_rest, HOVER_GROW) } else { at_rest };
            let centre = canvas::centre(b);
            let radius = b[2] / 2.0;
            // The frame's texture, or the same shape as polygons without it.
            let state = statuses[n].state;
            let mut fallback = polygon_draws(node.shape, centre, radius, state_look(state).0);
            fallback.extend(polygon_draws(node.shape, centre, (radius - 1.5).max(0.0), NODE_FILL));
            scene.draws.push(Draw::Sprite { name: NODE_SPRITE.to_owned(), src: node_cell(node.shape, state), dst: b, fallback });
            // A diamond has less room inside than the other shapes.
            let share = if node.shape == Shape::Diamond { 0.45 } else { 0.6 };
            let icon = (b[2] * share).clamp(4.0, 64.0);
            let at = [centre[0] - icon / 2.0, centre[1] - icon / 2.0];
            if contains(area, [at[0], at[1], icon, icon]) {
                scene.draws.push(Draw::Item { item: quest_icon(quest).to_owned(), at, size: icon });
                // A locked quest: its icon greyed out under a veil.
                if state == QuestState::Locked {
                    scene.fill([at[0], at[1], icon, icon], LOCKED_VEIL);
                }
            }
            // And a small padlock on its bottom-right corner.
            if state == QuestState::Locked {
                if let lock @ Draw::Sprite { dst, .. } = lock_draws(b)
                    && contains(area, dst)
                {
                    scene.draws.push(lock);
                }
            }
            // The selected quest: white corners round it, as a viewfinder.
            if selected == Some(n) {
                scene.draws.extend(brackets(b));
            }
            scene.zones.push((part, Action::Quest(n)));
        }
        hovered.filter(|_| canvas::inside(area, s.mouse))
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
        let track = pin[0];
        let title_left = x + PAD + 16.0;
        let title = if quest.title.is_empty() { &quest.id } else { &quest.title };
        let title = fit_text(title, track - PAD - title_left, s.measure);
        let room = track - PAD - title_left;
        let title_x = title_left + ((room - (s.measure)(&title)) / 2.0).max(0.0);
        scene.text(title, [title_x, y + 5.0], INK);

        // Tasks on the left, rewards on the right.
        let content = popup_layout(s, &self.item_names, self.clock, statuses, n, scene.regions.canvas).1;
        let top = y + 1.0 + bar + 1.0;
        let half = (w - 2.0) / 2.0;
        let columns_h = content.columns_h;
        let left = [x + 1.0, top, half, columns_h];
        let right = [x + 1.0 + half, top, half, columns_h];
        scene.fill([x + 1.0 + half, top, 1.0, columns_h], POPUP_EDGE);
        scene.fill([x + 1.0, top + columns_h, w - 2.0, 1.0], POPUP_EDGE);
        let heading = if quest.sequential_tasks { "Tasks, in order" } else { "Tasks" };
        let mut tip = over_pin.then(|| vec![(pin_tip.to_owned(), INK)]);
        tip = draw_icons(scene, s, left, heading, TASK_INK, &content.tasks).or(tip);
        tip = draw_icons(scene, s, right, "Rewards", GOLD_INK, &content.rewards).or(tip);

        // What it is, under them: it scrolls with the wheel when it is long.
        let body = [x + 1.0, top + columns_h + 1.0, w - 2.0, (h - bar - columns_h - 4.0).max(0.0)];
        scene.popup_max_scroll = (content.lines.len() as f32 * TEXT + 2.0 * PAD - body[3]).max(0.0);
        for (i, (text, colour)) in content.lines.into_iter().enumerate() {
            let line_y = body[1] + PAD + i as f32 * TEXT - self.popup_scroll;
            if contains(body, [body[0], line_y, body[2], TEXT]) {
                scene.text(text, [x + PAD, line_y + 1.0], colour);
            }
        }
        tip
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

/// The quest window: at most this big, centred over the canvas.
const POPUP_SIZE: [f32; 2] = [300.0, 200.0];
const POPUP_BAR: f32 = 16.0;
const POPUP_BACK: u32 = 0xF41C_2230;
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
}

/// The quest window's width over the canvas `area`.
fn popup_width(area: [f32; 4]) -> f32 {
    POPUP_SIZE[0].min(area[2] - 2.0 * PAD).max(0.0)
}

/// Quest `n`'s window: its place over the canvas `area`, as tall as what it
/// holds (at most `POPUP_SIZE`, and the canvas), and its content.
fn popup_layout(s: &Screen<'_>, names: &HashMap<String, String>, clock: u32, statuses: &[Status], n: usize, area: [f32; 4]) -> ([f32; 4], PopupContent) {
    let [x, y, w, h] = area;
    let pw = popup_width(area);
    let half = (pw - 2.0) / 2.0;
    let (tasks, rewards) = (task_icons(s, names, n), reward_icons(s, names, clock, n));
    let per_row = ((half - 2.0 * PAD) / ICON_CELL).floor().max(1.0) as usize;
    let rows = tasks.len().max(rewards.len()).min(ICONS_MAX).div_ceil(per_row).max(1);
    let max_h = POPUP_SIZE[1].min(h - 2.0 * PAD).max(0.0);
    let columns_h = (3.0 * PAD + TEXT + rows as f32 * ICON_CELL).min((max_h - POPUP_BAR) * 0.6);
    let lines = popup_lines(s, statuses, n, (pw - 2.0 * PAD).max(0.0));
    let wanted = POPUP_BAR + columns_h + 4.0 + lines.len() as f32 * TEXT + 2.0 * PAD;
    let ph = wanted.min(max_h);
    ([x + (w - pw) / 2.0, y + (h - ph) / 2.0, pw, ph], PopupContent { tasks, rewards, columns_h, lines })
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
        if icon.item == CHECK_TASK_ICON || icon.item == CHECK_TASK_DONE {
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
    scene.fill(b, POPUP_BACK);
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

/// White corners round box `b`, a little outside it: the selected quest.
fn brackets(b: [f32; 4]) -> Vec<Draw> {
    let thick = (b[2] * 0.07).max(1.5);
    let gap = thick + 1.5;
    let len = (b[2] * 0.3).max(4.0);
    let [x, y, w, h] = [b[0] - gap, b[1] - gap, b[2] + 2.0 * gap, b[3] + 2.0 * gap];
    let (right, bottom) = (x + w, y + h);
    [
        [x, y, len, thick],
        [x, y, thick, len],
        [right - len, y, len, thick],
        [right - thick, y, thick, len],
        [x, bottom - thick, len, thick],
        [x, bottom - len, thick, len],
        [right - len, bottom - thick, len, thick],
        [right - thick, bottom - len, thick, len],
    ]
    .into_iter()
    .map(|rect| Draw::Fill { rect, colour: BRACKET })
    .collect()
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

    #[test]
    fn regions_split_the_window() {
        let closed = regions(RECT);
        assert_eq!(closed.header, [0.0, 0.0, 480.0, canvas::HEADER]);
        assert_eq!(closed.footer, [0.0, 270.0 - canvas::FOOTER, 480.0, canvas::FOOTER]);
        assert_eq!(closed.sidebar[2] + closed.canvas[2], 480.0);
        assert_eq!(closed.canvas[1], canvas::HEADER);
        assert_eq!(closed.canvas[3], 270.0 - canvas::HEADER - canvas::FOOTER);

        // A window too small for anything: nothing goes negative.
        for rect in [[0.0, 0.0, 40.0, 20.0], [0.0, 0.0, 0.0, 0.0]] {
            let tiny = regions(rect);
            for part in [tiny.header, tiny.sidebar, tiny.canvas, tiny.footer] {
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

        let corners = |scene: &Scene| scene.draws.iter().filter(|d| matches!(d, Draw::Fill { colour, .. } if *colour == BRACKET)).count();
        assert_eq!(corners(&rest), 0);
        click(&mut view, &book, &progress, &Action::Quest(0));
        assert_eq!(corners(&view.scene(&screen(&book, &progress, NOWHERE))), 8);
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
        // A reward waiting has its "!".
        assert_eq!(signs(&view.scene(&screen(&book, &progress, NOWHERE)), MarkCell::Alert), 1);
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
        assert_eq!(node_cell(Shape::Octagon, QuestState::Done), [128.0, 128.0, 32.0, 32.0]);
        assert!(frames.iter().all(|(_, fallback)| fallback.iter().all(|d| matches!(d, Draw::Quad { .. })) && !fallback.is_empty()));
    }

    #[test]
    fn lines_carry_chevrons_and_light_up_round_the_quest_under_the_pointer() {
        let (book, progress) = (book(), Progress::default());
        let view = opened(&book, &progress);
        // The bands: quads of a line colour, each followed by its darker chevrons.
        let colours = |mouse: [f32; 2]| -> Vec<u32> {
            view.scene(&screen(&book, &progress, mouse))
                .draws
                .iter()
                .filter_map(|d| match d {
                    Draw::Quad { colour, .. } if [LINE, LINE_DONE, LINE_NEEDS, LINE_OPENS].contains(colour) => Some(*colour),
                    _ => None,
                })
                .collect()
        };
        let chevrons = view
            .scene(&screen(&book, &progress, NOWHERE))
            .draws
            .iter()
            .filter(|d| matches!(d, Draw::Quad { colour, .. } if *colour == darker(LINE)))
            .count();
        assert_eq!(colours(NOWHERE), [LINE]);
        assert!(chevrons >= 2 && chevrons % 2 == 0, "{chevrons} chevron arms");
        // Over the first quest: the line to the quest it opens is yellow.
        let first = zone(&view.scene(&screen(&book, &progress, NOWHERE)), &Action::Quest(0)).unwrap();
        assert_eq!(colours(first), [LINE_OPENS]);
        // Over the second: the line from what it needs is cyan.
        let second = zone(&view.scene(&screen(&book, &progress, NOWHERE)), &Action::Quest(1)).unwrap();
        assert_eq!(colours(second), [LINE_NEEDS]);
        // A completed dependency's line is green while nothing is in focus.
        let done = progress_after(&book, &[QuestEvent::Check { task: "a/start/ok".into() }]);
        let scene = view.scene(&screen(&book, &done, NOWHERE));
        assert!(scene.draws.iter().any(|d| matches!(d, Draw::Quad { colour, .. } if *colour == LINE_DONE)));
        assert_eq!(darker(0xFF64_C8FF), 0xFF3C_7899);
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
        let has_veil = |scene: &Scene| scene.draws.iter().any(|d| matches!(d, Draw::Fill { colour: LOCKED_VEIL, .. }));
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
        let short = |mouse| Screen { rect: [0.0, 0.0, 480.0, 150.0], ..screen(&book, &progress, mouse) };
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
        assert!(texts(&shown).contains(&"drag: move | wheel: zoom | Esc: close"));
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
