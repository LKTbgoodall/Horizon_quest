// Every tick of every game: the window's retries a ping that could not leave.
fn horizon_quests_tick(game: &mut crate::game::Game<'_>) {
    crate::game::hq_client_glue::tick(game);
}
