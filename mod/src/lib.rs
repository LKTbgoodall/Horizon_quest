//! Horizon Quests' pure logic, as the bundle adds it to the game: each file is
//! a module of `crate::game` there and of this crate here, under the same
//! name, so `super::hq_canvas` reaches the same code in both. Files that touch
//! the game's types (`*_glue.rs` and the hook helpers) are not listed here.
//!
//! Every module listed here is also a `[[module]]` of `horizon-mixin.toml`.
//! The game is built with `-D warnings` and its modules are private, so code
//! no glue calls is dead code there even when `cargo test` builds clean here:
//! remove what the glue stops calling.

#[path = "../mixins/horizon_quests/hq_book.rs"]
pub mod hq_book;
#[path = "../mixins/horizon_quests/hq_canvas.rs"]
pub mod hq_canvas;
#[path = "../mixins/horizon_quests/hq_commands.rs"]
pub mod hq_commands;
#[path = "../mixins/horizon_quests/hq_config.rs"]
pub mod hq_config;
#[path = "../mixins/horizon_quests/hq_edit.rs"]
pub mod hq_edit;
#[path = "../mixins/horizon_quests/hq_logic.rs"]
pub mod hq_logic;
#[path = "../mixins/horizon_quests/hq_meta.rs"]
pub mod hq_meta;
#[path = "../mixins/horizon_quests/hq_model.rs"]
pub mod hq_model;
#[path = "../mixins/horizon_quests/hq_net.rs"]
pub mod hq_net;
#[path = "../mixins/horizon_quests/hq_progress.rs"]
pub mod hq_progress;
#[path = "../mixins/horizon_quests/hq_session.rs"]
pub mod hq_session;
#[path = "../mixins/horizon_quests/hq_store.rs"]
pub mod hq_store;
#[path = "../mixins/horizon_quests/hq_teams.rs"]
pub mod hq_teams;
#[path = "../mixins/horizon_quests/hq_validate.rs"]
pub mod hq_validate;
