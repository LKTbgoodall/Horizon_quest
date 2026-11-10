// Draws the quest book when its screen is the open one.
fn horizon_quests_render(
    game: &crate::game::Game<'_>,
    batch: &mut horizon_client::hud::HudBatch,
    screen: &horizon_client::mod_api::NativeScreenView<'_>,
) {
    crate::game::hq_client_glue::render(game, batch, screen);
}
