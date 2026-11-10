// A player's blow killed a mob: kill tasks.
fn horizon_quests_mob_killed(game: &mut crate::game::Game<'_>, player: u128, kind: &str) {
    crate::game::hq_server_glue::mob_killed(game, player, kind);
}
