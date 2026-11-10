// A mouse button went down over the quest book.
fn horizon_quests_pressed(
    game: &crate::game::Game<'_>,
    screen: &horizon_client::mod_api::NativeScreenView<'_>,
    button: i32,
    canceled: &mut bool,
) {
    crate::game::hq_client_glue::pressed(game, screen, button, canceled);
}
