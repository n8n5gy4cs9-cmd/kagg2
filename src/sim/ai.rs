//! Minion (and wingman) ship AI (T09): patrol → intercept → attack → flee / surrender.
//!
//! The AI only emits [`Addressed`] commands — exactly what a player or the network would send.
// ref: REFERENCE_tobe_DELETED snis_npc ai state machine (idea only)

use glam::Vec3;

use super::command::{Addressed, Command};
use super::flight::{CUTTER, DESTROYER, DREADNOUGHT, FORWARD, FRIGATE, INTERCEPTOR};
use super::ops::{Faction, Ops};
use super::world::{Entity, EntityId, Kind, World};

/// Orders the player can give to wingmen (T15).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Order {
    Attack,
    Guard,
    Follow,
    Retreat,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Mode {
    Patrol,
    Intercept,
    Attack,
    Flee,
    Disabled,
    Wingman(Order),
}

#[derive(Debug, Clone, Copy)]
pub struct Ai {
    pub mode: Mode,
    pub home: Vec3,
    pub wp: Vec3,
    /// Skill `0.3..=1.5`: scales accuracy and reaction.
    pub skill: f32,
    pub called: bool,
    pub target: Option<EntityId>,
    strafe: f32,
}

const DETECT: f32 = 7000.0;

impl Ai {
    pub fn new(home: Vec3, skill: f32) -> Ai {
        Ai {
            mode: Mode::Patrol,
            home,
            wp: home,
            skill,
            called: false,
            target: None,
            strafe: 1.0,
        }
    }

    pub fn wingman(skill: f32) -> Ai {
        Ai {
            mode: Mode::Wingman(Order::Follow),
            ..Ai::new(Vec3::ZERO, skill)
        }
    }
}

/// Preferred engagement distance per class.
fn standoff(name: &str) -> f32 {
    match name {
        "Interceptor" => 450.0,
        "Cutter" => 700.0,
        "Frigate" => 1100.0,
        "Destroyer" => 1300.0,
        "Dreadnought" => 1600.0,
        "Skiff" => 500.0,
        _ => 800.0,
    }
}

/// Deterministic pseudo-random `0..1` from a seed (keeps the AI independent of the world RNG).
fn hash01(a: u64, b: u64) -> f32 {
    let mut x = a.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ b.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x ^= x >> 31;
    x = x.wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^= x >> 29;
    (x >> 40) as f32 / (1u64 << 24) as f32
}

/// Turn demands steering the nose toward `dir` (world space).
pub fn steer(e: &Entity, dir: Vec3) -> Command {
    let l = e.rot.inverse() * dir.normalize_or_zero();
    let yaw_err = (-l.x).atan2(-l.z); // positive = turn left
    let pitch_err = l.y.atan2((l.x * l.x + l.z * l.z).sqrt().max(1e-4));
    Command::Turn {
        pitch: (pitch_err * 2.5).clamp(-1.0, 1.0),
        yaw: (yaw_err * 2.5).clamp(-1.0, 1.0),
        roll: 0.0,
    }
}

fn hostile_pool(w: &World, me: &Entity) -> Vec<(EntityId, Vec3, Vec3, f32)> {
    w.iter()
        .filter(|(_, e)| {
            e.kind == Kind::Ship
                && e.faction.hostile_to(me.faction)
                && e.ops.as_ref().is_some_and(|o| !o.dead && !o.disabled)
        })
        .map(|(i, e)| (i, e.pos, e.vel, e.radius))
        .collect()
}

fn minion_count(w: &World) -> usize {
    w.iter()
        .filter(|(_, e)| e.faction == Faction::Minion && e.kind == Kind::Ship)
        .count()
}

/// Spawn a Minion warship with AI at `pos`.
pub fn spawn_minion(
    w: &mut World,
    class: super::flight::ShipClass,
    pos: Vec3,
    mode: Mode,
) -> EntityId {
    let mut e = Entity::new(
        Kind::Ship,
        class.name,
        class.model,
        class.texture,
        class.radius,
    );
    e.pos = pos;
    e.ship = Some(super::flight::ShipState::new(class));
    let mut o = Ops::for_class(&class);
    o.aim_spread = 0.045 / w.difficulty.max(0.3);
    o.lance_freq = w.rng.below(10) as u8;
    o.shield_freq = w.rng.below(10) as u8;
    o.sys.preset(2);
    e.ops = Some(Box::new(o));
    e.faction = Faction::Minion;
    let mut ai = Ai::new(pos, 0.7 * w.difficulty + 0.2);
    ai.mode = mode;
    e.ai = Some(ai);
    w.spawn(e)
}

/// Run every AI ship for one step and return the commands they issue.
pub fn think(w: &mut World) -> Vec<Addressed> {
    let ids: Vec<EntityId> = w
        .iter()
        .filter(|(_, e)| e.ai.is_some() && e.ship.is_some())
        .map(|(i, _)| i)
        .collect();
    let mut out = Vec::new();
    let player = w.player;
    for id in ids {
        let Some(me) = w.get(id) else { continue };
        let (Some(mut ai), Some(ops)) = (me.ai, me.ops.as_deref()) else {
            continue;
        };
        if ops.dead || ops.disabled {
            continue;
        }
        let hull_frac = ops.hull / ops.max_hull;
        let class = ops.class_name;
        let tick = w.tick;
        let seed = id.index as u64 * 7919 + 13;
        let mut push = |cmd: Command| out.push(Addressed { ship: id, cmd });

        let pool = hostile_pool(w, me);
        let nearest = pool
            .iter()
            .filter(|p| p.1.distance(me.pos) < DETECT)
            .min_by(|a, b| {
                let da = a.1.distance(me.pos) - if Some(a.0) == player { 400.0 } else { 0.0 };
                let db = b.1.distance(me.pos) - if Some(b.0) == player { 400.0 } else { 0.0 };
                da.total_cmp(&db)
            })
            .copied();
        let mut call_help = false;

        // --- transitions
        match ai.mode {
            Mode::Wingman(order) => {
                wingman(w, me, ops, &mut ai, order, &pool, player, &mut push);
                if let Some(e) = w.get_mut(id) {
                    e.ai = Some(ai);
                }
                continue;
            }
            Mode::Disabled => continue,
            _ => {}
        }
        if hull_frac < 0.25 && class != "Dreadnought" && ai.mode != Mode::Flee {
            ai.mode = Mode::Flee;
            call_help = !ai.called;
            ai.called = true;
        } else if ai.mode == Mode::Patrol && nearest.is_some() {
            ai.mode = Mode::Intercept;
        } else if ai.mode == Mode::Intercept {
            match nearest {
                None => ai.mode = Mode::Patrol,
                Some(t) if t.1.distance(me.pos) < standoff(class) * 2.6 => ai.mode = Mode::Attack,
                _ => {}
            }
        } else if ai.mode == Mode::Attack && nearest.is_none() {
            ai.mode = Mode::Intercept;
        }
        ai.target = nearest.map(|n| n.0);

        // --- behaviour
        match (ai.mode, nearest) {
            (Mode::Patrol, _) => {
                if me.pos.distance(ai.wp) < 250.0 {
                    let a = hash01(seed, tick / 60) * std::f32::consts::TAU;
                    let r = 800.0 + 1700.0 * hash01(seed + 1, tick / 60);
                    ai.wp = ai.home
                        + Vec3::new(
                            a.cos() * r,
                            (hash01(seed + 2, tick / 60) - 0.5) * 400.0,
                            a.sin() * r,
                        );
                }
                push(steer(me, ai.wp - me.pos));
                push(Command::SetThrottle(0.35));
            }
            (Mode::Intercept, Some(t)) => {
                let lead = t.1 + t.2 * (t.1.distance(me.pos) / 500.0).min(3.0);
                push(steer(me, lead - me.pos));
                push(Command::SetThrottle(1.0));
            }
            (Mode::Attack, Some(t)) => {
                let dist = t.1.distance(me.pos);
                let want = standoff(class);
                let lead = t.1 + t.2 * (dist / 900.0).min(1.5);
                let to = lead - me.pos;
                push(steer(me, to));
                let thr = ((dist - want) / want).clamp(-0.2, 1.0);
                push(Command::SetThrottle(if dist < want * 0.6 {
                    0.15
                } else {
                    thr.max(0.25)
                }));
                // circle-strafe, flipping direction every few seconds
                if tick.is_multiple_of(240) {
                    ai.strafe = if hash01(seed, tick) > 0.5 { 1.0 } else { -1.0 };
                }
                push(Command::Strafe {
                    x: ai.strafe * 0.8,
                    y: 0.0,
                });
                // fire when the nose is on target
                let align = me.forward().angle_between(to);
                let react = (0.9 / ai.skill.max(0.3)).min(2.5);
                let gate = hash01(seed + 3, tick / 12) < 0.35 + 0.4 * ai.skill.min(1.5);
                if dist < super::combat::LANCE_RANGE * 0.9 && align < 0.07 + 0.05 * ai.skill && gate
                {
                    push(Command::FireLance);
                }
                let big = t.3 > 15.0;
                if align < 0.12
                    && (600.0..2200.0).contains(&dist)
                    && ops.torpedoes > 0
                    && (big || class == "Dreadnought")
                    && hash01(seed + 4, tick / 30) < 0.04 / react
                {
                    push(Command::SetTarget(Some(t.0)));
                    push(Command::FireTorpedo);
                }
                if ops.missiles > 0
                    && dist < 5000.0
                    && dist > 700.0
                    && hash01(seed + 5, tick / 30) < 0.02 / react
                {
                    push(Command::SetTarget(Some(t.0)));
                    push(Command::FireMissile);
                }
            }
            (Mode::Flee, _) => {
                let away = nearest.map_or(Vec3::Z, |t| me.pos - t.1);
                push(steer(me, away));
                push(Command::SetThrottle(1.0));
            }
            _ => {}
        }
        if let Some(e) = w.get_mut(id) {
            e.ai = Some(ai);
        }
        if call_help {
            reinforce(w, id);
        }
    }
    out
}

#[allow(clippy::too_many_arguments)]
fn wingman(
    w: &World,
    me: &Entity,
    ops: &Ops,
    ai: &mut Ai,
    order: Order,
    pool: &[(EntityId, Vec3, Vec3, f32)],
    player: Option<EntityId>,
    push: &mut impl FnMut(Command),
) {
    let Some(pe) = player.and_then(|p| w.get(p)) else {
        return;
    };
    let player_target = pe.ops.as_deref().and_then(|o| o.target);
    let foe = match order {
        Order::Attack => player_target
            .and_then(|t| w.get(t))
            .map(|e| (e.pos, e.vel, e.radius)),
        Order::Guard => pool
            .iter()
            .filter(|p| p.1.distance(pe.pos) < 3500.0)
            .min_by(|a, b| a.1.distance(me.pos).total_cmp(&b.1.distance(me.pos)))
            .map(|p| (p.1, p.2, p.3)),
        _ => None,
    };
    match (order, foe) {
        (Order::Attack | Order::Guard, Some((pos, vel, _))) => {
            let dist = pos.distance(me.pos);
            let lead = pos + vel * (dist / 900.0).min(1.5);
            let to = lead - me.pos;
            push(steer(me, to));
            push(Command::SetThrottle(if dist < 400.0 { 0.3 } else { 1.0 }));
            push(Command::Strafe {
                x: 0.5 * ai.strafe,
                y: 0.0,
            });
            if me.forward().angle_between(to) < 0.1 && dist < 2400.0 {
                push(Command::FireLance);
            }
        }
        (Order::Retreat, _) => {
            let away = pool
                .iter()
                .min_by(|a, b| a.1.distance(me.pos).total_cmp(&b.1.distance(me.pos)))
                .map_or(me.pos - pe.pos, |p| me.pos - p.1);
            push(steer(me, away));
            push(Command::SetThrottle(1.0));
        }
        _ => {
            // Follow: hold station behind-right of the player.
            let slot = pe.pos + pe.rot * Vec3::new(160.0, 20.0, 260.0);
            let to = slot - me.pos;
            let d = to.length();
            if d > 120.0 {
                push(steer(me, to));
                push(Command::SetThrottle((d / 800.0).clamp(0.2, 1.0)));
            } else {
                push(steer(me, pe.rot * FORWARD));
                push(Command::SetThrottle(
                    (pe.vel.length() / 320.0).clamp(0.0, 1.0),
                ));
            }
        }
    }
    let _ = ops;
}

/// A fleeing ship calls up to two interceptors (once per world twice).
fn reinforce(w: &mut World, caller: EntityId) {
    if w.reinforcements_left == 0 || minion_count(w) >= 10 {
        return;
    }
    w.reinforcements_left -= 1;
    let Some(p) = w.player.and_then(|p| w.get(p)).map(|e| e.pos) else {
        return;
    };
    let dir = w.rng.unit_vec();
    for k in 0..2 {
        let pos = p + dir * 4200.0 + Vec3::new(k as f32 * 90.0, 0.0, 0.0);
        spawn_minion(w, INTERCEPTOR, pos, Mode::Intercept);
    }
    let label = w.get(caller).map_or("MINION", |e| e.label);
    w.push_event(super::event::Event::Message(format!(
        "{} CALLED FOR HELP - MINION REINFORCEMENTS INBOUND",
        label.to_uppercase()
    )));
}

/// Minion ship class by lowercase name (unknown names give a dreadnought).
pub fn class_of(name: &str) -> super::flight::ShipClass {
    match name {
        "interceptor" => INTERCEPTOR,
        "cutter" => CUTTER,
        "frigate" => FRIGATE,
        "destroyer" => DESTROYER,
        _ => DREADNOUGHT,
    }
}

/// Spawn a standard encounter group (`n` ships mixing classes) around `center`.
pub fn spawn_squad(w: &mut World, center: Vec3, classes: &[&str]) {
    for (i, c) in classes.iter().enumerate() {
        let class = class_of(c);
        let a = i as f32 * 2.1;
        let pos = center + Vec3::new(a.cos() * 220.0, (i as f32 - 1.0) * 40.0, a.sin() * 220.0);
        spawn_minion(w, class, pos, Mode::Patrol);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::combat::tests::spawn_ship;
    use crate::sim::flight::LONGSHIP;
    use crate::sim::{Addressed, Event};

    fn arena(difficulty: f32) -> (World, EntityId) {
        let mut w = World::new(9);
        w.difficulty = difficulty;
        let p = spawn_ship(&mut w, LONGSHIP, Vec3::ZERO, Faction::Mars);
        w.player = Some(p);
        (w, p)
    }

    #[test]
    fn patrol_spots_the_player_and_closes_in() {
        let (mut w, p) = arena(1.0);
        let m = spawn_minion(&mut w, CUTTER, Vec3::new(0.0, 0.0, -4500.0), Mode::Patrol);
        let d0 = w.get(m).map(|e| e.pos.distance(Vec3::ZERO)).unwrap_or(0.0);
        for _ in 0..600 {
            w.step(&[]);
        }
        let e = w.get(m).expect("alive");
        assert!(e.pos.distance(w.get(p).map_or(Vec3::ZERO, |x| x.pos)) < d0 * 0.7);
        assert!(matches!(
            e.ai.map(|a| a.mode),
            Some(Mode::Intercept | Mode::Attack)
        ));
    }

    #[test]
    fn attackers_actually_hurt_a_passive_player() {
        let (mut w, p) = arena(1.0);
        spawn_minion(&mut w, CUTTER, Vec3::new(0.0, 0.0, -1500.0), Mode::Attack);
        let (mut fired, mut hurt) = (false, false);
        for _ in 0..60 * 40 {
            for e in w.step(&[]) {
                fired |= matches!(e, Event::LanceFired { .. });
                hurt |= matches!(e, Event::ShieldHit { ship, .. } if ship == p);
            }
        }
        assert!(fired && hurt, "fired {fired} hurt {hurt}");
    }

    #[test]
    fn wounded_ship_flees_and_calls_reinforcements_once() {
        let (mut w, _) = arena(1.0);
        let m = spawn_minion(&mut w, CUTTER, Vec3::new(0.0, 0.0, -1500.0), Mode::Attack);
        if let Some(o) = w.get_mut(m).and_then(|e| e.ops.as_deref_mut()) {
            o.hull = 20.0;
        }
        let before = minion_count(&w);
        let mut msg = 0;
        for _ in 0..30 {
            msg += w
                .step(&[])
                .iter()
                .filter(|e| matches!(e, Event::Message(s) if s.contains("HELP")))
                .count();
        }
        assert_eq!(
            w.get(m).and_then(|e| e.ai).map(|a| a.mode),
            Some(Mode::Flee)
        );
        assert_eq!(msg, 1);
        assert_eq!(minion_count(&w), before + 2);
    }

    #[test]
    fn wingman_obeys_orders() {
        let (mut w, p) = arena(1.0);
        let wing = spawn_ship(
            &mut w,
            crate::sim::flight::SKIFF,
            Vec3::new(600.0, 0.0, 300.0),
            Faction::Mars,
        );
        if let Some(e) = w.get_mut(wing) {
            e.ai = Some(Ai::wingman(1.0));
        }
        let foe = spawn_minion(
            &mut w,
            INTERCEPTOR,
            Vec3::new(0.0, 0.0, -2500.0),
            Mode::Disabled,
        );
        w.step(&[Addressed {
            ship: p,
            cmd: Command::SetTarget(Some(foe)),
        }]);
        let d0 = w.get(wing).map_or(0.0, |e| {
            e.pos.distance(w.get(foe).map_or(Vec3::ZERO, |f| f.pos))
        });
        super::super::nav::handle(&mut w, p, Command::Order(Order::Attack));
        for _ in 0..600 {
            w.step(&[]);
        }
        let d1 = w.get(wing).map_or(0.0, |e| {
            e.pos.distance(w.get(foe).map_or(Vec3::ZERO, |f| f.pos))
        });
        assert!(
            d1 < d0,
            "wingman should advance on the target ({d0} -> {d1})"
        );
    }
}
