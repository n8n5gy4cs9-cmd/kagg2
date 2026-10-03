//! Headless simulation — must never import `macroquad` (see `ARCHITECTURE.md`).
//!
//! Fixed 60 Hz step, seeded RNG, inputs as [`command::Command`]s, outputs as [`event::Event`]s.

pub mod ai;
pub mod clock;
pub mod combat;
pub mod command;
pub mod comms;
pub mod crew;
pub mod econ;
pub mod event;
pub mod flight;
pub mod mission;
pub mod nav;
pub mod ops;
pub mod rng;
pub mod science;
pub mod sector;
pub mod systems;
pub mod world;

pub use clock::{DT, Stepper};
pub use command::{Addressed, Command};
pub use event::Event;
pub use sector::{SectorId, new_game, transfer};
pub use world::{Entity, EntityId, Kind, World};
