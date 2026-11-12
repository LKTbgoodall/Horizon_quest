// A key over an open screen: the quest book takes it while text is typed, and Escape closes its windows first (API 104).
fn horizon_quests_screen_key(
    game: &crate::game::Game<'_>,
    screen: &horizon_client::mod_api::NativeScreenView<'_>,
    key: &str,
    canceled: &mut bool,
) {
    crate::game::hq_client_glue::screen_key(game, screen, key, canceled);
}
