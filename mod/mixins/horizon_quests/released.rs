// A mouse button came up over the quest book: a quest dragged in edit mode is dropped (API 80).
fn horizon_quests_released(
    game: &crate::game::Game<'_>,
    screen: &horizon_client::mod_api::NativeScreenView<'_>,
    button: i32,
    canceled: &mut bool,
) {
    crate::game::hq_client_glue::released(game, screen, button, canceled);
}
