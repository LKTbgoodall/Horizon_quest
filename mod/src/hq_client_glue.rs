// The client half: the graphical quest book (hq_book). A key or a right-click
// with the Quest Book opens the mod's own screen; it talks to the server over
// the channel "horizon_quests:book" (hq_net). The client state lives behind a
// Mutex: screen hooks borrow the host immutably and payloads arrive apart.

use super::hq_book::{self as book_ui, BookView, Draw, Reaction, Screen};
use super::hq_config::{self as config, ClientConfig};
use super::hq_edit::{self as edit, Edit};
use super::hq_logic::Book;
use super::hq_meta as meta;
use super::hq_model::Progress;
use super::hq_net::{self as net, ToClient, ToServer};
use super::hq_net_glue;
use mod_api::{
    ClientHost, Config, ConfigFile, ConfigKind, ConfigPath, Corner, Corners, DrawContext, Host, Identifier, ItemKey,
    ItemStack, KeyRegistration, ModEvent, PayloadBytes, Registrar, ScreenView, Text,
};
use std::collections::{HashMap, HashSet};
use std::sync::{LazyLock, Mutex};

/// This player's settings (hq_config), as `horizon_quests-client.toml` last
/// said; the defaults until the window reads it.
static CLIENT_CONFIG: Mutex<ClientConfig> = Mutex::new(ClientConfig::DEFAULT);

fn client_config() -> ClientConfig {
    *CLIENT_CONFIG.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// The open key, on Options > Controls.
pub(super) fn register(r: &mut dyn Registrar) {
    r.register_key_mapping(KeyRegistration::new(meta::OPEN_KEY, meta::OPEN_KEY_DEFAULT).label(meta::OPEN_KEY_LABEL));
}

/// A config file was read (at start, or after a change): ours, the client's.
pub(super) fn config_read(file: &ConfigFile) {
    if file.mod_id != meta::ID || file.kind != ConfigKind::Client {
        return;
    }
    let (read, problems) = config::parse_client(&file.text);
    for problem in &problems {
        log::warn!("{}: {}: {problem}", meta::NAME, file.file);
    }
    *CLIENT_CONFIG.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = read;
}

/// The client config file, `config/horizon_quests-client.toml`.
pub(super) fn config_file() -> Config {
    Config::new(meta::ID, ConfigKind::Client, config::CLIENT_TEXT)
}

struct ClientState {
    view: BookView,
    book: Option<Book>,
    progress: Progress,
    player: String,
    sender: net::Sender,
    receiver: net::Receiver,
    rect: [f32; 4],
    close_requested: bool,
    /// Window ticks since the last `Hello` went unanswered; None once the
    /// server answered (or before anything was asked).
    waited: Option<u32>,
    /// Toasts the server sent, shown on the HUD one at a time.
    toasts: book_ui::Toasts,
    /// The tracked quest was read for this world.
    settings_read: bool,
    /// A `Hello` went out on joining the world, so the tracked quest and
    /// toasts work before the book is first opened.
    greeted: bool,
    /// The item drawn for an item tag (`#minecraft:logs`): the first item of
    /// the tag. Tags met while painting wait in `tags_wanted` for the next
    /// tick, which can read the registries.
    tag_icons: HashMap<String, String>,
    tags_wanted: HashSet<String>,
    /// Item ids the painter met, resolved once (None: no such item).
    item_keys: HashMap<String, Option<ItemKey>>,
}

impl ClientState {
    fn new() -> Self {
        Self {
            view: BookView::default(),
            book: None,
            progress: Progress::default(),
            player: String::new(),
            sender: net::Sender::default(),
            receiver: net::Receiver::new(net::CLIENT_MAX_PARTS),
            rect: [0.0, 0.0, 800.0, 600.0],
            close_requested: false,
            waited: None,
            toasts: book_ui::Toasts::default(),
            settings_read: false,
            greeted: false,
            tag_icons: HashMap::new(),
            tags_wanted: HashSet::new(),
            item_keys: HashMap::new(),
        }
    }

    /// Whether anything is kept from a world besides toasts: a pack, a
    /// request, or what was done once on joining it (the settings read, the
    /// first Hello), which the next world must do again.
    fn in_use(&self) -> bool {
        self.book.is_some() || self.waited.is_some() || self.sender.waiting() > 0 || self.greeted || self.settings_read
    }
}

static STATE: LazyLock<Mutex<ClientState>> = LazyLock::new(|| Mutex::new(ClientState::new()));

fn with<R>(f: impl FnOnce(&mut ClientState) -> R) -> R {
    f(&mut STATE.lock().unwrap_or_else(|poisoned| poisoned.into_inner()))
}

fn ours(host: &dyn ClientHost) -> bool {
    host.mod_screen() == Some(meta::ID)
}

/// The file in `horizon-mod-data/horizon_quests/` that keeps the tracked
/// quest (a full quest id, empty for none).
const TRACKED_FILE: &str = "tracked.txt";
/// The icon of an item tag no item has.
const TAG_FALLBACK: &str = "minecraft:barrier";
/// The most recipes the recipe view lists for one item.
const RECIPE_LIMIT: usize = 64;

/// Text without an alpha byte is drawn opaque, as the old helpers did.
fn opaque(colour: u32) -> u32 {
    if colour >> 24 == 0 { colour | 0xFF00_0000 } else { colour }
}

/// The width of `text` as the client's font draws it.
fn text_width(host: &dyn ClientHost, text: &str) -> f32 {
    host.text_width(&Text::literal(text)).map_or(text.len() as f32 * 6.0, |w| w as f32)
}

/// The tracked quest in the top-left corner and the toast on show in the
/// top-right one, hidden while the book is open.
pub(super) fn hud(host: &dyn ClientHost, draw: &mut dyn DrawContext, corners: &mut Corners) {
    if ours(host) || !host.playing() {
        return;
    }
    let tracked = {
        let measure = |text: &str| draw.text_width(text);
        with(|s| {
            let (Some(book), Some(quest)) = (&s.book, &s.view.tracked) else {
                return None;
            };
            let [w, h] = book_ui::tracked_size(book, &s.progress, &s.player, quest, &measure)?;
            let (x, y) = corners.place(Corner::TopLeft, w, h);
            Some(book_ui::tracked_draws(book, &s.progress, &s.player, quest, [x, y], &measure))
        })
    };
    if let Some(draws) = tracked {
        paint(host, draw, draws);
    }
    if let Some(toast) = with(|s| s.toasts.current().cloned()) {
        let draws = {
            let measure = |text: &str| draw.text_width(text);
            let [w, h] = book_ui::toast_size(&toast, &measure);
            let (x, y) = corners.place(Corner::TopRight, w, h);
            book_ui::toast_draws(&toast, [x, y], &measure)
        };
        paint(host, draw, draws);
    }
}

/// The item `id` names, looked up once.
fn item_key(host: &dyn Host, id: &str) -> Option<ItemKey> {
    if let Some(found) = with(|s| s.item_keys.get(id).copied()) {
        return found;
    }
    let key = Identifier::parse(id).ok().and_then(|id| host.item_key(&id));
    with(|s| s.item_keys.insert(id.to_owned(), key));
    key
}

/// Carries a draw list out on `draw`.
fn paint(host: &dyn ClientHost, draw: &mut dyn DrawContext, draws: Vec<Draw>) {
    for one in draws {
        match one {
            Draw::Fill { rect, colour } => draw.fill(rect, colour),
            Draw::Quad { corners, colour } => draw.quad(corners, colour),
            Draw::Text { text, at, colour, shadow } => draw.text(&text, at, opaque(colour), shadow),
            Draw::Item { item, at, size } => {
                // An item tag shows one of its items, once the tick found it.
                let item = if item.starts_with('#') {
                    match with(|s| s.tag_icons.get(&item).cloned()) {
                        Some(found) => found,
                        None => {
                            with(|s| s.tags_wanted.insert(item));
                            continue;
                        }
                    }
                } else {
                    item
                };
                if let Some(key) = item_key(host, &item) {
                    draw.item_stack(&ItemStack::of(key), at, size);
                }
            }
            Draw::Sprite { name, src, dst, fallback } => {
                if !draw.sprite(&name, src, dst, None) {
                    paint(host, draw, fallback);
                }
            }
        }
    }
}

/// Opens the book screen over the world and says Hello to the server, which
/// answers with the pack and the player's progress.
fn open_book(host: &mut dyn ClientHost) {
    if host.playing() && host.mod_screen_open(meta::ID) {
        with(|s| {
            let _ = s.sender.send(&ToServer::Hello { protocol: net::PROTOCOL, version: meta::VERSION.into() });
            s.waited = Some(0);
        });
        flush_net(host);
    }
}

/// The tracked quest as the player left it.
fn read_tracked(host: &dyn ClientHost) -> Option<String> {
    let path = ConfigPath::new(TRACKED_FILE).ok()?;
    let bytes = host.data_files().read(&path).ok()?;
    let quest = String::from_utf8(bytes).ok()?.trim().to_owned();
    (!quest.is_empty()).then_some(quest)
}

fn write_tracked(host: &dyn ClientHost, quest: Option<&str>) {
    let written = ConfigPath::new(TRACKED_FILE).map_err(|e| format!("{e:?}")).and_then(|path| {
        host.data_files().write(&path, quest.unwrap_or("").as_bytes()).map_err(|e| format!("{e:?}"))
    });
    if let Err(e) = written {
        log::warn!("{}: could not keep the tracked quest: {e}", meta::NAME);
    }
}

/// Every client tick: the open key and the Quest Book, outgoing net frames,
/// a close a click asked for. Out of a world, the last world's pack and
/// requests are forgotten, so another server's book starts from nothing.
pub(super) fn tick(host: &mut dyn ClientHost) {
    if !host.playing() {
        // No world, or between two dimensions (the server, which already
        // placed the player, may send a toast meanwhile: "Somewhere
        // warmer"). The pack and requests are forgotten; the toasts wait.
        with(|s| {
            if s.in_use() {
                let toasts = std::mem::take(&mut s.toasts);
                *s = ClientState::new();
                s.toasts = toasts;
            }
        });
        return;
    }
    // The open key toggles the book.
    if host.consume_click(meta::ID, meta::OPEN_KEY) {
        if ours(host) {
            host.mod_screen_close(meta::ID);
        } else {
            open_book(host);
        }
    }
    // A right-click with the Quest Book is ours alone (the server never
    // hears of it) and opens the book as the key does.
    host.claim_items(meta::ID, &[meta::BOOK_ITEM.to_owned()]);
    let used = host.poll(meta::ID).into_iter().any(|event| matches!(event, ModEvent::UseStart(..)));
    if used && !ours(host) {
        open_book(host);
    }
    if with(|s| std::mem::take(&mut s.close_requested)) {
        host.mod_screen_close(meta::ID);
    }
    // Once per world: the tracked quest, and a Hello so the pack and the
    // progress come without opening the book.
    if !with(|s| s.settings_read) {
        let tracked = read_tracked(host);
        with(|s| {
            s.view.tracked = tracked;
            s.settings_read = true;
        });
    }
    with(|s| {
        if !s.greeted && s.book.is_none() {
            let _ = s.sender.send(&ToServer::Hello { protocol: net::PROTOCOL, version: meta::VERSION.into() });
            s.greeted = true;
        }
    });
    // The items' names in the player's language, for the book's tooltips:
    // once per world.
    if with(|s| s.view.item_names.is_empty()) {
        let names: HashMap<String, String> = host
            .item_keys()
            .into_iter()
            .map(|key| (key.id(host), host.hover_name(&ItemStack::of(key))))
            .filter(|(_, name)| !name.is_empty())
            .collect();
        with(|s| s.view.item_names = names);
    }
    // Items for the tags the book met.
    let wanted: Vec<String> = with(|s| s.tags_wanted.drain().collect());
    if !wanted.is_empty() {
        let found: Vec<(String, String)> = wanted
            .into_iter()
            .map(|tag| {
                let item = tag_items(host, &tag).into_iter().next().map(|key| key.id(host));
                (tag, item.unwrap_or_else(|| TAG_FALLBACK.to_owned()))
            })
            .collect();
        with(|s| s.tag_icons.extend(found));
    }
    // The recipes the player asked for.
    if let Some(wanted) = with(|s| s.view.recipe_wanted.take()) {
        let overlay = recipe_overlay(host, &wanted);
        with(|s| s.view.recipe = Some(overlay));
    }
    // A toast's time only runs while the HUD shows it: not under the book.
    let hud_in_view = !ours(host);
    with(|s| {
        s.view.clock = s.view.clock.wrapping_add(1);
        if let Some(recipe) = &mut s.view.recipe {
            recipe.clock = recipe.clock.wrapping_add(1);
        }
        if let Some(waited) = &mut s.waited {
            *waited = waited.saturating_add(1);
        }
        if hud_in_view {
            s.toasts.tick();
        }
    });
    flush_net(host);
}

/// The items of tag `#namespace:path` (or `namespace:path`).
fn tag_items(host: &dyn Host, tag: &str) -> Vec<ItemKey> {
    Identifier::parse(tag.trim_start_matches('#'))
        .ok()
        .and_then(|id| host.tags().key(&id))
        .and_then(|key| host.tags().items(key))
        .unwrap_or_default()
}

fn flush_net(host: &mut dyn ClientHost) {
    let frames = with(|s| s.sender.tick());
    if frames.is_empty() {
        return;
    }
    let channel = hq_net_glue::channel();
    let mut put_back = Vec::new();
    for frame in frames {
        if host.send_payload_to_server(&channel, PayloadBytes(frame.clone())).is_err() {
            put_back.push(frame);
        }
    }
    if !put_back.is_empty() {
        with(|s| s.sender.put_back(put_back));
    }
}

/// A frame from the server on channel "horizon_quests:book".
pub(super) fn from_server(_host: &mut dyn ClientHost, data: &[u8]) {
    let msg = with(|s| match s.receiver.push::<ToClient>(data) {
        Ok(msg) => msg,
        Err(e) => {
            log::warn!("{}: bad net frame from server: {e}", meta::NAME);
            None
        }
    });
    let Some(msg) = msg else {
        return;
    };
    with(|s| match msg {
        ToClient::Welcome { .. } => {
            s.waited = None;
        }
        ToClient::Incompatible { protocol, version } => {
            s.waited = None;
            s.view.notice = format!("Incompatible quest mod: server is {version} (proto {protocol})");
        }
        ToClient::Book { file } => {
            s.waited = None;
            let book = Book::new(file);
            let measure = |text: &str| text.len() as f32 * 6.0;
            let screen = Screen { book: &book, progress: &s.progress, player: &s.player, rect: s.rect, mouse: [0.0, 0.0], measure: &measure };
            s.view.open(&screen);
            s.view.pack_changed(s.book.as_ref(), &book, None);
            s.book = Some(book);
        }
        ToClient::Editor { allowed, source, advancements } => {
            s.view.editor = allowed;
            s.view.editing &= allowed;
            s.view.source = source;
            s.view.advancements = advancements;
        }
        ToClient::Progress { player, progress } => {
            s.player = player;
            s.progress = progress;
        }
        ToClient::Notice { text } => {
            s.view.notice = text;
        }
        ToClient::Party { name } => {
            s.view.party = name;
        }
        ToClient::Toast { title, text, icon } => {
            let settings = client_config();
            if settings.toasts {
                s.toasts.set_stay(settings.toast_ticks());
                s.toasts.push(book_ui::Toast { title, text, icon });
            }
        }
    });
}

/// Draws the quest book over the world.
pub(super) fn render(host: &dyn ClientHost, draw: &mut dyn DrawContext, screen: &ScreenView<'_>) {
    if !ours(host) {
        return;
    }
    let rect = screen.rect;
    let (draws, loading) = {
        let measure = |text: &str| draw.text_width(text);
        with(|s| {
            s.rect = rect;
            if let Some(book) = &s.book {
                let scr = Screen { book, progress: &s.progress, player: &s.player, rect, mouse: screen.mouse, measure: &measure };
                s.view.open(&scr);
                s.view.hover(&scr);
                s.view.partial_tick = screen.partial_tick;
                (s.view.scene(&scr).draws, None)
            } else {
                let line = book_ui::loading_line(&s.view.notice, s.waited.unwrap_or(0));
                (Vec::new(), Some(line.to_owned()))
            }
        })
    };
    if let Some(line) = loading {
        // No pack yet: a dark backdrop and where the request stands.
        draw.fill(rect, 0xD010_1018);
        let [x, y, w, h] = rect;
        draw.fill([x, y, w, 24.0], 0xFF20_2030);
        draw.text(&format!("{} - Loading", meta::title()), [x + 6.0, y + 8.0], 0xFFFF_FFFF, true);
        let nw = draw.text_width(&line);
        draw.text(&line, [x + (w - nw) / 2.0, y + h / 2.0], 0xFFFF_C040, true);
        return;
    }
    paint(host, draw, draws);
}

/// A click on the book activates buttons, chapters, quests or actions; in
/// edit mode a right click opens the edit menu. True when the book took it.
pub(super) fn pressed(host: &dyn ClientHost, screen: &ScreenView<'_>, button: i32) -> bool {
    if !ours(host) {
        return false;
    }
    if button == 0 || button == 1 {
        let reaction = on_book(host, screen.rect, screen.mouse, |view, scr| if button == 0 { view.click(scr) } else { view.right_click(scr) });
        react(host, reaction);
    }
    true
}

/// A button came up over the book: a quest dragged in edit mode is dropped
/// (left), a link dragged is dropped or a quest's menu opens (right).
pub(super) fn released(host: &dyn ClientHost, screen: &ScreenView<'_>, button: i32) -> bool {
    if !ours(host) {
        return false;
    }
    if button == 0 || button == 1 {
        let reaction = on_book(host, screen.rect, screen.mouse, |view, scr| if button == 0 { view.release(scr) } else { view.right_release(scr) });
        react(host, reaction);
    }
    true
}

/// `f` on the book as it is under the pointer; requests it returns are sent.
fn on_book(host: &dyn ClientHost, rect: [f32; 4], mouse: [f32; 2], f: impl FnOnce(&mut BookView, &Screen<'_>) -> Reaction) -> Reaction {
    let measure = |text: &str| text_width(host, text);
    with(|s| {
        let Some(book) = &s.book else {
            return Reaction::Nothing;
        };
        let scr = Screen { book, progress: &s.progress, player: &s.player, rect, mouse, measure: &measure };
        let reaction = f(&mut s.view, &scr);
        match &reaction {
            Reaction::Send(to_server) => {
                let _ = s.sender.send(to_server);
            }
            Reaction::Edit(change) => {
                let _ = s.sender.send(&ToServer::Edit { edit: change.clone() });
                edit_here(s, change);
            }
            Reaction::Nothing | Reaction::Close | Reaction::Tracked(_) => {}
        }
        reaction
    })
}

/// What the glue does after the book reacted.
fn react(host: &dyn ClientHost, reaction: Reaction) {
    match reaction {
        Reaction::Close => with(|s| s.close_requested = true),
        Reaction::Tracked(quest) => write_tracked(host, quest.as_deref()),
        Reaction::Nothing | Reaction::Send(_) | Reaction::Edit(_) => {}
    }
}

/// An edit sent to the server, shown at once on the book's own copy of the
/// pack when it checks out (the server's answer replaces it); else why not.
fn edit_here(s: &mut ClientState, change: &Edit) {
    let Some(book) = &s.book else {
        return;
    };
    let mut file = book.file().clone();
    match edit::apply(&mut file, change).and_then(|made| Ok((edit::check(&file)?, made))) {
        Ok((file, made)) => {
            let book = Book::new(file);
            s.view.pack_changed(s.book.as_ref(), &book, made.as_deref());
            s.book = Some(book);
        }
        Err(e) => s.view.notice = e,
    }
}

/// A key over the book: while text is typed or the picker is up, the book
/// takes every key. Escape closes the window on top (the picker, the edit
/// menu, the panel, the recipe view, the choice window, then the quest
/// window), and the book once none is open.
pub(super) fn screen_key(host: &dyn ClientHost, screen: &ScreenView<'_>, key: &str) -> bool {
    if !ours(host) {
        return false;
    }
    let mut taken = false;
    let reaction = on_book(host, screen.rect, screen.mouse, |view, scr| match view.key(scr, key) {
        Some(reaction) => {
            taken = true;
            reaction
        }
        None => Reaction::Nothing,
    });
    react(host, reaction);
    if !taken
        && key == "key.keyboard.escape"
        && with(|s| s.view.recipe.take().is_some() || s.view.choosing.take().is_some() || s.view.close_quest())
    {
        taken = true;
    }
    taken
}

/// A character typed over the book: into the text typed in place or the picker's search.
pub(super) fn typed(host: &dyn ClientHost, c: char) -> bool {
    ours(host) && with(|s| s.view.typed(c))
}

/// Dragging pans the canvas.
pub(super) fn dragged(host: &dyn ClientHost, screen: &ScreenView<'_>, delta: [f32; 2]) -> bool {
    if !ours(host) {
        return false;
    }
    let measure = |text: &str| text_width(host, text);
    with(|s| {
        if let Some(book) = &s.book {
            let scr = Screen { book, progress: &s.progress, player: &s.player, rect: s.rect, mouse: screen.mouse, measure: &measure };
            s.view.drag(&scr, delta);
        }
    });
    true
}

/// The wheel zooms the canvas, scrolls lists, steps a panel's number.
pub(super) fn scrolled(host: &dyn ClientHost, screen: &ScreenView<'_>, scroll: [f64; 2]) -> bool {
    if !ours(host) {
        return false;
    }
    let reaction = on_book(host, screen.rect, screen.mouse, |view, scr| view.scroll(scr, scroll[1]));
    react(host, reaction);
    true
}

/// The recipe types the recipe view lists, with where each is made.
const STATIONS: [(&str, &str); 7] = [
    ("minecraft:crafting", "Crafting table"),
    ("minecraft:smelting", "Furnace"),
    ("minecraft:blasting", "Blast furnace"),
    ("minecraft:smoking", "Smoker"),
    ("minecraft:campfire_cooking", "Campfire"),
    ("minecraft:stonecutting", "Stonecutter"),
    ("minecraft:smithing", "Smithing table"),
];

/// The recipes that make `wanted` (an item id, or `#tag`): the game's own
/// recipes as the client knows them, at most `RECIPE_LIMIT`.
fn recipe_overlay(host: &dyn ClientHost, wanted: &str) -> book_ui::RecipeOverlay {
    let plain = book_ui::pretty(wanted.trim_start_matches('#'));
    let fits: Box<dyn Fn(ItemKey) -> bool> = match wanted.strip_prefix('#') {
        Some(tag) => {
            let items = tag_items(host, tag);
            Box::new(move |key| items.contains(&key))
        }
        None => {
            let target = item_key(host, wanted);
            Box::new(move |key| Some(key) == target)
        }
    };
    let title = match (wanted.starts_with('#'), item_key(host, wanted)) {
        (false, Some(key)) => Some(host.hover_name(&ItemStack::of(key))).filter(|name| !name.is_empty()).unwrap_or(plain),
        _ => wanted.to_owned(),
    };
    let mut recipes = Vec::new();
    'kinds: for (kind, place) in STATIONS {
        for recipe in host.recipes_of_type(kind).iter() {
            if recipes.len() >= RECIPE_LIMIT {
                break 'kinds;
            }
            let Some(made) = recipe.results.iter().find(|stack| fits(stack.item())) else {
                continue;
            };
            let slots: Vec<Vec<String>> = recipe
                .ingredients
                .iter()
                .map(|ingredient| ingredient.item_keys(host).map(|keys| keys.iter().map(|key| key.id(host)).collect()).unwrap_or_default())
                .collect();
            // A shaped recipe's width is its pattern's; a shapeless one fills a 3-wide grid.
            let pattern = recipe.json.get("pattern").and_then(|p| p.as_array()).and_then(|rows| rows.first()).and_then(|row| row.as_str());
            let width = match pattern {
                Some(row) => u32::try_from(row.chars().count()).unwrap_or(3).max(1),
                None if kind == "minecraft:crafting" => u32::try_from(slots.len()).unwrap_or(3).clamp(1, 3),
                None => u32::try_from(slots.len()).unwrap_or(1).max(1),
            };
            let station = match recipe.cooking_time.filter(|ticks| *ticks > 0) {
                Some(ticks) => format!("{place}, {} s", ticks as f32 / 20.0),
                None => place.to_owned(),
            };
            recipes.push(book_ui::RecipeView { station, width, slots, result: made.item().id(host), count: made.count() });
        }
    }
    book_ui::RecipeOverlay { title, recipes, page: 0, clock: 0 }
}
