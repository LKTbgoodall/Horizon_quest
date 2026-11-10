// A drag over the quest book pans its canvas.
fn horizon_quests_dragged(game: &crate::game::Game<'_>, delta: [f32; 2], canceled: &mut bool) {
    crate::game::hq_client_glue::dragged(game, delta, canceled);
}
