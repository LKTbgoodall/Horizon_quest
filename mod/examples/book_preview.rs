//! Draws the quest book without the game: runs `hq_book` on a few situations
//! and writes each scene's draw list as an SVG picture.
//!
//!     cargo run --example book_preview
//!
//! The pictures go to `vault/04 - Design/previews/` (the vault embeds them)
//! and a page with all of them to `target/preview/index.html`.
//!
//! What is real: every position, size, colour, text and button, which come
//! from the same code the game will run. What is not: the font (a stand-in
//! stretched to the widths below) and the item icons (boxes with initials:
//! the game draws the real ones, and no game asset is in this repository).

use horizon_mod_horizon_quests::hq_book::{Action, BookView, Draw, Scene, Screen, Typed};
use horizon_mod_horizon_quests::hq_inline::{NewTask, PanelAction, PickAction, Purpose, Subject};
use horizon_mod_horizon_quests::hq_model::Shape;
use horizon_mod_horizon_quests::hq_edit::Edit;
use horizon_mod_horizon_quests::hq_logic::Book;
use horizon_mod_horizon_quests::hq_model::Progress;
use horizon_mod_horizon_quests::hq_progress::{Actor, QuestEvent, apply};
use horizon_mod_horizon_quests::hq_validate::{load_dir, load_pack};
use std::fmt::Write as _;
use std::path::Path;

/// The window in GUI pixels: 1920x1080 at GUI scale 4.
const RECT: [f32; 4] = [0.0, 0.0, 480.0, 270.0];
/// Screen pixels per GUI pixel in the pictures.
const SCALE: f32 = 2.0;
const PLAYER: &str = "steve";
const AWAY: [f32; 2] = [-100.0, -100.0];

/// A text's width, near the game's default font: most characters advance 6
/// pixels, a few are narrower.
fn measure(text: &str) -> f32 {
    text.chars()
        .map(|c| match c {
            'i' | '!' | '.' | ',' | ':' | ';' | '|' | '\'' => 2.0,
            'l' | '`' => 3.0,
            't' | 'I' | '[' | ']' | ' ' => 4.0,
            'f' | 'k' | '<' | '>' | '"' | '(' | ')' | '*' | '{' | '}' => 5.0,
            '@' | '~' => 7.0,
            _ => 6.0,
        })
        .sum()
}

fn screen<'a>(book: &'a Book, progress: &'a Progress, mouse: [f32; 2]) -> Screen<'a> {
    Screen { book, progress, player: PLAYER, rect: RECT, mouse, measure: &measure }
}

/// A point of the zone that does `action`, low and to the right so the
/// pointer drawn there does not hide what it points at.
fn pointing_at(view: &BookView, book: &Book, progress: &Progress, action: &Action) -> [f32; 2] {
    let scene = view.scene(&screen(book, progress, AWAY));
    let (rect, _) = scene.zones.iter().rev().find(|(_, a)| a == action).expect("a zone for the action");
    [rect[0] + rect[2] * 0.8, rect[1] + rect[3] * 0.6]
}

/// The middle of the zone that does `action`.
fn zone(view: &BookView, book: &Book, progress: &Progress, action: &Action) -> [f32; 2] {
    let scene = view.scene(&screen(book, progress, AWAY));
    let (rect, _) = scene
        .zones
        .iter()
        .rev()
        .find(|(_, a)| a == action)
        .unwrap_or_else(|| panic!("nothing on screen does {action:?}"));
    [rect[0] + rect[2] / 2.0, rect[1] + rect[3] / 2.0]
}

fn click(view: &mut BookView, book: &Book, progress: &Progress, action: &Action) {
    let at = zone(view, book, progress, action);
    view.click(&screen(book, progress, at));
}

fn play(book: &Book, progress: &mut Progress, events: &[QuestEvent]) {
    for event in events {
        apply(book, progress, &Actor::new(PLAYER, 1), event);
    }
}

fn items(task: &str, count: u64) -> QuestEvent {
    QuestEvent::ItemCount { task: format!("getting_started/{task}"), count }
}

fn claim(reward: &str) -> QuestEvent {
    QuestEvent::Claim { reward: format!("getting_started/{reward}"), choice: None }
}

fn rgb(colour: u32) -> String {
    format!("#{:06x}", colour & 0xFF_FFFF)
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

/// The initials of an item's name, and a colour of its own.
fn stand_in(item: &str) -> (String, String) {
    let name = item.rsplit(':').next().unwrap_or(item);
    let initials: String = name.split('_').filter_map(|word| word.chars().next()).take(2).collect();
    let hash = item.bytes().fold(7u32, |h, b| h.wrapping_mul(31).wrapping_add(u32::from(b)));
    (initials.to_uppercase(), format!("hsl({}, 45%, 45%)", hash % 360))
}

/// The scene as an SVG picture, with the pointer when it is on screen.
fn svg(scene: &Scene, mouse: [f32; 2]) -> String {
    let [_, _, w, h] = RECT;
    let mut out = String::new();
    let _ = write!(
        out,
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {w} {h}" width="{}" height="{}" font-family="Consolas, 'DejaVu Sans Mono', monospace">"#,
        w * SCALE,
        h * SCALE
    );
    // A stand-in for the world behind the book: sky, hills, grass.
    let _ = write!(
        out,
        r##"<defs><linearGradient id="sky" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#78a7ff"/><stop offset="1" stop-color="#c8dcff"/></linearGradient></defs><rect width="{w}" height="{h}" fill="url(#sky)"/><path d="M0 170 L70 130 L150 165 L240 120 L330 160 L410 125 L480 150 L480 270 L0 270 Z" fill="#5d8f4a"/><rect y="200" width="{w}" height="70" fill="#6aa84f"/>"##
    );
    // No textures here: a sprite is drawn as its fallback.
    fn flatten(draws: &[Draw], out: &mut Vec<Draw>) {
        for draw in draws {
            match draw {
                Draw::Sprite { fallback, .. } => flatten(fallback, out),
                other => out.push(other.clone()),
            }
        }
    }
    let mut flat = Vec::new();
    flatten(&scene.draws, &mut flat);
    for draw in &flat {
        match draw {
            Draw::Fill { rect: [x, y, w, h], colour } => {
                let _ = write!(
                    out,
                    r#"<rect x="{x:.2}" y="{y:.2}" width="{w:.2}" height="{h:.2}" fill="{}" fill-opacity="{:.3}"/>"#,
                    rgb(*colour),
                    (colour >> 24) as f32 / 255.0
                );
            }
            Draw::Quad { corners, colour } => {
                let points: Vec<String> = corners.iter().map(|p| format!("{:.2},{:.2}", p[0], p[1])).collect();
                let _ = write!(
                    out,
                    r#"<polygon points="{}" fill="{}" fill-opacity="{:.3}"/>"#,
                    points.join(" "),
                    rgb(*colour),
                    (colour >> 24) as f32 / 255.0
                );
            }
            Draw::Text { text, at: [x, y], colour, shadow } => {
                let width = measure(text);
                let line = |x: f32, y: f32, fill: String| {
                    format!(
                        r#"<text x="{x:.2}" y="{:.2}" font-size="8" textLength="{width:.2}" lengthAdjust="spacingAndGlyphs" fill="{fill}">{}</text>"#,
                        y + 7.0,
                        escape(text)
                    )
                };
                if *shadow {
                    // The game's shadow: the colour at a quarter, one pixel down and right.
                    let dark = (colour >> 2) & 0x3F_3F3F;
                    out.push_str(&line(x + 1.0, y + 1.0, rgb(dark)));
                }
                out.push_str(&line(*x, *y, rgb(*colour)));
            }
            Draw::Item { item, at: [x, y], size } => {
                let (initials, colour) = stand_in(item);
                let _ = write!(
                    out,
                    r##"<rect x="{x:.2}" y="{y:.2}" width="{size:.2}" height="{size:.2}" rx="{:.2}" fill="{colour}" stroke="#101018" stroke-width="0.5"/><text x="{:.2}" y="{:.2}" font-size="{:.2}" text-anchor="middle" fill="#ffffff">{initials}</text>"##,
                    size * 0.15,
                    x + size / 2.0,
                    y + size * 0.68,
                    size * 0.5
                );
            }
            // Flattened above.
            Draw::Sprite { .. } => {}
        }
    }
    if mouse[0] >= 0.0 && mouse[1] >= 0.0 {
        let [x, y] = mouse;
        let _ = write!(
            out,
            r##"<path d="M{x:.2} {y:.2} l0 9 l2.5 -2.2 l1.8 4 l1.6 -0.7 l-1.8 -4 l3.3 0 Z" fill="#ffffff" stroke="#000000" stroke-width="0.6"/>"##
        );
    }
    out.push_str("</svg>");
    out
}

/// A pack that shows what the example pack does not: groups, every shape,
/// sizes, and a chapter in several branches.
fn showcase() -> Book {
    let settings = r#"{ "groups": [ { "id": "story", "title": "Story" }, { "id": "side", "title": "Side quests" } ] }"#;
    let shapes = r#"{ "id": "shapes", "title": "Shapes and branches", "subtitle": "Every node shape", "icon": "compass",
        "group": "story", "quests": [
        { "id": "square", "title": "Square", "x": 0, "y": 0, "icon": "oak_log" },
        { "id": "circle", "title": "Circle", "x": 2, "y": -1.5, "shape": "circle", "icon": "clock", "dependencies": ["square"] },
        { "id": "diamond", "title": "Diamond", "x": 2, "y": 1.5, "shape": "diamond", "icon": "diamond", "dependencies": ["square"] },
        { "id": "hexagon", "title": "Hexagon", "x": 4, "y": -1.5, "shape": "hexagon", "icon": "honeycomb", "dependencies": ["circle"],
          "tasks": [ { "id": "t", "type": "checkmark" } ] },
        { "id": "octagon", "title": "Old octagon", "x": 4, "y": 1.5, "shape": "octagon", "icon": "shield", "dependencies": ["diamond"],
          "tasks": [ { "id": "t", "type": "kill", "entity": "skeleton", "count": 5 } ] },
        { "id": "boss", "title": "A big one", "x": 6.5, "y": 0, "shape": "circle", "size": 2, "icon": "nether_star",
          "dependencies": ["hexagon", "octagon"], "tasks": [ { "id": "t", "type": "kill", "entity": "wither" } ],
          "rewards": [ { "id": "r", "type": "xp_levels", "levels": 10 } ] },
        { "id": "small", "title": "A small one", "x": 2, "y": 0, "shape": "circle", "size": 0.6, "icon": "torch", "optional": true,
          "dependencies": ["square"], "tasks": [ { "id": "t", "type": "checkmark" } ] }
    ] }"#;
    let nether = r#"{ "id": "nether", "title": "Into the Nether", "icon": "netherrack", "group": "story",
        "quests": [ { "id": "portal", "title": "Portal", "tasks": [ { "id": "t", "type": "item", "item": "obsidian", "count": 10 } ] } ] }"#;
    let farming = r#"{ "id": "farming", "title": "Farming", "icon": "wheat", "group": "side",
        "quests": [ { "id": "seeds", "title": "Seeds", "tasks": [ { "id": "t", "type": "item", "item": "wheat_seeds", "count": 8 } ] } ] }"#;
    let fishing = r#"{ "id": "fishing", "title": "Fishing for compliments", "icon": "fishing_rod", "group": "side",
        "quests": [ { "id": "rod", "title": "Rod", "tasks": [ { "id": "t", "type": "item", "item": "fishing_rod" } ] } ] }"#;
    let intro = r#"{ "id": "intro", "title": "Welcome", "icon": "writable_book",
        "quests": [ { "id": "hello", "title": "Hello", "tasks": [ { "id": "t", "type": "checkmark" } ] } ] }"#;
    let loaded = load_pack(
        Some(settings),
        &[("1.json", intro), ("2.json", shapes), ("3.json", nether), ("4.json", farming), ("5.json", fishing)],
        &[],
    );
    assert!(loaded.problems.is_empty(), "{:?}", loaded.problems);
    Book::new(loaded.file)
}

fn main() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let loaded = load_dir(&root.join("mixins/horizon_quests/data/quests"));
    assert!(loaded.problems.is_empty(), "{:?}", loaded.problems);
    let book = Book::new(loaded.file);
    let quest = |id: &str| Action::Quest(book.find(&format!("getting_started/{id}")).expect("a quest of the example pack"));
    let mut pictures: Vec<(&str, &str, String)> = Vec::new();

    // 1. A new player opens the book.
    let progress = Progress::default();
    let mut view = BookView::default();
    view.open(&screen(&book, &progress, AWAY));
    let mouse = pointing_at(&view, &book, &progress, &quest("first_wood"));
    pictures.push((
        "book-1-first-open",
        "A new player opens the book: one quest is open, the others are locked. The pointer is over a quest.",
        svg(&view.scene(&screen(&book, &progress, mouse)), mouse),
    ));

    // 2. They click the first quest.
    click(&mut view, &book, &progress, &quest("welcome"));
    let mouse = pointing_at(&view, &book, &progress, &Action::Check("getting_started/welcome/ready".into()));
    pictures.push((
        "book-2-quest-panel",
        "A click on a quest opens its window: tasks and rewards as icons, then its description. The pointer is on the checkmark task.",
        svg(&view.scene(&screen(&book, &progress, mouse)), mouse),
    ));

    // 3. Some way in: quests done, claimed or not, one half done.
    let mut progress = Progress::default();
    play(
        &book,
        &mut progress,
        &[
            QuestEvent::Check { task: "getting_started/welcome/ready".into() },
            items("first_wood/logs", 8),
            claim("first_wood/xp"),
            items("crafting_table/table", 1),
            items("wooden_pickaxe/pickaxe", 1),
            claim("wooden_pickaxe/xp"),
            items("stone/cobblestone", 9),
        ],
    );
    let mut view = BookView::default();
    view.open(&screen(&book, &progress, AWAY));
    click(&mut view, &book, &progress, &quest("stone"));
    pictures.push((
        "book-3-in-progress",
        "Later: green is done, blue has rewards waiting, yellow is in progress, white is open, grey is locked. The footer offers to claim everything.",
        svg(&view.scene(&screen(&book, &progress, AWAY)), AWAY),
    ));

    // 4. Near the end: a hand-in, the hidden last quest revealed, and a refusal in the footer.
    play(
        &book,
        &mut progress,
        &[
            QuestEvent::ClaimAll,
            items("stone/cobblestone", 16),
            items("furnace/furnace", 1),
            items("iron/ingots", 3),
            items("bed/bed", 1),
            QuestEvent::Killed { entity: "zombie".into() },
            QuestEvent::Killed { entity: "zombie".into() },
            QuestEvent::Killed { entity: "zombie".into() },
            QuestEvent::Submitted { task: "getting_started/leftovers/flesh".into(), count: 3 },
        ],
    );
    let mut view = BookView::default();
    view.open(&screen(&book, &progress, AWAY));
    click(&mut view, &book, &progress, &quest("leftovers"));
    view.notice = "You have no rotten flesh to hand in.".into();
    let mouse = pointing_at(&view, &book, &progress, &Action::Submit("getting_started/leftovers/flesh".into()));
    pictures.push((
        "book-4-hand-in",
        "Near the end: the surprise quest (the big diamond) has appeared, a hand-in task has a Submit button, and the footer shows what the server answered.",
        svg(&view.scene(&screen(&book, &progress, mouse)), mouse),
    ));

    // 5. Another pack: groups in the chapter list, every shape, sizes.
    let book = showcase();
    let mut progress = Progress::default();
    play(
        &book,
        &mut progress,
        &[
            QuestEvent::Check { task: "intro/hello/t".into() },
            QuestEvent::Check { task: "shapes/hexagon/t".into() },
            QuestEvent::Killed { entity: "skeleton".into() },
            QuestEvent::Killed { entity: "skeleton".into() },
        ],
    );
    let mut view = BookView::default();
    view.open(&screen(&book, &progress, AWAY));
    view.sidebar_open = true;
    click(&mut view, &book, &progress, &Action::Chapter(1));
    let mouse = pointing_at(&view, &book, &progress, &Action::Chapter(3));
    pictures.push((
        "book-5-shapes-and-groups",
        "Another pack: chapter groups in the list, the five node shapes, a big node and a small one.",
        svg(&view.scene(&screen(&book, &progress, mouse)), mouse),
    ));

    // 6. A quest done: checks on its tasks and on the claimed reward, a "!"
    // on the choice to make; then the choice window.
    let chapter = r#"{ "id": "c", "title": "Rewards", "quests": [ { "id": "q", "title": "Seasoned",
        "description": ["Pick what you like best."], "icon": "experience_bottle",
        "tasks": [ { "id": "t", "type": "checkmark" }, { "id": "w", "type": "item", "item": "oak_log", "count": 8 } ],
        "rewards": [ { "id": "xp", "type": "xp_levels", "levels": 2 }, { "id": "pick", "type": "choice", "table": "picks" } ] } ] }"#;
    let table = r#"{ "id": "picks", "entries": [ { "type": "item", "item": "iron_ingot", "count": 4 },
        { "type": "item", "item": "golden_apple" }, { "type": "xp_levels", "levels": 5 } ] }"#;
    let loaded = load_pack(None, &[("c.json", chapter)], &[("picks.json", table)]);
    assert!(loaded.problems.is_empty(), "{:?}", loaded.problems);
    let book = Book::new(loaded.file);
    let mut progress = Progress::default();
    play(
        &book,
        &mut progress,
        &[
            QuestEvent::Check { task: "c/q/t".into() },
            QuestEvent::ItemCount { task: "c/q/w".into(), count: 8 },
            QuestEvent::Claim { reward: "c/q/xp".into(), choice: None },
        ],
    );
    let mut view = BookView::default();
    view.open(&screen(&book, &progress, AWAY));
    click(&mut view, &book, &progress, &Action::Quest(0));
    let mouse = pointing_at(&view, &book, &progress, &Action::OpenChoice("c/q/pick".into()));
    pictures.push((
        "book-6-done-and-a-choice",
        "A quest done: a check on each task and on the reward claimed, a red ! on the choice still to make. The pointer is on it.",
        svg(&view.scene(&screen(&book, &progress, mouse)), mouse),
    ));
    click(&mut view, &book, &progress, &Action::OpenChoice("c/q/pick".into()));
    let mouse = pointing_at(&view, &book, &progress, &Action::Choose("c/q/pick".into(), 1));
    pictures.push((
        "book-7-choice-window",
        "A click on the choice opens a window with each reward to pick from.",
        svg(&view.scene(&screen(&book, &progress, mouse)), mouse),
    ));

    // 8. Edit mode: the pencil on, the grid, a right click on a quest.
    view.choosing = None;
    view.close_quest();
    (view.editor, view.editing, view.source) = (true, true, "world".to_owned());
    let at = pointing_at(&view, &book, &progress, &Action::Quest(0));
    view.right_click(&screen(&book, &progress, at));
    let mouse = pointing_at(&view, &book, &progress, &Action::OpenPanel(Subject::Quest("c/q".into())));
    pictures.push((
        "book-8-edit-menu",
        "Edit mode (an operator clicked the pencil): orange band, grid dots, and the menu of a right click on a quest.",
        svg(&view.scene(&screen(&book, &progress, mouse)), mouse),
    ));

    // 9. In edit mode the quest window is the editor: the title typed in place.
    view.menu = None;
    click(&mut view, &book, &progress, &Action::Quest(0));
    view.release(&screen(&book, &progress, AWAY));
    click(&mut view, &book, &progress, &Action::Type(Typed::Title("c/q".into())));
    " best".chars().for_each(|c| {
        view.typed(c);
    });
    pictures.push((
        "book-9-title-typed",
        "Edit mode: the quest window is the editor. A click on the title types it in place; the icon opens the item picker, the gear the settings.",
        svg(&view.scene(&screen(&book, &progress, AWAY)), AWAY),
    ));
    view.typing = None;

    // 10. The gear: the quest's settings in a panel beside the window.
    click(&mut view, &book, &progress, &Action::OpenPanel(Subject::Quest("c/q".into())));
    let mouse = pointing_at(&view, &book, &progress, &Action::Panel(PanelAction::Shape("shape", Some(Shape::Diamond))));
    pictures.push((
        "book-10-quest-settings",
        "The gear opens the quest's settings beside its window: the shapes as drawn, numbers with - and +, switches, choices side by side. Every click applies at once.",
        svg(&view.scene(&screen(&book, &progress, mouse)), mouse),
    ));

    // 11. The green + of the tasks: a menu of task types.
    click(&mut view, &book, &progress, &Action::AddTask("c/q".into()));
    let mouse = pointing_at(&view, &book, &progress, &Action::PickFor(Purpose::NewTask { quest: "c/q".into(), kind: NewTask::Kill }, true));
    pictures.push((
        "book-11-add-task",
        "The green + under Tasks: what kind of task, each with its icon.",
        svg(&view.scene(&screen(&book, &progress, mouse)), mouse),
    ));

    // 12. The picker: every item (here a few), with a search line.
    view.item_names = [
        ("minecraft:oak_log", "Oak Log"),
        ("minecraft:oak_planks", "Oak Planks"),
        ("minecraft:stone", "Stone"),
        ("minecraft:iron_ingot", "Iron Ingot"),
        ("minecraft:diamond", "Diamond"),
        ("minecraft:bread", "Bread"),
        ("minecraft:golden_apple", "Golden Apple"),
        ("minecraft:torch", "Torch"),
        ("minecraft:crafting_table", "Crafting Table"),
        ("minecraft:furnace", "Furnace"),
        ("minecraft:chest", "Chest"),
        ("minecraft:wheat", "Wheat"),
    ]
    .into_iter()
    .map(|(a, b)| (a.to_owned(), b.to_owned()))
    .collect();
    click(&mut view, &book, &progress, &Action::PickFor(Purpose::NewTask { quest: "c/q".into(), kind: NewTask::Have }, false));
    let mouse = pointing_at(&view, &book, &progress, &Action::Pick(PickAction::Choose("minecraft:diamond".into())));
    pictures.push((
        "book-12-item-picker",
        "Then the item picker: every item of the game, a search line (an item tag works for tasks), the wheel for more.",
        svg(&view.scene(&screen(&book, &progress, mouse)), mouse),
    ));
    view.picker = None;

    // 13. A click on a task: its panel.
    click(&mut view, &book, &progress, &Action::OpenPanel(Subject::Task("c/q/w".into())));
    let mouse = pointing_at(&view, &book, &progress, &Action::Panel(PanelAction::Step("count", 1.0)));
    pictures.push((
        "book-13-task-panel",
        "A click on a task opens its panel: the item slot opens the picker, the count steps with - / + or the wheel, a red button deletes it.",
        svg(&view.scene(&screen(&book, &progress, mouse)), mouse),
    ));
    view.panel = None;
    view.close_quest();

    // 14. The chapter list in edit mode: new chapter and group rows, a chapter's menu.
    view.sidebar_open = true;
    let at = pointing_at(&view, &book, &progress, &Action::Chapter(0));
    view.right_click(&screen(&book, &progress, at));
    let mouse = pointing_at(&view, &book, &progress, &Action::Edit(Edit::MoveChapter { chapter: "c".into(), by: 1 }));
    pictures.push((
        "book-14-edit-chapters",
        "In edit mode the chapter list ends with + New chapter and + New group; a right click on a chapter edits, moves or deletes it.",
        svg(&view.scene(&screen(&book, &progress, mouse)), mouse),
    ));

    let vault = root.join("../vault/04 - Design/previews");
    let page_dir = root.join("target/preview");
    std::fs::create_dir_all(&vault).expect("the previews folder");
    std::fs::create_dir_all(&page_dir).expect("the page's folder");
    let mut page = String::from(
        "<!doctype html><meta charset=\"utf-8\"><title>Horizon Quests: book previews</title>\
         <style>body{background:#16161d;color:#e8e8f0;font:15px/1.5 system-ui,sans-serif;margin:24px auto;max-width:980px;padding:0 16px}\
         h1{font-size:22px}figure{margin:28px 0}figcaption{margin:0 0 8px;color:#b8b8c8}svg{max-width:100%;height:auto;border:1px solid #33334a}\
         p.note{color:#9a9ab0;font-size:13px}</style>\
         <h1>Horizon Quests: the quest book, drawn without the game</h1>\
         <p class=\"note\">Positions, sizes, colours, texts and buttons come from the real layout code (hq_book). \
         The font and the item icons are stand-ins: the game draws its own font and the real item icons.</p>",
    );
    for (name, caption, picture) in &pictures {
        std::fs::write(vault.join(format!("{name}.svg")), picture).expect("a picture");
        let _ = write!(page, "<figure><figcaption>{}</figcaption>{picture}</figure>", escape(caption));
        println!("{name}.svg");
    }
    std::fs::write(page_dir.join("index.html"), page).expect("the page");
    println!("{}", page_dir.join("index.html").display());
}
