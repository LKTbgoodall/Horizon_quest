// The client half (the window's game): the test quest book of spikes 1 to 3.
// A key opens the mod's own screen; it draws a demo chapter on a pan/zoom
// canvas, takes clicks, drags and the wheel, and pings the server half over
// mod messages. Screen hooks borrow the game immutably, so the book's state
// lives in a thread local: the window's hooks all run on its one thread.

use super::hq_canvas::{self as canvas, View};
use super::hq_meta as meta;
use super::hq_protocol::{self as proto, Msg};
use crate::game::Game;
use horizon_client::extras::Corner;
use horizon_client::extras_hud::Corners;
use horizon_client::hud::HudBatch;
use horizon_client::items::ItemStack;
use horizon_client::mod_api::NativeScreenView;
use std::cell::RefCell;

/// Colours, `0xAARRGGBB` for fills and `0xRRGGBB` for text.
const SHADE: u32 = 0xD010_1018;
const BAND: u32 = 0xF020_2030;
const LINE: u32 = 0xFF70_7080;
const NODE_FRAME: u32 = 0xFF90_90A0;
const NODE_SELECTED: u32 = 0xFFFF_C040;
const NODE_FILL: u32 = 0xFF30_3040;
const HUD_BOX: u32 = 0x7800_0000;
const INK: u32 = 0xFF_FFFF;
const DIM_INK: u32 = 0xA0_A0B0;
const GOLD_INK: u32 = 0xFF_C040;

/// How many ticks the book keeps trying to send its ping (5 seconds).
const PING_TRIES: u32 = 100;

struct Book {
    view: View,
    selected: Option<usize>,
    /// The last ping number sent.
    sent: u32,
    /// Ticks left to try sending a ping that has not left yet; 0 when none.
    tries_left: u32,
    /// What the net spike says, for the footer.
    net: String,
}

impl Book {
    const fn new() -> Self {
        Self {
            view: View { pan: [0.0, 0.0], zoom: 1.0 },
            selected: None,
            sent: 0,
            tries_left: 0,
            net: String::new(),
        }
    }
}

thread_local! {
    static BOOK: RefCell<Book> = const { RefCell::new(Book::new()) };
}

fn with<R>(f: impl FnOnce(&mut Book) -> R) -> R {
    BOOK.with(|b| f(&mut b.borrow_mut()))
}

fn ours(game: &Game<'_>) -> bool {
    game.native_mod_screen() == Some(meta::ID)
}

fn open_key(game: &Game<'_>) -> String {
    game.native_mod_key(meta::ID, meta::OPEN_KEY, meta::OPEN_KEY_DEFAULT)
}

/// Spike 1: a one-line hint in the top-left corner.
pub fn hud(game: &mut Game<'_>, batch: &mut HudBatch, corners: &mut Corners) {
    if !game.native_on(meta::ID) || !game.native_corner_shown(Corner::TopLeft) || game.native_player().is_none() {
        return;
    }
    let line = meta::hud_hint(&meta::key_label(&open_key(game)));
    let w = game.native_text_width(&line) + 4.0;
    let (x, y) = corners.place(Corner::TopLeft, w, 12.0);
    game.native_fill(batch, [x, y, w, 12.0], HUD_BOX);
    game.native_text(batch, &line, [x + 2.0, y + 2.0], INK, true);
}

/// The open key toggles the book (spike 2) and opening it pings (spike 3).
pub fn key(game: &mut Game<'_>, key: &str, pressed: bool) {
    if !pressed || !game.native_window() || !game.native_on(meta::ID) || key != open_key(game) {
        return;
    }
    if ours(game) {
        game.native_mod_screen_close(meta::ID);
        return;
    }
    // Only over the world: not while typing in chat or with another screen.
    if game.native_playing() && game.native_mod_screen_open(meta::ID) {
        with(|b| {
            b.tries_left = PING_TRIES;
            b.net = "sending ping...".into();
        });
        try_ping(game);
    }
}

/// Every tick of the window: retry a ping that could not leave yet.
pub fn tick(game: &mut Game<'_>) {
    if game.native_window() && with(|b| b.tries_left > 0) {
        try_ping(game);
    }
}

fn try_ping(game: &mut Game<'_>) {
    let n = with(|b| b.sent + 1);
    let sent = game.native_net_send_to_server(proto::CHANNEL, &proto::encode(Msg::Ping(n)));
    with(|b| match sent {
        Ok(()) => {
            b.sent = n;
            b.tries_left = 0;
            b.net = format!("ping #{n} sent, waiting for pong...");
        }
        Err(e) => {
            b.tries_left = b.tries_left.saturating_sub(1);
            b.net = if b.tries_left == 0 {
                format!("NET FAILED: {e}")
            } else {
                format!("retrying ping: {e}")
            };
        }
    });
}

/// A mod message from the server: the pong of spike 3.
pub fn from_server(game: &mut Game<'_>, channel: &str, data: &[u8]) {
    if channel != proto::CHANNEL {
        return;
    }
    if let Some(Msg::Pong(n)) = proto::decode(data) {
        with(|b| b.net = format!("NET OK: pong #{n} received"));
        game.native_chat(&format!("[{}] the server half answered pong #{n}", meta::NAME));
    }
}

/// Draws the book over the world (spike 2).
pub fn render(game: &Game<'_>, batch: &mut HudBatch, screen: &NativeScreenView<'_>) {
    if !ours(game) {
        return;
    }
    let rect = screen.rect;
    let area = canvas::canvas_rect(rect);
    let origin = canvas::centre(area);
    let (view, selected, net) = with(|b| (b.view, b.selected, b.net.clone()));

    game.native_fill(batch, rect, SHADE);

    // Dependency lines first, under the nodes.
    let width = (2.0 * view.zoom).max(1.0);
    for (from, to) in canvas::DEMO_LINKS {
        let a = view.to_screen(origin, canvas::cell_centre(canvas::DEMO[from].cell));
        let b = view.to_screen(origin, canvas::cell_centre(canvas::DEMO[to].cell));
        if let Some(quad) = canvas::line_quad(a, b, width) {
            if canvas::overlaps(area, canvas::quad_bounds(&quad)) {
                game.native_quad(batch, quad, LINE);
            }
        }
    }

    // The nodes: a frame, a fill, the item's icon and, zoomed in, a label.
    for (i, node) in canvas::DEMO.iter().enumerate() {
        let b = canvas::node_box(&view, origin, node.cell);
        if !canvas::overlaps(area, b) {
            continue;
        }
        let frame = if selected == Some(i) { NODE_SELECTED } else { NODE_FRAME };
        game.native_fill(batch, b, frame);
        game.native_fill(batch, [b[0] + 1.0, b[1] + 1.0, b[2] - 2.0, b[3] - 2.0], NODE_FILL);
        let icon = (16.0 * view.zoom).clamp(8.0, 64.0);
        let stack = ItemStack { item: node.item.into(), count: 1, damage: 0, extra: None };
        let at = [b[0] + (b[2] - icon) / 2.0, b[1] + (b[3] - icon) / 2.0];
        game.native_item_stack(batch, &stack, at, icon);
        if view.zoom >= 0.75 {
            let w = game.native_text_width(node.label);
            game.native_text(batch, node.label, [b[0] + (b[2] - w) / 2.0, b[1] + b[3] + 2.0], DIM_INK, true);
        }
    }

    // Header and footer bands, drawn last so they cover nodes under them.
    let [x, y, w, h] = rect;
    game.native_fill(batch, [x, y, w, canvas::HEADER], BAND);
    game.native_text(batch, &format!("{} - test book (phase 0)", meta::title()), [x + 6.0, y + 8.0], INK, true);
    if let Some(i) = selected {
        let label = canvas::DEMO[i].label;
        let lw = game.native_text_width(label);
        game.native_text(batch, label, [x + w - lw - 6.0, y + 8.0], GOLD_INK, true);
    }
    let fy = y + h - canvas::FOOTER;
    game.native_fill(batch, [x, fy, w, canvas::FOOTER], BAND);
    let status = format!(
        "{net} | zoom {:.2} | pan {:.0}, {:.0} | drag: pan, wheel: zoom, click: select, Esc: close",
        view.zoom, view.pan[0], view.pan[1]
    );
    game.native_text(batch, &status, [x + 6.0, fy + 3.0], DIM_INK, false);
}

/// A click on the book selects the node under the pointer.
pub fn pressed(game: &Game<'_>, screen: &NativeScreenView<'_>, button: i32, canceled: &mut bool) {
    if !ours(game) {
        return;
    }
    *canceled = true;
    if button != 0 {
        return;
    }
    let area = canvas::canvas_rect(screen.rect);
    if !canvas::inside(area, screen.mouse) {
        return;
    }
    let origin = canvas::centre(area);
    let cells = canvas::demo_cells();
    with(|b| b.selected = canvas::hit(&b.view, origin, &cells, screen.mouse));
}

/// Dragging pans the canvas.
pub fn dragged(game: &Game<'_>, delta: [f32; 2], canceled: &mut bool) {
    if !ours(game) {
        return;
    }
    *canceled = true;
    with(|b| b.view.drag(delta));
}

/// The wheel zooms around the pointer.
pub fn scrolled(game: &Game<'_>, screen: &NativeScreenView<'_>, scroll: [f64; 2], canceled: &mut bool) {
    if !ours(game) {
        return;
    }
    *canceled = true;
    let origin = canvas::centre(canvas::canvas_rect(screen.rect));
    with(|b| b.view.zoom_at(origin, screen.mouse, scroll[1]));
}
