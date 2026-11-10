// A player took a furnace's output: craft tasks, and a look at their inventory.
fn horizon_quests_furnace_taken(game: &mut crate::game::Game<'_>, player: u128, item: &str, count: u32) {
    crate::game::hq_server_glue::crafted(game, player, item, count);
}
