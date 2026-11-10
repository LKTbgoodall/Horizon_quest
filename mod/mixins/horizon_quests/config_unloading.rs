// A server's configs are dropped as it stops (API 397).
fn horizon_quests_config_unloading(config: &crate::game::native_configs::NativeConfigFile) {
    crate::game::hq_server_glue::config_dropped(config);
}
