// The data packs' quest folders the mod reads on each data load (API 400).
fn horizon_quests_reload_listeners(
    game: &mut crate::game::Game<'_>,
    listeners: &mut Vec<crate::game::native_reload_listeners::NativeReloadListener>,
) {
    crate::game::hq_server_glue::reload_listeners(game, listeners);
}
