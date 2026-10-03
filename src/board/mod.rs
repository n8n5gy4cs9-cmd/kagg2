//! Boarding actions (M2): level generator, player movement, weapons, monsters, level logic.
//! Headless — never imports `macroquad` (rendering lives in `gfx::board_scene`).

pub mod level;
pub mod monster;
pub mod pmove;
pub mod weapons;
pub mod world;

pub use level::{Level, Objective, Theme, generate};
pub use pmove::Input;
pub use weapons::Weapon;
pub use world::{BEvent, BoardWorld, Outcome};
