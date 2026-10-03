//! Inputs to the simulation. UI, crew AI, bots and (later) the network all speak `Command`.

use super::world::EntityId;

/// What a controller wants its ship to do. Axes are `-1.0..=1.0`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Command {
    /// Set the throttle (target speed fraction, `-0.3..=1.0`).
    SetThrottle(f32),
    /// Nudge the throttle by a delta (held keys).
    AdjustThrottle(f32),
    /// Pitch/yaw/roll rate demand.
    Turn { pitch: f32, yaw: f32, roll: f32 },
    /// Lateral (x) and vertical (y) thrust demand.
    Strafe { x: f32, y: f32 },
    /// Cut velocity to zero ("match velocity" with the sector frame).
    Brake(bool),
    /// Flight assist on/off (off = pure Newtonian thrust).
    SetAssist(bool),
}

/// A command addressed to a ship — the multiplayer seam: each player/crew seat controls a ship.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Addressed {
    pub ship: EntityId,
    pub cmd: Command,
}
