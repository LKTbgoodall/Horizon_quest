// The mod's config files: config/horizon_quests-server.toml (API 397).
fn horizon_quests_register_configs(
    game: &mut crate::game::Game<'_>,
    configs: &mut Vec<crate::game::native_configs::NativeConfig>,
) {
    crate::game::hq_server_glue::register_configs(game, configs);
}
