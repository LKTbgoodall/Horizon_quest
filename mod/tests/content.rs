//! The content half of the mod (`mod/content`), packed with the native code
//! into one `horizon_quests.hmod`: same mod, same version, and the Quest
//! Book's model and texture where the game looks for them.

use horizon_mod_horizon_quests::hq_book::NODE_SPRITE;
use horizon_mod_horizon_quests::hq_meta::{BOOK_ITEM, ID, NAME, VERSION};
use std::path::Path;

fn content() -> &'static Path {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/content"))
}

#[test]
fn the_content_half_is_the_same_mod_and_version() {
    let text = std::fs::read_to_string(content().join("horizon-mod.toml")).unwrap();
    let manifest: toml::Table = text.parse().unwrap();
    assert_eq!(manifest["id"].as_str(), Some(ID));
    assert_eq!(manifest["name"].as_str(), Some(NAME));
    assert_eq!(manifest["version"].as_str(), Some(VERSION));
    assert_eq!(manifest["api"].as_str(), Some("0.2"));
    // Assets only: never compared with a server's copy, so a LAN guest's own build joins.
    assert_eq!(manifest["side"].as_str(), Some("client"));
}

#[test]
fn the_book_has_its_model_and_texture() {
    let (namespace, path) = BOOK_ITEM.split_once(':').unwrap();
    let assets = content().join("assets").join(namespace);
    let read = |p: &Path| -> serde_json::Value { serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap() };

    let item = read(&assets.join("items").join(format!("{path}.json")));
    let model_id = format!("{namespace}:item/{path}");
    assert_eq!(item["model"]["model"], serde_json::json!(model_id));

    let model = read(&assets.join("models/item").join(format!("{path}.json")));
    assert_eq!(model["textures"]["layer0"], serde_json::json!(model_id));

    let png = std::fs::read(assets.join("textures/item").join(format!("{path}.png"))).unwrap();
    assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
    // IHDR: 16 x 16, 8-bit RGBA.
    assert_eq!(&png[16..24], &[0, 0, 0, 16, 0, 0, 0, 16]);
    assert_eq!(&png[24..26], &[8, 6]);
}

#[test]
fn the_node_frames_are_where_the_book_draws_them_from() {
    // `horizon_quests:gui/nodes` is `assets/horizon_quests/textures/gui/nodes.png`.
    let (namespace, path) = NODE_SPRITE.split_once(':').unwrap();
    let png = std::fs::read(content().join("assets").join(namespace).join("textures").join(format!("{path}.png"))).unwrap();
    assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
    // IHDR: 5 shapes x 5 states of 32 x 32 pixels, 8-bit RGBA.
    assert_eq!(&png[16..24], &[0, 0, 0, 160, 0, 0, 0, 160]);
    assert_eq!(&png[24..26], &[8, 6]);
}

#[test]
fn the_registry_icon_is_a_small_square_png() {
    // playhorizon.dev asks for a PNG icon of at most 256 KiB.
    let png = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/icon.png")).unwrap();
    assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
    assert_eq!(&png[16..24], &[0, 0, 1, 0, 0, 0, 1, 0]);
    assert!(png.len() <= 256 * 1024);
}
