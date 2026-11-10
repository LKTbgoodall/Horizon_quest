// A player took a furnace's output: look at their inventory next tick.
fn horizon_quests_furnace_taken(game: &mut crate::game::Game<'_>, player: u128) {
    crate::game::hq_server_glue::poke(game, player);
}
