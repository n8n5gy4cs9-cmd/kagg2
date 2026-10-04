//! Minion monsters (T22): a Quake 2 style state machine — stand → walk → run → attack → pain →
//! dead — with sight (cone + line of sight), hearing (noise alerts) and the medic's revive.
// ref: REFERENCE_tobe_DELETED g_ai.c ai_stand/ai_walk/ai_run/ai_charge, FindTarget visibility;
//      m_medic.c medic_cable_attack (revive idea)

use glam::Vec3;

use super::level::{CELL, Level, MonsterKind};
use super::pmove::{Player, slide};
use crate::sim::rng::Rng;

#[derive(Debug, Clone, Copy)]
pub enum Attack {
    Ranged {
        damage: f32,
        cooldown: f32,
        range: f32,
    },
    Melee {
        damage: f32,
        cooldown: f32,
        range: f32,
    },
}

#[derive(Debug, Clone, Copy)]
pub struct Stats {
    pub health: f32,
    pub speed: f32,
    pub radius: f32,
    pub height: f32,
    pub attack: Attack,
    pub model: &'static str,
    pub name: &'static str,
}

pub fn stats(k: MonsterKind) -> Stats {
    let s = |name, model, health, speed, radius, height, attack| Stats {
        name,
        model,
        health,
        speed,
        radius,
        height,
        attack,
    };
    let r = |damage, cooldown, range| Attack::Ranged {
        damage,
        cooldown,
        range,
    };
    match k {
        MonsterKind::Grunt => s(
            "GRUNT",
            "minion_grunt",
            30.0,
            3.4,
            0.4,
            1.8,
            r(5.0, 1.0, 26.0),
        ),
        MonsterKind::Guard => s(
            "GUARD",
            "minion_guard",
            100.0,
            3.0,
            0.45,
            1.9,
            r(6.0, 0.6, 28.0),
        ),
        MonsterKind::Gunner => s(
            "GUNNER",
            "minion_gunner",
            175.0,
            2.8,
            0.5,
            2.0,
            r(7.0, 0.3, 30.0),
        ),
        MonsterKind::Medic => s(
            "MEDIC",
            "minion_medic",
            300.0,
            3.2,
            0.4,
            1.8,
            r(4.0, 1.0, 22.0),
        ),
        MonsterKind::Enforcer => s(
            "ENFORCER",
            "minion_enforcer",
            240.0,
            4.6,
            0.55,
            2.2,
            Attack::Melee {
                damage: 26.0,
                cooldown: 0.9,
                range: 2.3,
            },
        ),
        MonsterKind::Mech => s("MECH", "mech", 750.0, 2.4, 1.0, 4.2, r(14.0, 0.35, 34.0)),
        MonsterKind::Voss => s(
            "PREFECT VOSS",
            "prefect_voss",
            1500.0,
            2.2,
            0.7,
            2.6,
            r(20.0, 0.45, 36.0),
        ),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Stand,
    Walk,
    Run,
    Attack,
    Pain,
    Dead,
}

#[derive(Debug, Clone)]
pub struct Monster {
    pub kind: MonsterKind,
    pub pos: Vec3,
    pub yaw: f32,
    pub health: f32,
    pub state: State,
    pub t: f32,
    pub cooldown: f32,
    pub alerted: bool,
    pub path: Vec<(i32, i32)>,
    path_t: f32,
    strafe: f32,
    strafe_t: f32,
    /// Medic: dead monster being revived and progress.
    revive: Option<(usize, f32)>,
    pub(crate) wander: f32,
    /// Seconds since death (corpse fade for the renderer).
    pub dead_for: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MEvent {
    /// Fired at the player: damage before armour and whether it hit.
    Fire {
        damage: f32,
        hit: bool,
        melee: bool,
    },
    Revive(usize),
    Alerted,
}

impl Monster {
    pub fn new(kind: MonsterKind, pos: Vec3, yaw: f32) -> Monster {
        Monster {
            kind,
            pos,
            yaw,
            health: stats(kind).health,
            state: State::Stand,
            t: 0.0,
            cooldown: 1.0,
            alerted: false,
            path: Vec::new(),
            path_t: 0.0,
            strafe: 1.0,
            strafe_t: 0.0,
            revive: None,
            wander: 0.0,
            dead_for: 0.0,
        }
    }

    pub fn alive(&self) -> bool {
        self.state != State::Dead
    }

    pub fn forward(&self) -> Vec3 {
        Vec3::new(-self.yaw.sin(), 0.0, -self.yaw.cos())
    }

    /// Centre of the body (for line-of-sight and splash).
    pub fn centre(&self) -> Vec3 {
        self.pos + Vec3::Y * stats(self.kind).height * 0.55
    }

    /// Apply damage; returns true if this killed it.
    pub fn hurt(&mut self, dmg: f32, rng: &mut Rng) -> bool {
        if !self.alive() {
            return false;
        }
        self.health -= dmg;
        self.alerted = true;
        if self.health <= 0.0 {
            self.state = State::Dead;
            self.dead_for = 0.0;
            self.revive = None;
            return true;
        }
        let st = stats(self.kind);
        if dmg >= st.health * 0.08 && rng.f32() < 0.55 && self.kind != MonsterKind::Voss {
            self.state = State::Pain;
            self.t = 0.35;
        }
        false
    }

    fn face(&mut self, dir: Vec3, dt: f32) {
        if dir.length_squared() < 1e-6 {
            return;
        }
        let want = (-dir.x).atan2(-dir.z);
        let mut d = (want - self.yaw + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU)
            - std::f32::consts::PI;
        d = d.clamp(-7.0 * dt, 7.0 * dt);
        self.yaw += d;
    }

    /// Walk toward `target` (flat), opening doors in the way.
    fn walk(&mut self, level: &mut Level, target: Vec3, speed: f32, dt: f32) {
        let mut dir = target - self.pos;
        dir.y = 0.0;
        let dir = dir.normalize_or_zero();
        self.face(dir, dt);
        let r = stats(self.kind).radius;
        // open an unlocked door we are about to touch
        let ahead = self.pos + dir * (r + 1.2);
        let (tx, ty) = level.tile_of(ahead);
        if let Some(d) = level.door_at(tx, ty)
            && level.doors[d].lock.is_none()
        {
            level.doors[d].hold = 2.0;
        }
        slide(level, &mut self.pos, dir * speed * dt, r);
    }

    fn follow_path(&mut self, level: &mut Level, goal: Vec3, speed: f32, dt: f32) {
        self.path_t -= dt;
        if self.path_t <= 0.0 {
            self.path_t = 0.6;
            self.path = level
                .path(level.tile_of(self.pos), level.tile_of(goal))
                .unwrap_or_default();
            if !self.path.is_empty() {
                self.path.remove(0);
            }
        }
        let here = level.tile_of(self.pos);
        let next = self.path.iter().find(|&&t| t != here).copied();
        let target = match next {
            Some((x, y)) => level.centre(x, y),
            None => goal,
        };
        self.walk(level, target, speed, dt);
    }
}

/// Everything a monster may look at while thinking.
pub struct Senses<'a> {
    pub player: &'a Player,
    pub noise: Option<Vec3>,
    pub difficulty: f32,
}

/// Think for one monster. `corpses` are `(index, position, revivable)` of dead monsters.
pub fn think(
    m: &mut Monster,
    level: &mut Level,
    senses: &Senses,
    corpses: &[(usize, Vec3, bool)],
    rng: &mut Rng,
    dt: f32,
) -> Vec<MEvent> {
    let mut ev = Vec::new();
    if m.state == State::Dead {
        m.dead_for += dt;
        return ev;
    }
    let st = stats(m.kind);
    m.cooldown = (m.cooldown - dt).max(0.0);
    m.strafe_t -= dt;
    if m.strafe_t <= 0.0 {
        m.strafe_t = rng.range(0.8, 1.8);
        m.strafe = if rng.f32() < 0.5 { 1.0 } else { -1.0 };
    }
    if m.state == State::Pain {
        m.t -= dt;
        if m.t <= 0.0 {
            m.state = State::Run;
        }
        return ev;
    }
    let p = senses.player;
    let to_player = p.pos - m.pos;
    let dist = to_player.length();
    let eyes_m = m.pos + Vec3::Y * st.height * 0.85;
    let los = level.los(eyes_m, p.eye());
    // --- senses
    if !m.alerted {
        let seen = los && dist < 32.0 && m.forward().dot(to_player.normalize_or_zero()) > 0.1;
        let heard = senses
            .noise
            .is_some_and(|n| n.distance(m.pos) < crate::balance::BOARD_HEARING);
        if seen || heard {
            m.alerted = true;
            m.state = State::Run;
            ev.push(MEvent::Alerted);
        }
    }
    if !m.alerted {
        // idle: stand, occasionally amble
        m.wander -= dt;
        if m.wander <= 0.0 {
            m.wander = rng.range(2.0, 5.0);
            m.state = if m.state == State::Stand {
                State::Walk
            } else {
                State::Stand
            };
            m.yaw += rng.range(-1.5, 1.5);
        }
        if m.state == State::Walk {
            let dir = m.forward();
            let r = st.radius;
            let before = m.pos;
            slide(level, &mut m.pos, dir * st.speed * 0.3 * dt, r);
            if m.pos.distance(before) < 0.001 {
                m.yaw += 1.7;
            }
        }
        return ev;
    }
    // --- medic: go and revive a corpse
    if m.kind == MonsterKind::Medic {
        if let Some((idx, prog)) = m.revive {
            let still = corpses.iter().any(|c| c.0 == idx && c.2);
            if !still {
                m.revive = None;
            } else {
                let np = prog + dt;
                if np >= 3.0 {
                    ev.push(MEvent::Revive(idx));
                    m.revive = None;
                } else {
                    m.revive = Some((idx, np));
                }
                return ev;
            }
        } else if let Some(c) = corpses
            .iter()
            .filter(|c| c.2 && c.1.distance(m.pos) < 14.0)
            .min_by(|a, b| a.1.distance(m.pos).total_cmp(&b.1.distance(m.pos)))
        {
            if c.1.distance(m.pos) < 2.4 {
                m.revive = Some((c.0, 0.0));
                m.state = State::Stand;
            } else {
                let goal = c.1;
                m.state = State::Run;
                m.follow_path(level, goal, st.speed, dt);
            }
            return ev;
        }
    }
    // --- engage
    let (range, melee) = match st.attack {
        Attack::Ranged { range, .. } => (range, false),
        Attack::Melee { range, .. } => (range, true),
    };
    let in_range = dist < range && los;
    let hold = if melee { range * 0.9 } else { range * 0.55 };
    if in_range && (dist <= hold || melee) {
        m.state = State::Attack;
        m.face(to_player, dt);
        if !melee {
            // circle-strafe while shooting
            let side = Vec3::new(-to_player.z, 0.0, to_player.x).normalize_or_zero() * m.strafe;
            let r = st.radius;
            slide(level, &mut m.pos, side * st.speed * 0.4 * dt, r);
        } else if dist > range * 0.8 {
            m.walk(level, p.pos, st.speed, dt);
        }
        if m.cooldown <= 0.0 {
            let aligned = m.forward().dot(to_player.normalize_or_zero()) > 0.9;
            if aligned {
                match st.attack {
                    Attack::Ranged {
                        damage, cooldown, ..
                    } => {
                        let chance = ((0.85 - dist * 0.018)
                            * crate::balance::hit_scale(senses.difficulty))
                        .clamp(0.12, 0.9);
                        let moving_bonus = if p.vel.length() > 3.0 { 0.75 } else { 1.0 };
                        ev.push(MEvent::Fire {
                            damage,
                            hit: rng.f32() < chance * moving_bonus,
                            melee: false,
                        });
                        m.cooldown = cooldown * rng.range(0.9, 1.5);
                    }
                    Attack::Melee {
                        damage, cooldown, ..
                    } => {
                        ev.push(MEvent::Fire {
                            damage,
                            hit: dist < range,
                            melee: true,
                        });
                        m.cooldown = cooldown;
                    }
                }
            }
        }
    } else {
        m.state = State::Run;
        if los && dist > 3.0 {
            m.walk(level, p.pos, st.speed, dt);
        } else {
            m.follow_path(level, p.pos, st.speed, dt);
        }
    }
    ev
}

/// Cell size helper for callers that need tile coordinates of a monster.
pub fn tile(level: &Level, m: &Monster) -> (i32, i32) {
    let _ = CELL;
    level.tile_of(m.pos)
}
