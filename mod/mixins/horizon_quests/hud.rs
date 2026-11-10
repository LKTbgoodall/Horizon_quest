// The hint line in the top-left corner: which key opens the quest book.
fn horizon_quests_hud(
    game: &mut crate::game::Game<'_>,
    batch: &mut horizon_client::hud::HudBatch,
    corners: &mut horizon_client::extras_hud::Corners,
) {
    crate::game::hq_client_glue::hud(game, batch, corners);
}
