// A character typed over an open screen: the quest book's edit form takes it (API 104).
fn horizon_quests_typed(game: &crate::game::Game<'_>, c: char, canceled: &mut bool) {
    crate::game::hq_client_glue::typed(game, c, canceled);
}
