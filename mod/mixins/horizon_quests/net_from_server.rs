// A mod message from the server, in the window's game.
fn horizon_quests_net_from_server(game: &mut crate::game::Game<'_>, channel: &str, data: &[u8]) {
    crate::game::hq_client_glue::from_server(game, channel, data);
}
