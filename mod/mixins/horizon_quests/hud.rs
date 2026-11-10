// The quest toasts in the top-right corner of the HUD.
fn horizon_quests_hud(
    game: &mut crate::game::Game<'_>,
    batch: &mut horizon_client::hud::HudBatch,
    corners: &mut horizon_client::extras_hud::Corners,
) {
    crate::game::hq_client_glue::hud(game, batch, corners);
}
