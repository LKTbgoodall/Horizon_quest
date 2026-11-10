// A listener's files were read from the data packs: ours when its id says so (API 400).
fn horizon_quests_reload_listener_apply(
    game: &mut crate::game::Game<'_>,
    listener: &crate::game::native_reload_listeners::NativeReloadListener,
    files: &[(String, serde_json::Value)],
) {
    crate::game::hq_server_glue::reload_listener_apply(game, listener, files);
}
