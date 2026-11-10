// A player left: save their progress.
fn horizon_quests_logged_out(game: &mut crate::game::Game<'_>, player: u128) {
    crate::game::hq_server_glue::logged_out(game, player);
}
