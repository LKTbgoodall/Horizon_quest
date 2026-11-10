// A player's experience level changed: experience tasks.
fn horizon_quests_level_changed(game: &mut crate::game::Game<'_>, player: u128, to: i32) {
    crate::game::hq_server_glue::level_changed(game, player, to);
}
