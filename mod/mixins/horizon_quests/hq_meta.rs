// Horizon Quests' identity and the small texts the spikes show. Pure: no game
// types, so `cargo test` runs it.

/// The mod's id: its settings folder, its key lines, its net channel prefix.
pub const ID: &str = "horizon_quests";
pub const NAME: &str = "Horizon Quests";
pub const VERSION: &str = "0.0.1";
pub const DESCRIPTION: &str =
    "An FTB Quests-style quest book. Phase 0 spikes: press K for the test book, /quests on the server.";

/// The key that opens the quest book: its name in `horizon-mods.txt`, its
/// label in Options > Mods, and its default. K is free in vanilla (L opens
/// the advancements).
pub const OPEN_KEY: &str = "open_book";
pub const OPEN_KEY_LABEL: &str = "Open the quest book";
pub const OPEN_KEY_DEFAULT: &str = "key.keyboard.k";

/// `Horizon Quests 0.0.1`, as Options > Mods titles the row.
pub fn title() -> String {
    format!("{NAME} {VERSION}")
}

/// The one-line HUD hint of spike 1.
pub fn hud_hint(key_label: &str) -> String {
    format!("{} - {} opens the quest book", title(), key_label)
}

/// `key.keyboard.k` → `K`: a short label for a vanilla key name.
pub fn key_label(key: &str) -> String {
    let last = key.rsplit('.').next().unwrap_or(key);
    if last.chars().count() == 1 {
        last.to_uppercase()
    } else {
        last.to_owned()
    }
}

/// What `/quests` answers on the server (spike 4).
pub fn command_reply(player: &str, pongs: u32) -> String {
    format!(
        "{} is running on this server. Hello {player}! Book pings answered so far: {pongs}.",
        title()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_is_a_valid_registry_id() {
        assert!(ID.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_'));
    }

    #[test]
    fn key_labels_are_short() {
        assert_eq!(key_label("key.keyboard.k"), "K");
        assert_eq!(key_label("key.keyboard.f6"), "f6");
        assert_eq!(key_label("k"), "K");
    }

    #[test]
    fn texts_name_the_mod() {
        assert_eq!(title(), "Horizon Quests 0.0.1");
        assert_eq!(hud_hint("K"), "Horizon Quests 0.0.1 - K opens the quest book");
        assert!(command_reply("Steve", 3).contains("Hello Steve!"));
        assert!(command_reply("Steve", 3).ends_with("so far: 3."));
    }
}
