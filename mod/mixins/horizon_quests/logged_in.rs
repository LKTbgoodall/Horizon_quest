// A player joined: load their progress.
fn horizon_quests_logged_in(game: &mut crate::game::Game<'_>, player: u128, name: &str) {
    crate::game::hq_server_glue::logged_in(game, player, name);
}
