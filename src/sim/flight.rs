//! Newtonian-lite flight model: throttle = target speed, rate-limited turning, flight assist.

use glam::{Quat, Vec3};

use super::command::Command;
use super::world::Entity;

/// Nose direction in the body frame (−Z), up is +Y, right is +X.
pub const FORWARD: Vec3 = Vec3::NEG_Z;
pub const UP: Vec3 = Vec3::Y;
pub const RIGHT: Vec3 = Vec3::X;

/// Performance numbers of a ship class (units/s, rad/s).
#[derive(Debug, Clone, Copy)]
pub struct ShipClass {
    pub name: &'static str,
    pub model: &'static str,
    pub texture: &'static str,
    pub radius: f32,
    pub max_speed: f32,
    pub accel: f32,
    pub strafe_speed: f32,
    pub max_pitch: f32,
    pub max_yaw: f32,
    pub max_roll: f32,
    pub turn_accel: f32,
}

pub const LONGSHIP: ShipClass = ShipClass {
    name: "Longship",
    model: "longship",
    texture: "hull_viking_a",
    radius: 22.0,
    max_speed: 320.0,
    accel: 90.0,
    strafe_speed: 70.0,
    max_pitch: 0.9,
    max_yaw: 0.8,
    max_roll: 1.4,
    turn_accel: 3.0,
};

pub const SKIFF: ShipClass = ShipClass {
    name: "Skiff",
    model: "skiff",
    texture: "hull_viking_b",
    radius: 10.0,
    max_speed: 420.0,
    accel: 160.0,
    strafe_speed: 110.0,
    max_pitch: 1.4,
    max_yaw: 1.2,
    max_roll: 2.2,
    turn_accel: 5.0,
};

pub const HAULER: ShipClass = ShipClass {
    name: "Hauler",
    model: "hauler",
    texture: "hull_minion_dark",
    radius: 30.0,
    max_speed: 120.0,
    accel: 30.0,
    strafe_speed: 20.0,
    max_pitch: 0.3,
    max_yaw: 0.3,
    max_roll: 0.3,
    turn_accel: 1.0,
};

/// Dynamic flight state and pilot demands.
#[derive(Debug, Clone, Copy)]
pub struct ShipState {
    pub class: ShipClass,
    pub throttle: f32,
    pub strafe: (f32, f32),
    /// Pitch, yaw, roll demand.
    pub turn: Vec3,
    pub brake: bool,
    pub assist: bool,
}

impl ShipState {
    pub fn new(class: ShipClass) -> ShipState {
        ShipState {
            class,
            throttle: 0.0,
            strafe: (0.0, 0.0),
            turn: Vec3::ZERO,
            brake: false,
            assist: true,
        }
    }

    pub fn apply(&mut self, cmd: Command) {
        match cmd {
            Command::SetThrottle(t) => self.throttle = t.clamp(-0.3, 1.0),
            Command::AdjustThrottle(d) => self.throttle = (self.throttle + d).clamp(-0.3, 1.0),
            Command::Turn { pitch, yaw, roll } => {
                self.turn = Vec3::new(pitch, yaw, roll).clamp(Vec3::splat(-1.0), Vec3::splat(1.0));
            }
            Command::Strafe { x, y } => self.strafe = (x.clamp(-1.0, 1.0), y.clamp(-1.0, 1.0)),
            Command::Brake(b) => self.brake = b,
            Command::SetAssist(a) => self.assist = a,
        }
    }
}

fn approach(current: Vec3, target: Vec3, max_delta: f32) -> Vec3 {
    let d = target - current;
    let len = d.length();
    if len <= max_delta {
        target
    } else {
        current + d / len * max_delta
    }
}

/// Advance one ship by `dt`.
pub fn integrate(e: &mut Entity, dt: f32) {
    let Some(s) = e.ship else { return };
    let c = s.class;

    // Rotation: body-frame rates chase the demand (pitch about X, yaw about Y, roll about Z).
    // Pitch up and yaw left are positive demands, so signs map to right-handed axes.
    let want = Vec3::new(
        s.turn.x * c.max_pitch,
        s.turn.y * c.max_yaw,
        -s.turn.z * c.max_roll,
    );
    e.ang_vel = approach(e.ang_vel, want, c.turn_accel * dt);
    e.rot = (e.rot * Quat::from_scaled_axis(e.ang_vel * dt)).normalize();

    let (fwd, right, up) = (e.rot * FORWARD, e.rot * RIGHT, e.rot * UP);
    if s.assist || s.brake {
        let desired = if s.brake {
            Vec3::ZERO
        } else {
            fwd * s.throttle * c.max_speed + (right * s.strafe.0 + up * s.strafe.1) * c.strafe_speed
        };
        let power = if s.brake { 2.0 } else { 1.0 };
        e.vel = approach(e.vel, desired, c.accel * power * dt);
    } else {
        let thrust =
            fwd * s.throttle * c.accel + (right * s.strafe.0 + up * s.strafe.1) * c.accel * 0.5;
        e.vel += thrust * dt;
    }
    let cap = c.max_speed * 1.05;
    if e.vel.length_squared() > cap * cap {
        e.vel = e.vel.normalize() * cap;
    }
    e.pos += e.vel * dt;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::world::Kind;

    fn ship() -> Entity {
        let mut e = Entity::new(Kind::Ship, "t", "longship", "hull_viking_a", 20.0);
        e.ship = Some(ShipState::new(LONGSHIP));
        e
    }

    #[test]
    fn throttle_accelerates_forward_to_target_speed() {
        let mut e = ship();
        if let Some(s) = e.ship.as_mut() {
            s.apply(Command::SetThrottle(0.5));
        }
        for _ in 0..600 {
            integrate(&mut e, 1.0 / 60.0);
        }
        assert!((e.vel.length() - 160.0).abs() < 1.0);
        assert!(e.vel.dot(FORWARD) > 0.0 && e.pos.z < 0.0);
    }

    #[test]
    fn brake_stops_the_ship() {
        let mut e = ship();
        e.vel = Vec3::new(100.0, 0.0, -100.0);
        if let Some(s) = e.ship.as_mut() {
            s.apply(Command::Brake(true));
        }
        for _ in 0..600 {
            integrate(&mut e, 1.0 / 60.0);
        }
        assert!(e.vel.length() < 0.01);
    }

    #[test]
    fn yaw_turns_toward_the_demand_and_keeps_unit_rotation() {
        let mut e = ship();
        if let Some(s) = e.ship.as_mut() {
            s.apply(Command::Turn {
                pitch: 0.0,
                yaw: 1.0,
                roll: 0.0,
            });
        }
        for _ in 0..120 {
            integrate(&mut e, 1.0 / 60.0);
        }
        assert!(
            e.forward().x < -0.3,
            "positive yaw should turn left, fwd {:?}",
            e.forward()
        );
        assert!((e.rot.length() - 1.0).abs() < 1e-4);
    }

    #[test]
    fn newtonian_mode_keeps_drifting() {
        let mut e = ship();
        e.vel = Vec3::new(50.0, 0.0, 0.0);
        if let Some(s) = e.ship.as_mut() {
            s.apply(Command::SetAssist(false));
        }
        for _ in 0..60 {
            integrate(&mut e, 1.0 / 60.0);
        }
        assert!((e.vel.x - 50.0).abs() < 1e-3);
    }
}
