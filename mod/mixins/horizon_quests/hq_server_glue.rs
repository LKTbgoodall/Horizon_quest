// The server half (single player's server thread, a LAN host, later a
// dedicated server): answers the book's pings (spike 3) and `/quests`
// (spike 4). Server hooks run on the server thread, so state is atomics.

use super::hq_meta as meta;
use super::hq_protocol as proto;
use crate::game::Game;
use crate::game::native_server::{NativeCommand, NativeServerPlayer};
use horizon_client::text::Text;
use std::sync::atomic::{AtomicU32, Ordering::Relaxed};

/// The pings answered since the game started.
static PONGS: AtomicU32 = AtomicU32::new(0);

/// A mod message from a player's client: answer a ping with its pong.
pub fn from_client(game: &mut Game<'_>, player: u128, channel: &str, data: &[u8]) {
    if channel != proto::CHANNEL || !game.native_on(meta::ID) {
        return;
    }
    let Some(reply) = proto::decode(data).and_then(proto::answer) else {
        return;
    };
    match game.native_net_send_to_player(player, proto::CHANNEL, &proto::encode(reply)) {
        Ok(()) => {
            PONGS.fetch_add(1, Relaxed);
        }
        Err(e) => log::warn!("{}: could not answer a ping: {e}", meta::NAME),
    }
}

/// The mod's commands in every player's command tree.
pub fn commands(commands: &mut Vec<NativeCommand>) {
    commands.push(NativeCommand::new("quests"));
}

/// `/quests`: say the server half is here.
pub fn perform(game: &mut Game<'_>, player: &NativeServerPlayer, command: &str, ran: &mut bool) {
    if command.split_whitespace().next() != Some("quests") || !game.native_on(meta::ID) {
        return;
    }
    *ran = true;
    game.native_feedback(Text::literal(meta::command_reply(&player.name, PONGS.load(Relaxed))));
}
