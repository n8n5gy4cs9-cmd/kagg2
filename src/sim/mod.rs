//! Headless simulation — must never import `macroquad` (see `ARCHITECTURE.md`).
//!
//! Fixed 60 Hz step, seeded RNG, inputs as [`command::Command`]s, outputs as [`event::Event`]s.

pub mod clock;
pub mod command;
pub mod event;
pub mod flight;
pub mod rng;
pub mod sector;
pub mod world;

pub use clock::{DT, Stepper};
pub use command::{Addressed, Command};
pub use event::Event;
pub use sector::{SectorId, new_game};
pub use world::{Entity, EntityId, Kind, World};
