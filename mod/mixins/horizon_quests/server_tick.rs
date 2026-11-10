// Every server tick: inventories, the quest engine, rewards, saving.
fn horizon_quests_server_tick(game: &mut crate::game::Game<'_>, phase: horizon_client::mod_api::ServerTickPhase) {
    crate::game::hq_server_glue::server_tick(game, phase);
}
