//! Things that happened during a step; the renderer, audio and mission code read these.

use glam::Vec3;

use super::sector::SectorId;
use super::world::EntityId;

/// Kinds of launched ordnance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ordnance {
    Torpedo,
    Missile,
}

#[derive(Debug, Clone, PartialEq)]
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
    /// A lance beam from `from` to `to`; `hit` is the entity struck.
    LanceFired {
        shooter: EntityId,
        from: Vec3,
        to: Vec3,
        hit: Option<EntityId>,
    },
    Launched {
        shooter: EntityId,
        kind: Ordnance,
    },
    Explosion {
        pos: Vec3,
        size: f32,
    },
    /// Shields absorbed a hit on `facing` (`ops::FORE`…); `dir` is the world direction of the hit.
    ShieldHit {
        ship: EntityId,
        facing: usize,
        dir: Vec3,
    },
    HullHit {
        ship: EntityId,
        amount: f32,
    },
    SystemDamaged {
        ship: EntityId,
        sys: super::systems::Sys,
    },
    Destroyed {
        id: EntityId,
        pos: Vec3,
        radius: f32,
        label: &'static str,
        by: Option<EntityId>,
    },
    Surrendered(EntityId),
    Docked(EntityId),
    Undocked,
    WarpJump(SectorId),
    /// Text for the comms/mission log.
    Message(String),
}
