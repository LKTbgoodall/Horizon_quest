// The client half (the window's game): the graphical quest book (hq_book).
// A key opens the mod's own screen; it talks to the server over mod messages
// (hq_net) on channel "horizon_quests:book". Screen hooks borrow the game
// immutably, so the client state lives in a thread local on the window thread.

use super::hq_book::{self as book_ui, BookView, Draw, Reaction, Screen};
use super::hq_logic::Book;
use super::hq_meta as meta;
use super::hq_model::Progress;
use super::hq_net::{self as net, ToClient, ToServer};
use crate::game::Game;
use horizon_client::extras::Corner;
use horizon_client::extras_hud::Corners;
use horizon_client::hud::HudBatch;
use horizon_client::items::ItemStack;
use horizon_client::mod_api::NativeScreenView;
use std::cell::RefCell;
use std::sync::Arc;

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
        }
    }

    /// Whether anything is kept from a world: a pack, or a request.
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

/// A one-line hint in the top-left corner, hidden while the book is open.
pub fn hud(game: &mut Game<'_>, batch: &mut HudBatch, corners: &mut Corners) {
    if !game.native_on(meta::ID)
        || ours(game)
        || !game.native_corner_shown(Corner::TopLeft)
        || game.native_player().is_none()
    {
        return;
    }
    let line = meta::hud_hint(&meta::key_label(&open_key(game)));
    let w = game.native_text_width(&line) + 4.0;
    let (x, y) = corners.place(Corner::TopLeft, w, 12.0);
    game.native_fill(batch, [x, y, w, 12.0], 0x7800_0000);
    game.native_text(batch, &line, [x + 2.0, y + 2.0], 0xFF_FFFF, true);
}

/// The open key toggles the book screen and sends Hello to the server.
pub fn key(game: &mut Game<'_>, key: &str, pressed: bool) {
    if !pressed || !game.native_window() || !game.native_on(meta::ID) || key != open_key(game) {
        return;
    }
    if ours(game) {
        game.native_mod_screen_close(meta::ID);
        return;
    }
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
pub fn tick(game: &mut Game<'_>) {
    if !game.native_window() {
        return;
    }
    if game.native_player().is_none() {
        with(|s| {
            if s.in_use() {
                *s = ClientState::new();
            }
        });
        return;
    }
    if with(|s| s.close_requested) {
        with(|s| s.close_requested = false);
        game.native_mod_screen_close(meta::ID);
    }
    with(|s| {
        if let Some(waited) = &mut s.waited {
            *waited = waited.saturating_add(1);
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
pub fn from_server(game: &mut Game<'_>, channel: &str, data: &[u8]) {
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
    });
}

/// Draws the quest book over the world.
pub fn render(game: &Game<'_>, batch: &mut HudBatch, screen: &NativeScreenView<'_>) {
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
                let stack = ItemStack {
                    item: Arc::from(item.as_str()),
                    count: 1,
                    damage: 0,
                    extra: None,
                };
                game.native_item_stack(batch, &stack, at, size);
            }
        }
    }
}

/// A click on the book activates buttons, chapters, quests or actions.
pub fn pressed(game: &Game<'_>, screen: &NativeScreenView<'_>, button: i32, canceled: &mut bool) {
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
pub fn dragged(game: &Game<'_>, delta: [f32; 2], canceled: &mut bool) {
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
pub fn scrolled(game: &Game<'_>, screen: &NativeScreenView<'_>, scroll: [f64; 2], canceled: &mut bool) {
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
