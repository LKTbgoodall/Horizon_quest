// The client half (the window's game): the graphical quest book (hq_book).
// A key opens the mod's own screen; it talks to the server over mod messages
// (hq_net) on channel "horizon_quests:book". Screen hooks borrow the game
// immutably, so the client state lives in a thread local on the window thread.

use super::hq_book::{self as book_ui, BookView, Draw, Reaction, Screen};
use super::hq_config::{self as config, ClientConfig};
use super::hq_logic::Book;
use super::hq_meta as meta;
use super::hq_model::Progress;
use super::hq_net::{self as net, ToClient, ToServer};
use super::native_claimed_items::NativeModEvent;
use super::native_configs::{NativeConfig, NativeConfigFile, NativeConfigKind};
use crate::game::Game;
use horizon_client::extras::Corner;
use horizon_client::extras_hud::Corners;
use horizon_client::hud::HudBatch;
use horizon_client::items::ItemStack;
use horizon_client::mod_api::NativeScreenView;
use std::cell::RefCell;
use std::sync::{Arc, Mutex};

/// This player's settings (hq_config), as `horizon_quests-client.toml` last
/// said; the defaults until the window reads it.
static CLIENT_CONFIG: Mutex<ClientConfig> = Mutex::new(ClientConfig::DEFAULT);

fn client_config() -> ClientConfig {
    *CLIENT_CONFIG.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// The window read our client config file (at start, or after a change).
pub(super) fn config_read(file: &NativeConfigFile) {
    let (read, problems) = config::parse_client(&file.text);
    for problem in &problems {
        log::warn!("{}: {}: {problem}", meta::NAME, file.path.display());
    }
    *CLIENT_CONFIG.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = read;
}

/// The client config file, registered with the server's (hq_server_glue).
pub(super) fn config_file() -> NativeConfig {
    NativeConfig::new(meta::ID, NativeConfigKind::Client, config::CLIENT_TEXT)
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
        }
    }

    /// Whether anything is kept from a world besides toasts: a pack or a
    /// request.
    fn in_use(&self) -> bool {
        self.book.is_some() || self.waited.is_some() || self.sender.waiting() > 0
    }
}

thread_local! {
    static STATE: RefCell<ClientState> = RefCell::new(ClientState::new());
}

fn with<R>(f: impl FnOnce(&mut ClientState) -> R) -> R {
    STATE.with(|b| f(&mut b.borrow_mut()))
}

fn ours(game: &Game<'_>) -> bool {
    game.native_mod_screen() == Some(meta::ID)
}

fn open_key(game: &Game<'_>) -> String {
    game.native_mod_key(meta::ID, meta::OPEN_KEY, meta::OPEN_KEY_DEFAULT)
}

/// The toast on show, in the top-right corner, hidden while the book is
/// open.
pub(super) fn hud(game: &mut Game<'_>, batch: &mut HudBatch, corners: &mut Corners) {
    if !game.native_on(meta::ID) || ours(game) || game.native_player().is_none() {
        return;
    }
    if game.native_corner_shown(Corner::TopRight)
        && let Some(toast) = with(|s| s.toasts.current().cloned())
    {
        let measure = |text: &str| game.native_text_width(text);
        let [w, h] = book_ui::toast_size(&toast, &measure);
        let (x, y) = corners.place(Corner::TopRight, w, h);
        paint(game, batch, book_ui::toast_draws(&toast, [x, y], &measure));
    }
}

/// Carries a draw list out with the game's helpers.
fn paint(game: &Game<'_>, batch: &mut HudBatch, draws: Vec<Draw>) {
    for draw in draws {
        match draw {
            Draw::Fill { rect, colour } => {
                game.native_fill(batch, rect, colour);
            }
            Draw::Quad { corners, colour } => {
                game.native_quad(batch, corners, colour);
            }
            Draw::Text { text, at, colour, shadow } => {
                game.native_text(batch, &text, at, colour, shadow);
            }
            Draw::Item { item, at, size } => {
                let stack = ItemStack { item: Arc::from(item.as_str()), count: 1, damage: 0, extra: None };
                game.native_item_stack(batch, &stack, at, size);
            }
        }
    }
}

/// The open key toggles the book screen.
pub(super) fn key(game: &mut Game<'_>, key: &str, pressed: bool) {
    if !pressed || !game.native_window() || !game.native_on(meta::ID) || key != open_key(game) {
        return;
    }
    if ours(game) {
        game.native_mod_screen_close(meta::ID);
        return;
    }
    open_book(game);
}

/// Opens the book screen over the world and says Hello to the server, which
/// answers with the pack and the player's progress.
fn open_book(game: &mut Game<'_>) {
    if game.native_playing() && game.native_mod_screen_open(meta::ID) {
        with(|s| {
            let _ = s.sender.send(&ToServer::Hello {
                protocol: net::PROTOCOL,
                version: meta::VERSION.into(),
            });
            s.waited = Some(0);
        });
        flush_net(game);
    }
}

/// Every window tick: dispatch outgoing net frames or close the screen if
/// requested. Out of a world, the last world's pack and requests are
/// forgotten, so another server's book starts from nothing.
pub(super) fn tick(game: &mut Game<'_>) {
    if !game.native_window() {
        return;
    }
    if game.native_player().is_none() {
        // No player: out of a world, or between two dimensions (the client
        // drops its player while the next level loads, and the server, which
        // already placed it, may send a toast meanwhile: "Somewhere warmer").
        // The pack and requests are forgotten; the toasts wait for the HUD.
        with(|s| {
            if s.in_use() {
                let toasts = std::mem::take(&mut s.toasts);
                *s = ClientState::new();
                s.toasts = toasts;
            }
        });
        return;
    }
    // A right-click with the quest book is ours alone (the server never
    // hears of it) and opens the book as the key does.
    game.native_claim_items(meta::ID, &[meta::BOOK_ITEM.to_owned()]);
    let used = game.native_poll(meta::ID).into_iter().any(|event| matches!(event, NativeModEvent::UseStart(..)));
    if used && !ours(game) {
        open_book(game);
    }
    if with(|s| s.close_requested) {
        with(|s| s.close_requested = false);
        game.native_mod_screen_close(meta::ID);
    }
    // A toast's time only runs while the HUD shows it: not under the book.
    let hud_in_view = !ours(game);
    with(|s| {
        if let Some(waited) = &mut s.waited {
            *waited = waited.saturating_add(1);
        }
        if hud_in_view {
            s.toasts.tick();
        }
    });
    flush_net(game);
}

fn flush_net(game: &mut Game<'_>) {
    let frames = with(|s| s.sender.tick());
    if frames.is_empty() {
        return;
    }
    let mut put_back = Vec::new();
    for frame in frames {
        if let Err(_) = game.native_net_send_to_server(net::CHANNEL, &frame) {
            put_back.push(frame);
        }
    }
    if !put_back.is_empty() {
        with(|s| s.sender.put_back(put_back));
    }
}

/// Incoming net message from the server on channel "horizon_quests:book".
pub(super) fn from_server(game: &mut Game<'_>, channel: &str, data: &[u8]) {
    if channel != net::CHANNEL || !game.native_on(meta::ID) {
        return;
    }
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
            let screen = Screen {
                book: &book,
                progress: &s.progress,
                player: &s.player,
                rect: s.rect,
                mouse: [0.0, 0.0],
                measure: &measure,
            };
            s.view.open(&screen);
            s.book = Some(book);
        }
        ToClient::Progress { player, progress } => {
            s.player = player;
            s.progress = progress;
        }
        ToClient::Notice { text } => {
            s.view.notice = text;
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
pub(super) fn render(game: &Game<'_>, batch: &mut HudBatch, screen: &NativeScreenView<'_>) {
    if !ours(game) {
        return;
    }
    let rect = screen.rect;
    let mouse = screen.mouse;
    let measure = |text: &str| game.native_text_width(text);

    let (draws, loading) = with(|s| {
        s.rect = rect;
        if let Some(book) = &s.book {
            let scr = Screen {
                book,
                progress: &s.progress,
                player: &s.player,
                rect,
                mouse,
                measure: &measure,
            };
            s.view.open(&scr);
            let scene = s.view.scene(&scr);
            (scene.draws, None)
        } else {
            let line = book_ui::loading_line(&s.view.notice, s.waited.unwrap_or(0));
            (Vec::new(), Some(line.to_owned()))
        }
    });

    if let Some(line) = loading {
        // No pack yet: a dark backdrop and where the request stands.
        game.native_fill(batch, rect, 0xD010_1018);
        let [x, y, w, h] = rect;
        game.native_fill(batch, [x, y, w, 24.0], 0xFF20_2030);
        game.native_text(batch, &format!("{} - Loading", meta::title()), [x + 6.0, y + 8.0], 0xFF_FFFF, true);
        let nw = game.native_text_width(&line);
        game.native_text(batch, &line, [x + (w - nw) / 2.0, y + h / 2.0], 0xFF_C040, true);
        return;
    }

    paint(game, batch, draws);
}

/// A click on the book activates buttons, chapters, quests or actions.
pub(super) fn pressed(game: &Game<'_>, screen: &NativeScreenView<'_>, button: i32, canceled: &mut bool) {
    if !ours(game) {
        return;
    }
    *canceled = true;
    if button != 0 {
        return;
    }
    let measure = |text: &str| game.native_text_width(text);
    let reaction = with(|s| {
        let Some(book) = &s.book else {
            return Reaction::Nothing;
        };
        let scr = Screen {
            book,
            progress: &s.progress,
            player: &s.player,
            rect: screen.rect,
            mouse: screen.mouse,
            measure: &measure,
        };
        let reaction = s.view.click(&scr);
        if let Reaction::Send(ref to_server) = reaction {
            let _ = s.sender.send(to_server);
        }
        reaction
    });
    if reaction == Reaction::Close {
        with(|s| s.close_requested = true);
    }
}

/// Dragging pans the canvas.
pub(super) fn dragged(game: &Game<'_>, delta: [f32; 2], canceled: &mut bool) {
    if !ours(game) {
        return;
    }
    *canceled = true;
    let mouse = game.native_cursor();
    let measure = |text: &str| game.native_text_width(text);
    with(|s| {
        let Some(book) = &s.book else {
            return;
        };
        let scr = Screen {
            book,
            progress: &s.progress,
            player: &s.player,
            rect: s.rect,
            mouse,
            measure: &measure,
        };
        s.view.drag(&scr, delta);
    });
}

/// The wheel zooms the canvas and scrolls lists.
pub(super) fn scrolled(game: &Game<'_>, screen: &NativeScreenView<'_>, scroll: [f64; 2], canceled: &mut bool) {
    if !ours(game) {
        return;
    }
    *canceled = true;
    let measure = |text: &str| game.native_text_width(text);
    with(|s| {
        let Some(book) = &s.book else {
            return;
        };
        let scr = Screen {
            book,
            progress: &s.progress,
            player: &s.player,
            rect: screen.rect,
            mouse: screen.mouse,
            measure: &measure,
        };
        s.view.scroll(&scr, scroll[1]);
    });
}
