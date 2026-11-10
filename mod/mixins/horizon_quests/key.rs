// The open key toggles the quest book.
fn horizon_quests_key(game: &mut crate::game::Game<'_>, key: &str, pressed: bool) {
    crate::game::hq_client_glue::key(game, key, pressed);
}
