//! Weapons, projectiles, shields, hull and subsystem damage, destruction (T07 + T08).
//!
//! Shield frequency mechanic: every shield has a frequency `0..=9`; a lance tuned within one step
//! of it does double shield damage, one tuned four or more steps away only half.

use glam::{Quat, Vec3};

use super::clock::DT;
use super::command::Command;
use super::event::{Event, Ordnance};
use super::flight::FORWARD;
use super::ops::{AFT, FORE, Ops, PORT, STARBOARD};
use super::systems::{SYS_COUNT, Sys};
use super::world::{Entity, EntityId, Kind, Proj, ProjKind, World};

pub const LANCE_RANGE: f32 = 2800.0;
const LANCE_DAMAGE: f32 = 14.0;
const LANCE_COOLDOWN: f32 = 0.32;
const LANCE_HEAT: f32 = 3.5;
const TORPEDO_SPEED: f32 = 260.0;
const TORPEDO_DAMAGE: f32 = 140.0;
const TORPEDO_SPLASH: f32 = 90.0;
const MISSILE_SPEED: f32 = 480.0;
const MISSILE_DAMAGE: f32 = 55.0;
const MISSILE_TURN: f32 = 2.4;
const MISSILE_RANGE: f32 = 9000.0;

fn ops_mut(w: &mut World, id: EntityId) -> Option<&mut Ops> {
    w.get_mut(id).and_then(|e| e.ops.as_deref_mut())
}

/// Damage multiplier of a lance tuned to `lance` against shields tuned to `shield`.
pub fn freq_mult(lance: u8, shield: u8) -> f32 {
    match lance.abs_diff(shield) {
        0 | 1 => 2.0,
        2 | 3 => 1.0,
        _ => 0.5,
    }
}

/// Shield facing hit by an attacker at `local` (attacker position in the victim's body frame).
pub fn facing_of(local: Vec3) -> usize {
    if local.z.abs() >= local.x.abs() {
        if local.z < 0.0 { FORE } else { AFT }
    } else if local.x > 0.0 {
        STARBOARD
    } else {
        PORT
    }
}

fn ray_sphere(from: Vec3, dir: Vec3, c: Vec3, r: f32) -> Option<f32> {
    let oc = c - from;
    let t = oc.dot(dir);
    let d2 = oc.length_squared() - t * t;
    if d2 > r * r {
        return None;
    }
    let t0 = t - (r * r - d2).sqrt();
    if t + (r * r - d2).sqrt() < 0.0 {
        return None;
    }
    Some(t0.max(0.0))
}

/// Closest solid struck by a ray (ships, stations, rocks, planets), excluding `skip`.
pub fn ray_cast(
    w: &World,
    from: Vec3,
    dir: Vec3,
    range: f32,
    skip: Option<EntityId>,
) -> Option<(EntityId, f32)> {
    let mut best: Option<(EntityId, f32)> = None;
    for (id, e) in w.iter() {
        if Some(id) == skip || matches!(e.kind, Kind::Projectile | Kind::Debris) {
            continue;
        }
        if let Some(t) = ray_sphere(from, dir, e.pos, e.radius)
            && t <= range
            && best.is_none_or(|b| t < b.1)
        {
            best = Some((id, t));
        }
    }
    best
}

/// Apply a game command that concerns weapons or ship state.
pub fn handle(w: &mut World, ship: EntityId, cmd: Command) {
    let alive = w
        .get(ship)
        .and_then(|e| e.ops.as_deref())
        .is_some_and(|o| !o.dead);
    if !alive {
        return;
    }
    match cmd {
        Command::FireLance => fire_lance(w, ship),
        Command::FireTorpedo => launch(w, ship, ProjKind::Torpedo),
        Command::FireMissile => launch(w, ship, ProjKind::Missile),
        Command::SetLanceFreq(f) => {
            if let Some(o) = ops_mut(w, ship) {
                o.lance_freq = f.min(9);
            }
        }
        Command::SetShieldFreq(f) => {
            if let Some(o) = ops_mut(w, ship) {
                o.shield_freq = f.min(9);
            }
        }
        Command::SetTarget(t) => {
            if let Some(o) = ops_mut(w, ship) {
                o.target = t;
            }
        }
        Command::SetPower(s, v) => {
            if let Some(o) = ops_mut(w, ship) {
                o.sys.set_power(s, v);
            }
        }
        Command::SetCoolant(s, v) => {
            if let Some(o) = ops_mut(w, ship) {
                o.sys.set_coolant(s, v);
            }
        }
        Command::PowerPreset(n) => {
            if let Some(o) = ops_mut(w, ship) {
                o.sys.preset(n);
            }
        }
        Command::AssignRepair { team, sys } => {
            if let Some(o) = ops_mut(w, ship)
                && let Some(t) = o.sys.teams.get_mut(team as usize)
            {
                *t = sys;
            }
        }
        _ => super::nav::handle(w, ship, cmd),
    }
}

fn fire_lance(w: &mut World, id: EntityId) {
    let Some(e) = w.get(id) else { return };
    let Some(o) = e.ops.as_deref() else { return };
    let eff = o.sys.eff(Sys::Lances);
    if o.disabled
        || o.docked.is_some()
        || o.lance_cd > 0.0
        || eff < 0.1
        || o.sys.heat[Sys::Lances.idx()] >= 95.0
    {
        return;
    }
    let (spread, freq, mult) = (o.aim_spread, o.lance_freq, o.lance_mult());
    let (pos, radius, fwd) = (e.pos, e.radius, e.forward());
    let dir = if spread > 0.0 {
        (fwd + w.rng.unit_vec() * spread).normalize_or_zero()
    } else {
        fwd
    };
    let from = pos + dir * radius * 0.9;
    let hit = ray_cast(w, from, dir, LANCE_RANGE, Some(id));
    let to = from + dir * hit.map_or(LANCE_RANGE, |h| h.1);
    if let Some(o) = ops_mut(w, id) {
        o.lance_cd = LANCE_COOLDOWN / (0.7 + 0.3 * eff.min(1.5));
        o.sys.add_heat(Sys::Lances, LANCE_HEAT);
    }
    w.push_event(Event::LanceFired {
        shooter: id,
        from,
        to,
        hit: hit.map(|h| h.0),
    });
    if let Some((victim, _)) = hit {
        let dmg = LANCE_DAMAGE * mult * eff.clamp(0.25, 1.6);
        apply_damage(w, victim, from, dmg, Some(freq), Some(id));
        w.push_event(Event::Explosion { pos: to, size: 6.0 });
    }
}

fn launch(w: &mut World, id: EntityId, kind: ProjKind) {
    let Some(e) = w.get(id) else { return };
    let Some(o) = e.ops.as_deref() else { return };
    if o.disabled || o.docked.is_some() {
        return;
    }
    let (pos, radius, fwd, vel, rot, faction) =
        (e.pos, e.radius, e.forward(), e.vel, e.rot, e.faction);
    let target = o
        .target
        .filter(|t| w.get(*t).is_some_and(|x| x.ops.is_some()));
    let freq = o.lance_freq;
    let (model, tex, speed, damage, ttl) = match kind {
        ProjKind::Torpedo => {
            if o.torpedoes == 0 || o.torp_cd > 0.0 {
                return;
            }
            ("torpedo", "torpedo", TORPEDO_SPEED, TORPEDO_DAMAGE, 14.0)
        }
        ProjKind::Missile => {
            if o.missiles == 0 || o.miss_cd > 0.0 {
                return;
            }
            let Some(t) = target else { return };
            let Some(te) = w.get(t) else { return };
            if te.pos.distance(pos) > MISSILE_RANGE {
                return;
            }
            (
                "missile",
                "missile_trail",
                MISSILE_SPEED,
                MISSILE_DAMAGE,
                12.0,
            )
        }
        ProjKind::Debris => return,
    };
    if let Some(o) = ops_mut(w, id) {
        match kind {
            ProjKind::Torpedo => {
                o.torpedoes -= 1;
                o.torp_cd = 2.5;
            }
            _ => {
                o.missiles -= 1;
                o.miss_cd = 1.2;
            }
        }
    }
    let mut p = Entity::new(Kind::Projectile, "Ordnance", model, tex, 5.0);
    p.pos = pos + fwd * (radius + 8.0);
    p.vel = vel + fwd * speed;
    p.rot = rot;
    p.faction = faction;
    p.proj = Some(Proj {
        kind,
        owner: id,
        ttl,
        damage,
        target: if kind == ProjKind::Missile {
            target
        } else {
            None
        },
        freq,
    });
    w.spawn(p);
    w.push_event(Event::Launched {
        shooter: id,
        kind: if kind == ProjKind::Torpedo {
            Ordnance::Torpedo
        } else {
            Ordnance::Missile
        },
    });
}

/// Missile homing: turn the velocity vector toward the target's predicted position.
pub fn steer_projectiles(w: &mut World) {
    let ids: Vec<EntityId> = w
        .iter()
        .filter(|(_, e)| e.proj.is_some_and(|p| p.kind == ProjKind::Missile))
        .map(|(i, _)| i)
        .collect();
    for id in ids {
        let Some(e) = w.get(id) else { continue };
        let Some(p) = e.proj else { continue };
        let Some(t) = p.target.and_then(|t| w.get(t)) else {
            continue;
        };
        let to = t.pos + t.vel * 0.5 - e.pos;
        let want = to.normalize_or_zero();
        let speed = e.vel.length().max(1.0);
        let cur = e.vel / speed;
        let ang = cur.angle_between(want);
        let step = (MISSILE_TURN * DT).min(ang);
        let new_dir = if ang > 1e-4 {
            let axis = cur.cross(want).normalize_or_zero();
            Quat::from_axis_angle(axis, step) * cur
        } else {
            cur
        };
        let new_speed = (speed + 220.0 * DT).min(MISSILE_SPEED * 1.4);
        if let Some(e) = w.get_mut(id) {
            e.vel = new_dir * new_speed;
        }
    }
}

/// Move-through collision for ordnance, ttl for debris/ordnance.
pub fn collide_projectiles(w: &mut World) {
    let ids: Vec<EntityId> = w
        .iter()
        .filter(|(_, e)| e.proj.is_some())
        .map(|(i, _)| i)
        .collect();
    for id in ids {
        let Some(e) = w.get_mut(id) else { continue };
        let Some(mut p) = e.proj else { continue };
        p.ttl -= DT;
        e.proj = Some(p);
        if p.kind == ProjKind::Debris {
            if p.ttl <= 0.0 {
                w.despawn(id);
            }
            continue;
        }
        let dir = (e.pos - e.prev_pos).normalize_or_zero();
        let len = e.pos.distance(e.prev_pos);
        if dir != Vec3::ZERO {
            e.rot = Quat::from_rotation_arc(FORWARD, dir);
        }
        let (a, pos) = (e.prev_pos, e.pos);
        let hit = ray_cast(w, a, dir, len + 4.0, Some(p.owner));
        if let Some((victim, t)) = hit {
            let at = a + dir * t;
            detonate(w, id, p, victim, at);
        } else if p.ttl <= 0.0 {
            w.push_event(Event::Explosion { pos, size: 14.0 });
            w.despawn(id);
        }
    }
}

fn detonate(w: &mut World, id: EntityId, p: Proj, victim: EntityId, at: Vec3) {
    let big = p.kind == ProjKind::Torpedo;
    w.push_event(Event::Explosion {
        pos: at,
        size: if big { 70.0 } else { 30.0 },
    });
    apply_damage(w, victim, at, p.damage, None, Some(p.owner));
    if big {
        let near: Vec<(EntityId, f32)> = w
            .iter()
            .filter(|(i, e)| *i != victim && *i != p.owner && e.ops.is_some())
            .map(|(i, e)| (i, e.pos.distance(at) - e.radius))
            .filter(|(_, d)| *d < TORPEDO_SPLASH)
            .collect();
        for (i, d) in near {
            let f = 1.0 - d.max(0.0) / TORPEDO_SPLASH;
            apply_damage(w, i, at, p.damage * 0.5 * f, None, Some(p.owner));
        }
    }
    w.despawn(id);
}

/// Hit `victim` for `amount` from direction of `from_pos`; handles facing, frequency, hull,
/// subsystems, surrender and destruction.
pub fn apply_damage(
    w: &mut World,
    victim: EntityId,
    from_pos: Vec3,
    amount: f32,
    freq: Option<u8>,
    by: Option<EntityId>,
) {
    let Some(e) = w.get(victim) else { return };
    if e.ops.is_none() {
        return;
    }
    let local = e.rot.inverse() * (from_pos - e.pos);
    let world_dir = (from_pos - e.pos).normalize_or_zero();
    let facing = facing_of(local);
    let roll = w.rng.f32();
    let sys_pick = w.rng.below(SYS_COUNT as u32) as usize;
    let Some(o) = ops_mut(w, victim) else { return };
    if o.dead {
        return;
    }
    let m = freq.map_or(1.0, |f| freq_mult(f, o.shield_freq));
    let sf = o.sys.eff(Sys::Shields);
    let sh = o.shields[facing];
    let mut overflow = amount;
    let mut shield_hit = false;
    if sf >= 0.05 && sh > 0.0 {
        shield_hit = true;
        let need = amount * m;
        if sh >= need {
            o.shields[facing] = sh - need;
            overflow = 0.0;
        } else {
            overflow = (need - sh) / m;
            o.shields[facing] = 0.0;
        }
    }
    o.shield_idle = 0.0;
    let mut events = Vec::new();
    if shield_hit {
        events.push(Event::ShieldHit {
            ship: victim,
            facing,
            dir: world_dir,
        });
    }
    let mut destroyed = false;
    if overflow > 0.0 {
        o.hull -= overflow;
        events.push(Event::HullHit {
            ship: victim,
            amount: overflow,
        });
        if roll < 0.3 + overflow / o.max_hull * 3.0 {
            let sys = Sys::ALL[sys_pick];
            o.sys.damage(sys, overflow * 0.6);
            events.push(Event::SystemDamaged { ship: victim, sys });
        }
        if o.hull <= 0.0 {
            o.hull = 0.0;
            destroyed = true;
        } else if o.hull < o.max_hull * 0.12 && !o.disabled && w_is_ai(w, victim) {
            events.push(Event::Surrendered(victim));
        }
    }
    for ev in events {
        if let Event::Surrendered(id) = ev {
            if let Some(o) = ops_mut(w, id) {
                o.disabled = true;
                o.nav.autopilot = false;
            }
            if let Some(e) = w.get_mut(id) {
                e.ai = e.ai.map(|mut a| {
                    a.mode = super::ai::Mode::Disabled;
                    a
                });
            }
            let label = w.get(id).map_or("", |e| e.label);
            w.push_event(Event::Message(format!("{label} IS DISABLED - BOARDABLE")));
        }
        w.push_event(ev);
    }
    if destroyed {
        destroy(w, victim, by);
    }
}

fn w_is_ai(w: &World, id: EntityId) -> bool {
    w.get(id).is_some_and(|e| e.ai.is_some())
}

/// Blow a ship up: debris, explosion event; the player's entity stays (flagged dead).
pub fn destroy(w: &mut World, id: EntityId, by: Option<EntityId>) {
    let Some(e) = w.get(id) else { return };
    let (pos, vel, radius, label) = (e.pos, e.vel, e.radius, e.label);
    w.push_event(Event::Destroyed {
        id,
        pos,
        radius,
        label,
        by,
    });
    w.push_event(Event::Explosion {
        pos,
        size: radius * 4.0 + 40.0,
    });
    // Bounty for kills by the player's side against hostiles.
    let hostile = w
        .get(id)
        .zip(by.and_then(|b| w.get(b)))
        .is_some_and(|(v, k)| v.faction.hostile_to(k.faction));
    if hostile && by == w.player {
        let bounty = w
            .get(id)
            .and_then(|e| e.ops.as_deref())
            .map_or(0, |o| (o.max_hull * 0.6) as i32);
        if let Some(o) = by.and_then(|b| ops_mut(w, b)) {
            o.cargo.credits += bounty;
        }
        w.push_event(Event::Message(format!("BOUNTY: +{bounty} CR")));
    }
    let n = (6.0 + radius / 8.0).min(16.0) as u32;
    for _ in 0..n {
        let dir = w.rng.unit_vec();
        let r = w.rng.range(2.0, 7.0);
        let spin = w.rng.range(-2.0, 2.0);
        let speed = w.rng.range(15.0, 90.0);
        let mut d = Entity::new(Kind::Debris, "Debris", "asteroid_c", "asteroid_c", r);
        d.pos = pos + dir * radius * 0.5;
        d.vel = vel * 0.5 + dir * speed;
        d.spin = spin;
        d.proj = Some(Proj {
            kind: ProjKind::Debris,
            owner: id,
            ttl: 25.0,
            damage: 0.0,
            target: None,
            freq: 0,
        });
        w.spawn(d);
    }
    if w.player == Some(id) {
        if let Some(e) = w.get_mut(id) {
            if let Some(o) = e.ops.as_deref_mut() {
                o.dead = true;
            }
            if let Some(s) = e.ship.as_mut() {
                s.throttle = 0.0;
            }
        }
    } else {
        w.despawn(id);
    }
}

/// Per-step upkeep for every ship: cooldowns, heat/repairs, shield regen, engine performance.
pub fn upkeep(w: &mut World) {
    let ids: Vec<EntityId> = w
        .iter()
        .filter(|(_, e)| e.ops.is_some())
        .map(|(i, _)| i)
        .collect();
    for id in ids {
        let Some(e) = w.get_mut(id) else { continue };
        let Some(o) = e.ops.as_deref_mut() else {
            continue;
        };
        o.lance_cd = (o.lance_cd - DT).max(0.0);
        o.torp_cd = (o.torp_cd - DT).max(0.0);
        o.miss_cd = (o.miss_cd - DT).max(0.0);
        o.shield_idle += DT;
        o.sys.step();
        let sf = o.sys.eff(Sys::Shields);
        let docked = o.docked.is_some();
        if sf >= 0.05 && (o.shield_idle > 3.0 || docked) {
            let rate = o.max_shield * if docked { 0.5 } else { 0.05 } * sf.min(1.5) * DT;
            for s in &mut o.shields {
                *s = (*s + rate).min(o.max_shield);
            }
        } else if sf < 0.05 {
            // Collapsed generator: shields bleed away.
            for s in &mut o.shields {
                *s = (*s - o.max_shield * 0.1 * DT).max(0.0);
            }
        }
        let still = o.disabled || o.dead || docked;
        let perf = if still { 0.0 } else { o.sys.eff(Sys::Engines) };
        if let Some(s) = e.ship.as_mut() {
            s.perf = perf;
            if still {
                s.throttle = 0.0;
                s.strafe = (0.0, 0.0);
                s.turn = Vec3::ZERO;
            }
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::sim::flight::{CUTTER, INTERCEPTOR, LONGSHIP};
    use crate::sim::ops::Faction;
    use crate::sim::world::Entity;
    use crate::sim::{Addressed, Command, DT};

    pub(crate) fn spawn_ship(
        w: &mut World,
        class: crate::sim::flight::ShipClass,
        pos: Vec3,
        f: Faction,
    ) -> EntityId {
        let mut e = Entity::new(
            Kind::Ship,
            class.name,
            class.model,
            class.texture,
            class.radius,
        );
        e.pos = pos;
        e.ship = Some(crate::sim::flight::ShipState::new(class));
        e.ops = Some(Box::new(Ops::for_class(&class)));
        e.faction = f;
        w.spawn(e)
    }

    fn duel() -> (World, EntityId, EntityId) {
        let mut w = World::new(5);
        let a = spawn_ship(&mut w, LONGSHIP, Vec3::ZERO, Faction::Mars);
        let b = spawn_ship(&mut w, CUTTER, Vec3::new(0.0, 0.0, -800.0), Faction::Minion);
        w.player = Some(a);
        (w, a, b)
    }

    fn fire(w: &mut World, a: EntityId, cmd: Command) -> Vec<Event> {
        w.step(&[Addressed { ship: a, cmd }])
    }

    #[test]
    fn frequency_multipliers() {
        assert_eq!(freq_mult(3, 3), 2.0);
        assert_eq!(freq_mult(3, 4), 2.0);
        assert_eq!(freq_mult(3, 5), 1.0);
        assert_eq!(freq_mult(0, 4), 0.5);
        assert_eq!(freq_mult(9, 0), 0.5);
    }

    #[test]
    fn facing_selection() {
        assert_eq!(facing_of(Vec3::new(0.0, 0.0, -5.0)), FORE);
        assert_eq!(facing_of(Vec3::new(0.0, 0.0, 5.0)), AFT);
        assert_eq!(facing_of(Vec3::new(-5.0, 0.0, 1.0)), PORT);
        assert_eq!(facing_of(Vec3::new(5.0, 0.0, 1.0)), STARBOARD);
    }

    #[test]
    fn lance_hits_shields_first_then_hull() {
        let (mut w, a, b) = duel();
        let ev = fire(&mut w, a, Command::FireLance);
        assert!(
            ev.iter()
                .any(|e| matches!(e, Event::LanceFired { hit: Some(h), .. } if *h == b))
        );
        let o = w
            .get(b)
            .and_then(|e| e.ops.as_deref())
            .map(|o| (o.hull, o.shields));
        let (hull, sh) = o.expect("ops");
        assert_eq!(hull, 120.0, "shields should absorb the first hit");
        // The bow is struck (attacker is ahead of the cutter's nose? cutter faces -Z away from us).
        assert!(sh.iter().any(|&s| s < 70.0));
        // Burn the shields down: eventually hull takes damage.
        for _ in 0..400 {
            fire(&mut w, a, Command::FireLance);
        }
        let hull = w.get(b).and_then(|e| e.ops.as_deref()).map(|o| o.hull);
        assert!(hull.is_none_or(|h| h < 120.0));
    }

    #[test]
    fn matching_frequency_does_double_shield_damage() {
        let dmg = |lance: u8| {
            let (mut w, a, b) = duel();
            if let Some(o) = w.get_mut(b).and_then(|e| e.ops.as_deref_mut()) {
                o.shield_freq = 5;
            }
            if let Some(o) = w.get_mut(a).and_then(|e| e.ops.as_deref_mut()) {
                o.lance_freq = lance;
            }
            fire(&mut w, a, Command::FireLance);
            let o = w.get(b).and_then(|e| e.ops.as_deref()).expect("ops");
            70.0 * 4.0 - o.shield_total()
        };
        let (matched, off) = (dmg(5), dmg(0));
        assert!((matched / off - 4.0).abs() < 0.05, "{matched} vs {off}");
    }

    #[test]
    fn torpedo_flies_hits_and_uses_ammo() {
        let (mut w, a, b) = duel();
        let n0 = w.get(a).and_then(|e| e.ops.as_deref()).map(|o| o.torpedoes);
        let mut ev = fire(&mut w, a, Command::FireTorpedo);
        for _ in 0..400 {
            ev.extend(w.step(&[]));
        }
        assert_eq!(
            w.get(a).and_then(|e| e.ops.as_deref()).map(|o| o.torpedoes),
            n0.map(|n| n - 1)
        );
        assert!(
            ev.iter()
                .any(|e| matches!(e, Event::Explosion { size, .. } if *size > 50.0))
        );
        assert!(
            w.get(b)
                .and_then(|e| e.ops.as_deref())
                .is_some_and(|o| o.shield_total() < 280.0)
        );
        assert!(w.iter().all(|(_, e)| e.kind != Kind::Projectile));
    }

    #[test]
    fn missile_needs_lock_and_homes() {
        let (mut w, a, b) = duel();
        fire(&mut w, a, Command::FireMissile);
        assert!(
            w.iter().all(|(_, e)| e.kind != Kind::Projectile),
            "no lock, no launch"
        );
        // Target is off to the side; a locked missile must still find it.
        if let Some(e) = w.get_mut(b) {
            e.pos = Vec3::new(500.0, 100.0, -500.0);
        }
        w.step(&[Addressed {
            ship: a,
            cmd: Command::SetTarget(Some(b)),
        }]);
        fire(&mut w, a, Command::FireMissile);
        let mut hit = false;
        for _ in 0..600 {
            hit |= w
                .step(&[])
                .iter()
                .any(|e| matches!(e, Event::ShieldHit { ship, .. } if *ship == b));
        }
        assert!(hit, "missile should curve into the target");
    }

    #[test]
    fn ships_die_leave_debris_and_player_survives_as_dead() {
        let (mut w, a, b) = duel();
        if let Some(o) = ops_mut(&mut w, b) {
            o.hull = 1.0;
            o.shields = [0.0; 4];
        }
        let ev = fire(&mut w, a, Command::FireLance);
        assert!(
            ev.iter()
                .any(|e| matches!(e, Event::Destroyed { id, .. } if *id == b))
        );
        assert!(w.get(b).is_none());
        assert!(w.iter().any(|(_, e)| e.kind == Kind::Debris));
        destroy(&mut w, a, None);
        assert!(
            w.get(a)
                .and_then(|e| e.ops.as_deref())
                .is_some_and(|o| o.dead)
        );
    }

    #[test]
    fn killing_a_hostile_pays_a_bounty() {
        let (mut w, a, b) = duel();
        if let Some(o) = ops_mut(&mut w, b) {
            o.hull = 1.0;
            o.shields = [0.0; 4];
        }
        let c0 = w
            .get(a)
            .and_then(|e| e.ops.as_deref())
            .map_or(0, |o| o.cargo.credits);
        fire(&mut w, a, Command::FireLance);
        let c1 = w
            .get(a)
            .and_then(|e| e.ops.as_deref())
            .map_or(0, |o| o.cargo.credits);
        assert_eq!(c1 - c0, 72);
    }

    #[test]
    fn crippled_enemy_surrenders() {
        let mut w = World::new(1);
        let a = spawn_ship(&mut w, LONGSHIP, Vec3::ZERO, Faction::Mars);
        let b = spawn_ship(
            &mut w,
            INTERCEPTOR,
            Vec3::new(0.0, 0.0, -300.0),
            Faction::Minion,
        );
        if let Some(e) = w.get_mut(b) {
            e.ai = Some(crate::sim::ai::Ai::new(Vec3::ZERO, 1.0));
        }
        if let Some(o) = ops_mut(&mut w, b) {
            o.hull = 20.0;
            o.shields = [0.0; 4];
        }
        let ev = fire(&mut w, a, Command::FireLance);
        assert!(ev.iter().any(|e| matches!(e, Event::Surrendered(_))));
        assert!(
            w.get(b)
                .and_then(|e| e.ops.as_deref())
                .is_some_and(|o| o.disabled)
        );
    }

    #[test]
    fn lances_need_power_and_cool_down() {
        let (mut w, a, _) = duel();
        if let Some(o) = ops_mut(&mut w, a) {
            o.sys.set_power(Sys::Lances, 0.0);
        }
        let ev = fire(&mut w, a, Command::FireLance);
        assert!(!ev.iter().any(|e| matches!(e, Event::LanceFired { .. })));
        if let Some(o) = ops_mut(&mut w, a) {
            o.sys.set_power(Sys::Lances, 1.0);
        }
        let ev = fire(&mut w, a, Command::FireLance);
        assert!(ev.iter().any(|e| matches!(e, Event::LanceFired { .. })));
        let ev = fire(&mut w, a, Command::FireLance);
        assert!(
            !ev.iter().any(|e| matches!(e, Event::LanceFired { .. })),
            "cooldown"
        );
        let _ = DT;
    }
}
