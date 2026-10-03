//! Things that happened during a step; the renderer, audio and mission code read these.

use super::sector::SectorId;
use super::world::EntityId;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Event {
    Spawned(EntityId),
    Despawned(EntityId),
    /// A ship hit a solid body at `speed` (units/s).
    Collision {
        ship: EntityId,
        other: EntityId,
        speed: f32,
    },
    SectorLoaded(SectorId),
}
