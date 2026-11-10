// A player completed an advancement: advancement tasks.
fn horizon_quests_advancement_done(game: &mut crate::game::Game<'_>, player: u128, advancement: &str) {
    crate::game::hq_server_glue::advancement_done(game, player, advancement);
}
