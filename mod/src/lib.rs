//! Horizon Quests' pure logic, as the bundle adds it to the game: each file is
//! a module of `crate::game` there and of this crate here, under the same
//! name, so `super::hq_canvas` reaches the same code in both. Files that touch
//! the game's types (`*_glue.rs` and the hook helpers) are not listed here.

#[path = "../mixins/horizon_quests/hq_canvas.rs"]
pub mod hq_canvas;
#[path = "../mixins/horizon_quests/hq_meta.rs"]
pub mod hq_meta;
#[path = "../mixins/horizon_quests/hq_model.rs"]
pub mod hq_model;
#[path = "../mixins/horizon_quests/hq_protocol.rs"]
pub mod hq_protocol;
#[path = "../mixins/horizon_quests/hq_validate.rs"]
pub mod hq_validate;
