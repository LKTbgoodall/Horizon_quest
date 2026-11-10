// A config file changed on disk: ours when its mod id says so (API 397).
fn horizon_quests_config_reloading(
    game: &mut crate::game::Game<'_>,
    config: &crate::game::native_configs::NativeConfigFile,
) {
    crate::game::hq_server_glue::config_read(game, config, true);
}
