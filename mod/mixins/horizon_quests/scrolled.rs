// The wheel over the quest book zooms its canvas.
fn horizon_quests_scrolled(
    game: &crate::game::Game<'_>,
    screen: &horizon_client::mod_api::NativeScreenView<'_>,
    scroll: [f64; 2],
    canceled: &mut bool,
) {
    crate::game::hq_client_glue::scrolled(game, screen, scroll, canceled);
}
