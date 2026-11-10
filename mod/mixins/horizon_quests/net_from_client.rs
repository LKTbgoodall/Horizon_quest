// A mod message from a player's client, on the server.
fn horizon_quests_net_from_client(game: &mut crate::game::Game<'_>, player: u128, channel: &str, data: &[u8]) {
    crate::game::hq_server_glue::from_client(game, player, channel, data);
}
