// The world opened or closed, on the server.
fn horizon_quests_lifecycle(game: &mut crate::game::Game<'_>, phase: horizon_client::mod_api::WorldPhase) {
    crate::game::hq_server_glue::lifecycle(game, phase);
}
