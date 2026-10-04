//! Player controller (T20): a port of Quake 2's `pmove` ground/air movement (friction, stopspeed,
//! accelerate) with grid-AABB collision and sliding. Units: metres, seconds.
// ref: REFERENCE_tobe_DELETED p_move.c PM_Friction, PM_Accelerate, PM_AirMove, PM_StepSlideMove

use glam::Vec3;

use super::level::{Level, STEP_H, WALL_H};
use super::weapons::{Ammo, Weapon};

pub const RADIUS: f32 = 0.4;
pub const EYE: f32 = 1.6;
pub const EYE_CROUCH: f32 = 1.0;
pub const MAX_SPEED: f32 = 6.5;
const CROUCH_SPEED: f32 = 3.0;
const ACCEL: f32 = 10.0;
const AIR_ACCEL: f32 = 1.0;
const FRICTION: f32 = 6.0;
const STOP_SPEED: f32 = 2.5;
const JUMP: f32 = 6.5;

/// Movement environment (lower gravity for zero-g sections).
#[derive(Debug, Clone, Copy)]
pub struct Env {
    pub gravity: f32,
}

impl Default for Env {
    fn default() -> Self {
        Env { gravity: 20.0 }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Input {
    /// +1 forward, -1 back.
    pub forward: f32,
    /// +1 right, -1 left.
    pub right: f32,
    pub jump: bool,
    pub crouch: bool,
    pub fire: bool,
    pub use_key: bool,
    pub weapon: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct Player {
    /// Feet position.
    pub pos: Vec3,
    pub vel: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub on_ground: bool,
    pub crouching: bool,
    pub health: f32,
    pub max_health: f32,
    pub armor: u32,
    pub weapons: [bool; 6],
    pub current: Weapon,
    pub ammo: [u32; 4],
    pub keys: [bool; 3],
    pub cooldown: f32,
    /// Seconds since the last weapon fire (viewmodel kick).
    pub kick: f32,
}

impl Player {
    pub fn new(pos: Vec3) -> Player {
        let mut weapons = [false; 6];
        weapons[Weapon::Axe.idx()] = true;
        weapons[Weapon::Blaster.idx()] = true;
        weapons[Weapon::Scatter.idx()] = true;
        let mut ammo = [0; 4];
        ammo[Ammo::Shells.idx()] = crate::balance::BOARD_START_SHELLS;
        Player {
            pos,
            vel: Vec3::ZERO,
            yaw: 0.0,
            pitch: 0.0,
            on_ground: true,
            crouching: false,
            health: 100.0,
            max_health: 100.0,
            armor: crate::balance::BOARD_START_ARMOR,
            weapons,
            current: Weapon::Blaster,
            ammo,
            keys: [false; 3],
            cooldown: 0.0,
            kick: 1.0,
        }
    }

    pub fn eye(&self) -> Vec3 {
        self.pos + Vec3::Y * if self.crouching { EYE_CROUCH } else { EYE }
    }

    /// View direction.
    pub fn look(&self) -> Vec3 {
        let (sy, cy) = self.yaw.sin_cos();
        let (sp, cp) = self.pitch.sin_cos();
        Vec3::new(-sy * cp, sp, -cy * cp)
    }

    pub fn flat_forward(&self) -> Vec3 {
        Vec3::new(-self.yaw.sin(), 0.0, -self.yaw.cos())
    }

    pub fn flat_right(&self) -> Vec3 {
        Vec3::new(self.yaw.cos(), 0.0, -self.yaw.sin())
    }

    pub fn ammo_of(&self, a: Ammo) -> u32 {
        self.ammo[a.idx()]
    }

    pub fn add_ammo(&mut self, a: Ammo, n: u32) -> bool {
        let slot = &mut self.ammo[a.idx()];
        if *slot >= a.max() {
            return false;
        }
        *slot = (*slot + n).min(a.max());
        true
    }

    /// Apply damage through armour; returns hull damage dealt.
    pub fn hurt(&mut self, damage: f32) -> f32 {
        let (left, dmg) = super::weapons::absorb(self.armor, damage);
        self.armor = left;
        self.health -= dmg;
        dmg
    }
}

/// Does a vertical cylinder at `pos` overlap a solid tile?
pub fn collides(level: &Level, pos: Vec3, radius: f32) -> bool {
    let cell = super::level::CELL;
    let (x0, x1) = (
        ((pos.x - radius) / cell).floor() as i32,
        ((pos.x + radius) / cell).floor() as i32,
    );
    let (z0, z1) = (
        ((pos.z - radius) / cell).floor() as i32,
        ((pos.z + radius) / cell).floor() as i32,
    );
    (z0..=z1).any(|z| {
        (x0..=x1).any(|x| level.solid(x, z) || level.floor_h(x, z) > pos.y + super::level::STEP_H)
    })
}

/// Move by `delta` (xz), sliding along walls; returns the velocity components that were blocked.
pub fn slide(level: &Level, pos: &mut Vec3, delta: Vec3, radius: f32) -> (bool, bool) {
    let mut blocked = (false, false);
    let nx = Vec3::new(pos.x + delta.x, pos.y, pos.z);
    if collides(level, nx, radius) {
        blocked.0 = true;
    } else {
        pos.x = nx.x;
    }
    let nz = Vec3::new(pos.x, pos.y, pos.z + delta.z);
    if collides(level, nz, radius) {
        blocked.1 = true;
    } else {
        pos.z = nz.z;
    }
    blocked
}

fn accelerate(vel: &mut Vec3, wishdir: Vec3, wishspeed: f32, accel: f32, dt: f32) {
    let current = vel.dot(wishdir);
    let add = wishspeed - current;
    if add <= 0.0 {
        return;
    }
    let a = (accel * dt * wishspeed).min(add);
    *vel += wishdir * a;
}

/// One movement step.
pub fn pmove(level: &Level, p: &mut Player, input: &Input, env: Env, dt: f32) {
    // crouch (cannot stand up under a low ceiling — the ceiling is uniform here, so always can)
    p.crouching = input.crouch && (p.on_ground || p.crouching);
    let max = if p.crouching { CROUCH_SPEED } else { MAX_SPEED };
    let wish = p.flat_forward() * input.forward + p.flat_right() * input.right;
    let wishspeed = wish.length().min(1.0) * max;
    let wishdir = wish.normalize_or_zero();

    if p.on_ground {
        // friction (PM_Friction)
        let speed = Vec3::new(p.vel.x, 0.0, p.vel.z).length();
        if speed > 0.001 {
            let control = speed.max(STOP_SPEED);
            let drop = control * FRICTION * dt;
            let new = (speed - drop).max(0.0) / speed;
            p.vel.x *= new;
            p.vel.z *= new;
        }
        accelerate(&mut p.vel, wishdir, wishspeed, ACCEL, dt);
        if input.jump && !p.crouching {
            p.vel.y = JUMP;
            p.on_ground = false;
        } else {
            p.vel.y = 0.0;
        }
    } else {
        accelerate(&mut p.vel, wishdir, wishspeed.min(max * 0.9), AIR_ACCEL, dt);
        p.vel.y -= env.gravity * dt;
    }
    let blocked = slide(
        level,
        &mut p.pos,
        Vec3::new(p.vel.x, 0.0, p.vel.z) * dt,
        RADIUS,
    );
    if blocked.0 {
        p.vel.x = 0.0;
    }
    if blocked.1 {
        p.vel.z = 0.0;
    }
    let was_ground = p.on_ground;
    p.pos.y += p.vel.y * dt;
    let head = if p.crouching { 1.2 } else { 1.8 };
    let ground = level.ground_at(p.pos, RADIUS);
    let ceiling = level.ceiling_at(p.pos, RADIUS).max(WALL_H);
    if p.pos.y <= ground || (was_ground && p.vel.y <= 0.0 && p.pos.y - ground <= STEP_H) {
        // landed, or glued to the stairs on the way down
        p.pos.y = ground;
        p.vel.y = 0.0;
        p.on_ground = true;
    } else if p.pos.y + head >= ceiling {
        p.pos.y = ceiling - head;
        p.vel.y = p.vel.y.min(0.0);
        p.on_ground = false;
    } else {
        p.on_ground = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::level::{Objective, Theme, generate};

    fn setup() -> (Level, Player) {
        let l = generate(3, Theme::Corvette, Objective::CaptureBridge, 1);
        let p = Player::new(l.spawn_point());
        (l, p)
    }

    #[test]
    fn walking_accelerates_to_max_speed_and_friction_stops() {
        let (l, mut p) = setup();
        p.yaw = 0.0;
        let go = Input {
            forward: 1.0,
            ..Default::default()
        };
        for _ in 0..30 {
            pmove(&l, &mut p, &go, Env::default(), 1.0 / 60.0);
        }
        let v = Vec3::new(p.vel.x, 0.0, p.vel.z).length();
        assert!(v > 1.0 && v <= MAX_SPEED + 0.01, "speed {v}");
        for _ in 0..120 {
            pmove(&l, &mut p, &Input::default(), Env::default(), 1.0 / 60.0);
        }
        assert!(p.vel.length() < 0.05);
    }

    #[test]
    fn walls_stop_and_slide() {
        let (l, mut p) = setup();
        // run forever in one direction: must never end up inside a wall
        let go = Input {
            forward: 1.0,
            right: 0.3,
            ..Default::default()
        };
        for i in 0..2000 {
            p.yaw += 0.01 * (i % 7) as f32;
            pmove(&l, &mut p, &go, Env::default(), 1.0 / 60.0);
            assert!(
                !collides(&l, p.pos, RADIUS - 0.001),
                "inside a wall at step {i}: {:?}",
                p.pos
            );
        }
    }

    #[test]
    fn jump_lands_and_low_gravity_floats_longer() {
        let (l, mut p) = setup();
        let jump = Input {
            jump: true,
            ..Default::default()
        };
        let air = |gravity: f32, p: &mut Player| {
            pmove(&l, p, &jump, Env { gravity }, 1.0 / 60.0);
            let mut t = 1;
            while !p.on_ground && t < 2000 {
                pmove(&l, p, &Input::default(), Env { gravity }, 1.0 / 60.0);
                t += 1;
            }
            t
        };
        let normal = air(20.0, &mut p);
        let mut q = Player::new(l.spawn_point());
        let low = air(14.0, &mut q);
        assert!(normal > 20 && low > normal, "{normal} vs {low}");
        assert!(p.on_ground && p.pos.y == 0.0);
    }

    #[test]
    fn crouching_slows_and_lowers_the_eye() {
        let (l, mut p) = setup();
        let c = Input {
            forward: 1.0,
            crouch: true,
            ..Default::default()
        };
        for _ in 0..60 {
            pmove(&l, &mut p, &c, Env::default(), 1.0 / 60.0);
        }
        assert!(Vec3::new(p.vel.x, 0.0, p.vel.z).length() <= CROUCH_SPEED + 0.01);
        assert!(p.eye().y < EYE);
    }
}
