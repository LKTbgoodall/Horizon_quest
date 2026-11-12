// Horizon Quests' identity and its small fixed texts (Options > Mods, the
// quest book item, `/quests`). Pure: no game types, so `cargo test` runs it.

/// The mod's id: its settings folder, its key lines, its net channel prefix.
pub const ID: &str = "horizon_quests";
pub const NAME: &str = "Horizon Quests";
pub const VERSION: &str = "0.16.2";
pub const DESCRIPTION: &str =
    "An FTB Quests-style quest book: a graphical book, quests with tasks and rewards, synced with the server.";

/// The key that opens the quest book: its name in `horizon-mods.txt`, its
/// label in Options > Mods, and its default. K is free in vanilla (L opens
/// the advancements).
pub const OPEN_KEY: &str = "open_book";
pub const OPEN_KEY_LABEL: &str = "Open the quest book";
pub const OPEN_KEY_DEFAULT: &str = "key.keyboard.k";

/// `Horizon Quests 0.16.2`, as Options > Mods titles the row.
pub fn title() -> String {
    format!("{NAME} {VERSION}")
}

/// The quest book item: a right-click with it opens the book, as the key
/// does. Declared in code (API 145) over a vanilla template, whose look it
/// keeps while no content mod gives it its own.
pub const BOOK_ITEM: &str = "horizon_quests:quest_book";
pub const BOOK_TEMPLATE: &str = "minecraft:book";
pub const BOOK_NAME: &str = "Quest Book";

/// The book's data components over its template's, as a content mod's item
/// file writes them: its name and the line under it, an enchanted shine, and
/// no enchanting (a vanilla book would turn into an enchanted book).
pub fn book_components() -> Vec<(String, serde_json::Value)> {
    use serde_json::json;
    vec![
        ("minecraft:item_name".into(), json!({ "text": BOOK_NAME })),
        ("minecraft:lore".into(), json!([{ "text": "Right-click to open your quests", "color": "gray", "italic": false }])),
        ("minecraft:enchantment_glint_override".into(), json!(true)),
        ("!minecraft:enchantable".into(), json!({})),
    ]
}

/// The first line `/quests` answers on the server, before the help.
pub fn command_reply(player: &str) -> String {
    format!("{} is running on this server. Hello {player}!", title())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_is_a_valid_registry_id() {
        assert!(ID.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_'));
    }

    #[test]
    fn the_book_is_ours_and_named() {
        assert_eq!(BOOK_ITEM.split_once(':').map(|(ns, _)| ns), Some(ID));
        let components = book_components();
        assert!(components.iter().all(|(k, _)| k.trim_start_matches('!').starts_with("minecraft:")));
        let name = components.iter().find(|(k, _)| k == "minecraft:item_name").map(|(_, v)| v["text"].clone());
        assert_eq!(name, Some(serde_json::json!(BOOK_NAME)));
    }

    #[test]
    fn texts_name_the_mod() {
        assert_eq!(title(), format!("{NAME} {VERSION}"));
        assert_eq!(command_reply("Steve"), format!("{NAME} {VERSION} is running on this server. Hello Steve!"));
    }
}
