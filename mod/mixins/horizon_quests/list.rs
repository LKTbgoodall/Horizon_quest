// The mod's row in Options > Mods, with its rebindable key.
fn horizon_quests_listed(mods: &mut Vec<crate::game::native_hooks::NativeMod>) {
    use crate::game::hq_meta as m;
    mods.push(crate::game::native_hooks::NativeMod {
        id: m::ID.into(),
        title: m::title(),
        description: m::DESCRIPTION.into(),
        keys: vec![[m::OPEN_KEY.into(), m::OPEN_KEY_LABEL.into(), m::OPEN_KEY_DEFAULT.into()]],
    });
}
