// A player is about to pick an item up: look at their inventory next tick.
fn horizon_quests_picked_up(game: &mut crate::game::Game<'_>, player: u128) {
    crate::game::hq_server_glue::poke(game, player);
}
