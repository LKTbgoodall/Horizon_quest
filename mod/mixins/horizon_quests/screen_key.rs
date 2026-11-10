// A key over an open screen: Escape closes the book's recipe view first (API 104).
fn horizon_quests_screen_key(game: &crate::game::Game<'_>, key: &str, canceled: &mut bool) {
    crate::game::hq_client_glue::screen_key(game, key, canceled);
}
