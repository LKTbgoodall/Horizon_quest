// The quest book item, declared in code (API 145): a right-click with it
// opens the book (hq_client_glue claims it).
fn horizon_quests_items(items: &mut Vec<horizon_mods::content::Item>) {
    use crate::game::hq_meta as m;
    items.push(horizon_mods::content::Item {
        id: m::BOOK_ITEM.into(),
        template: m::BOOK_TEMPLATE.into(),
        max_stack_size: Some(1),
        rarity: Some(horizon_mods::content::Rarity::Epic),
        components: m::book_components().into_iter().collect(),
        crafting_remainder: None,
        place_block: None,
    });
}
