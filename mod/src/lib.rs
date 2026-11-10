//! Horizon Quests' pure logic, as the bundle adds it to the game: each file is
//! a module of `crate::game` there and of this crate here, under the same
//! name, so `super::hq_canvas` reaches the same code in both. Files that touch
//! the game's types (`*_glue.rs` and the hook helpers) are not listed here.
//!
//! The phase 1 modules (`hq_model`, `hq_validate`, `hq_logic`, `hq_progress`,
//! `hq_store`, `hq_net`, `hq_book`, `hq_commands`) are listed here only, not
//! yet in `horizon-mixin.toml`: the
//! game is built with `-D warnings`, and code no glue calls yet is dead code
//! there. Each one joins the bundle with the glue that uses it (phase 2).

#[path = "../mixins/horizon_quests/hq_book.rs"]
pub mod hq_book;
#[path = "../mixins/horizon_quests/hq_canvas.rs"]
pub mod hq_canvas;
#[path = "../mixins/horizon_quests/hq_commands.rs"]
pub mod hq_commands;
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
#[path = "../mixins/horizon_quests/hq_protocol.rs"]
pub mod hq_protocol;
#[path = "../mixins/horizon_quests/hq_store.rs"]
pub mod hq_store;
#[path = "../mixins/horizon_quests/hq_validate.rs"]
pub mod hq_validate;
