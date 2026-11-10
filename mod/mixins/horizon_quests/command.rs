// A player's command on the server: the mod's own, when it is one.
fn horizon_quests_perform_command(
    game: &mut crate::game::Game<'_>,
    player: &crate::game::native_server::NativeServerPlayer,
    command: &str,
    ran: &mut bool,
) {
    if !*ran {
        crate::game::hq_server_glue::perform(game, player, command, ran);
    }
}
