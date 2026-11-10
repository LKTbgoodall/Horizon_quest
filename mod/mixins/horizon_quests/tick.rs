// Every tick of every game: the window's book sends its waiting net frames
// and closes when a click asked it to.
fn horizon_quests_tick(game: &mut crate::game::Game<'_>) {
    crate::game::hq_client_glue::tick(game);
}
