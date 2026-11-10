// The mod's commands in every player's command tree.
fn horizon_quests_register_commands(commands: &mut Vec<crate::game::native_server::NativeCommand>) {
    crate::game::hq_server_glue::commands(commands);
}
