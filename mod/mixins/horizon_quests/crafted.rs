// A player took a crafting result: craft tasks, and a look at their inventory.
fn horizon_quests_crafted(game: &mut crate::game::Game<'_>, player: u128, stack: &horizon_client::items::ItemStack) {
    crate::game::hq_server_glue::crafted(game, player, &stack.item, stack.count);
}
