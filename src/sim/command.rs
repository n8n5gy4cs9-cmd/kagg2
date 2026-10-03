//! Inputs to the simulation. UI, crew AI, bots and (later) the network all speak `Command`.

use glam::Vec3;

use super::ai::Order;
use super::ops::Dest;
use super::sector::SectorId;
use super::systems::Sys;
use super::world::EntityId;

/// What a controller wants its ship to do. Axes are `-1.0..=1.0`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Command {
    /// Set the throttle (target speed fraction, `-0.3..=1.0`).
    SetThrottle(f32),
    /// Nudge the throttle by a delta (held keys).
    AdjustThrottle(f32),
    /// Pitch/yaw/roll rate demand.
    Turn {
        pitch: f32,
        yaw: f32,
        roll: f32,
    },
    /// Lateral (x) and vertical (y) thrust demand.
    Strafe {
        x: f32,
        y: f32,
    },
    /// Cut velocity to zero ("match velocity" with the sector frame).
    Brake(bool),
    /// Flight assist on/off (off = pure Newtonian thrust).
    SetAssist(bool),
    // --- weapons (T07/T08)
    FireLance,
    FireTorpedo,
    FireMissile,
    /// Retune the lance frequency (`0..=9`) / the shield frequency.
    SetLanceFreq(u8),
    SetShieldFreq(u8),
    /// Weapons/science target (also the missile lock).
    SetTarget(Option<EntityId>),
    // --- engineering (T11) and damage control (T12)
    SetPower(Sys, f32),
    SetCoolant(Sys, f32),
    PowerPreset(u8),
    /// Send repair team `0..3` to a system (or recall it).
    AssignRepair {
        team: u8,
        sys: Option<Sys>,
    },
    // --- science (T13)
    Scan(Option<EntityId>),
    Sweep,
    // --- navigation (T14)
    SetDest(Dest),
    AddWaypoint(Vec3),
    Autopilot(bool),
    Warp(Option<SectorId>),
    // --- comms (T15)
    Order(Order),
    DemandSurrender(EntityId),
    // --- starbases (T16)
    Dock(Option<EntityId>),
    Mine(Option<EntityId>),
}

/// A command addressed to a ship — the multiplayer seam: each player/crew seat controls a ship.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Addressed {
    pub ship: EntityId,
    pub cmd: Command,
}
