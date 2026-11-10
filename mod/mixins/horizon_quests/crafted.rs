// A player took a crafting result: look at their inventory next tick.
fn horizon_quests_crafted(game: &mut crate::game::Game<'_>, player: u128) {
    crate::game::hq_server_glue::poke(game, player);
}
