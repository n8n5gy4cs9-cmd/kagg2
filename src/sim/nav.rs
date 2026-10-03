//! Navigation (T14), docking and mining (T16), and the command router for non-combat orders.

use glam::Vec3;

use super::ai::{Mode, Order, steer};
use super::clock::DT;
use super::command::{Addressed, Command};
use super::event::Event;
use super::ops::{Dest, Faction, Ops};
use super::systems::Sys;
use super::world::{EntityId, Kind, World};

/// Commodity index of raw ore (see `econ`).
pub const ORE: usize = 0;
const DOCK_RANGE: f32 = 500.0;
const DOCK_SPEED: f32 = 60.0;
const MINE_RANGE: f32 = 600.0;

fn ops_mut(w: &mut World, id: EntityId) -> Option<&mut Ops> {
    w.get_mut(id).and_then(|e| e.ops.as_deref_mut())
}

fn msg(w: &mut World, text: impl Into<String>) {
    w.push_event(Event::Message(text.into()));
}

/// Command router for navigation, science, comms, docking and mining commands.
pub fn handle(w: &mut World, ship: EntityId, cmd: Command) {
    match cmd {
        Command::SetDest(d) => {
            if let Some(o) = ops_mut(w, ship) {
                o.nav.dest = d;
                o.nav.route.clear();
                if d == Dest::None {
                    o.nav.autopilot = false;
                }
            }
        }
        Command::AddWaypoint(p) => {
            if let Some(o) = ops_mut(w, ship) {
                if o.nav.dest == Dest::None {
                    o.nav.dest = Dest::Point(p);
                } else if o.nav.route.len() < 8 {
                    o.nav.route.push(p);
                }
            }
        }
        Command::Autopilot(on) => {
            if let Some(o) = ops_mut(w, ship) {
                o.nav.autopilot = on && o.nav.dest != Dest::None && o.docked.is_none();
            }
        }
        Command::Warp(target) => start_warp(w, ship, target),
        Command::Order(order) => {
            let wings: Vec<EntityId> = w
                .iter()
                .filter(|(_, e)| {
                    e.faction == Faction::Mars
                        && e.ai.is_some_and(|a| matches!(a.mode, Mode::Wingman(_)))
                })
                .map(|(i, _)| i)
                .collect();
            for id in &wings {
                if let Some(a) = w.get_mut(*id).and_then(|e| e.ai.as_mut()) {
                    a.mode = Mode::Wingman(order);
                }
            }
            if !wings.is_empty() {
                let what = match order {
                    Order::Attack => "ATTACK MY TARGET",
                    Order::Guard => "GUARD",
                    Order::Follow => "FORM UP",
                    Order::Retreat => "FALL BACK",
                };
                msg(w, format!("WINGMEN: ACKNOWLEDGED - {what}"));
            }
        }
        Command::DemandSurrender(t) => demand_surrender(w, ship, t),
        Command::Dock(Some(st)) => dock(w, ship, st),
        Command::Dock(None) => undock(w, ship),
        Command::Mine(t) => start_mining(w, ship, t),
        Command::Scan(_) | Command::Sweep => super::science::handle(w, ship, cmd),
        _ => {}
    }
}

fn start_warp(w: &mut World, ship: EntityId, target: Option<crate::sim::SectorId>) {
    let here = w.sector;
    let Some(o) = ops_mut(w, ship) else { return };
    match target {
        None => o.nav.warp = None,
        Some(to) if to == here => {}
        Some(to) => {
            let cost = here.warp_cost(to);
            if o.sys.eff(Sys::Warp) < 0.3 {
                msg(w, "WARP DRIVE OFFLINE - NEEDS POWER OR REPAIR");
            } else if o.nav.fuel < cost {
                msg(w, format!("NOT ENOUGH FUEL FOR WARP ({cost:.0} NEEDED)"));
            } else if o.docked.is_some() {
                msg(w, "UNDOCK BEFORE WARPING");
            } else {
                o.nav.warp = Some((to, 0.0));
                msg(
                    w,
                    format!("WARP DRIVE CHARGING - {}", to.name().to_uppercase()),
                );
            }
        }
    }
}

fn demand_surrender(w: &mut World, ship: EntityId, target: EntityId) {
    let roll = w.rng.f32();
    let Some(t) = w.get(target) else { return };
    let (Some(o), Some(me)) = (t.ops.as_deref(), w.get(ship)) else {
        return;
    };
    if me.pos.distance(t.pos) > 4000.0 || t.ai.is_none() || !t.faction.hostile_to(me.faction) {
        return;
    }
    let label = t.label;
    let frac = o.hull / o.max_hull;
    // Weak ships give up; a dreadnought never does.
    let chance = (0.65 - frac) * 2.0 / (1.0 + (o.boarding as f32 - 1.0) * 0.5);
    if o.disabled {
        msg(w, format!("{label}: ...we are already dead in space."));
    } else if roll < chance {
        if let Some(o) = ops_mut(w, target) {
            o.disabled = true;
        }
        if let Some(a) = w.get_mut(target).and_then(|e| e.ai.as_mut()) {
            a.mode = Mode::Disabled;
        }
        w.push_event(Event::Surrendered(target));
        msg(w, format!("{label}: WE YIELD! DON'T FIRE!"));
    } else {
        let seed = w.rng.next_u64();
        msg(w, format!("{label}: {}", super::comms::taunt(seed)));
    }
}

fn dock(w: &mut World, ship: EntityId, st: EntityId) {
    let Some(s) = w.get(st) else { return };
    let Some(e) = w.get(ship) else { return };
    let ok = s.kind == Kind::Station && s.faction != Faction::Minion;
    let rel_speed = (e.vel - s.vel).length();
    let gap = e.pos.distance(s.pos) - s.radius - e.radius;
    let (spos, srad, erad, epos, label) = (s.pos, s.radius, e.radius, e.pos, s.label);
    if !ok {
        msg(w, "DOCKING DENIED - HOSTILE OR NOT A STARBASE");
        return;
    }
    if gap > DOCK_RANGE {
        msg(
            w,
            format!("{}: TOO FAR TO DOCK ({gap:.0} U)", label.to_uppercase()),
        );
        return;
    }
    if rel_speed > DOCK_SPEED {
        msg(w, "SLOW DOWN TO DOCK (UNDER 60 U/S)");
        return;
    }
    let mut off = epos - spos;
    let min = srad + erad + 30.0;
    if off.length() < min {
        off = off.try_normalize().unwrap_or(Vec3::Y) * min;
    }
    if let Some(o) = ops_mut(w, ship) {
        o.docked = Some((st, off));
        o.nav.autopilot = false;
        o.nav.warp = None;
        o.mining = None;
    }
    w.push_event(Event::Docked(st));
    msg(w, format!("DOCKED AT {}", label.to_uppercase()));
}

fn undock(w: &mut World, ship: EntityId) {
    let Some((st, off)) = w
        .get(ship)
        .and_then(|e| e.ops.as_deref())
        .and_then(|o| o.docked)
    else {
        return;
    };
    let (spos, svel, srad) = w
        .get(st)
        .map_or((Vec3::ZERO, Vec3::ZERO, 0.0), |s| (s.pos, s.vel, s.radius));
    if let Some(e) = w.get_mut(ship) {
        let dir = off.try_normalize().unwrap_or(Vec3::Z);
        e.pos = spos + dir * (srad + e.radius + 90.0);
        e.vel = svel;
        if let Some(o) = e.ops.as_deref_mut() {
            o.docked = None;
        }
    }
    w.push_event(Event::Undocked);
}

fn start_mining(w: &mut World, ship: EntityId, target: Option<EntityId>) {
    let Some(t) = target else {
        if let Some(o) = ops_mut(w, ship) {
            o.mining = None;
        }
        return;
    };
    let Some(te) = w.get(t) else { return };
    let Some(me) = w.get(ship) else { return };
    if te.kind != Kind::Asteroid && te.kind != Kind::Moon {
        msg(w, "MINING BOT ONLY WORKS ON ASTEROIDS AND MOONLETS");
        return;
    }
    let gap = me.pos.distance(te.pos) - te.radius - me.radius;
    if gap > MINE_RANGE {
        msg(
            w,
            format!("TOO FAR TO MINE ({gap:.0} U, MAX {MINE_RANGE:.0})"),
        );
        return;
    }
    if let Some(o) = ops_mut(w, ship) {
        o.mining = Some((t, 0.0));
    }
    msg(w, "MINING BOT DEPLOYED");
}

/// World-space `(entity index, position, velocity)` for ships held by a station.
pub fn dock_positions(w: &mut World) -> Vec<(u32, Vec3, Vec3)> {
    let mut out = Vec::new();
    let mut lost = Vec::new();
    for (id, e) in w.iter() {
        if let Some((st, off)) = e.ops.as_deref().and_then(|o| o.docked) {
            match w.get(st) {
                Some(s) => out.push((id.index, s.pos + off, s.vel)),
                None => lost.push(id),
            }
        }
    }
    for id in lost {
        if let Some(o) = ops_mut(w, id) {
            o.docked = None;
        }
    }
    out
}

/// Where a destination currently is and how close counts as arrived.
fn dest_point(w: &World, d: Dest, ship_radius: f32) -> Option<(Vec3, f32)> {
    match d {
        Dest::None => None,
        Dest::Point(p) => Some((p, 120.0)),
        Dest::Body(id) => w.get(id).map(|e| {
            let pad = if e.kind == Kind::Station {
                260.0
            } else {
                400.0
            };
            (e.pos, e.radius + ship_radius + pad)
        }),
    }
}

/// Autopilot: steer and throttle toward the destination, route around solid bodies.
pub fn autopilot(w: &mut World) -> Vec<Addressed> {
    let ids: Vec<EntityId> = w
        .iter()
        .filter(|(_, e)| e.ops.as_ref().is_some_and(|o| o.nav.autopilot && !o.dead))
        .map(|(i, _)| i)
        .collect();
    let mut out = Vec::new();
    for id in ids {
        let Some(e) = w.get(id) else { continue };
        let Some(o) = e.ops.as_deref() else { continue };
        let dest = o.nav.dest;
        let Some((target, arrive)) = dest_point(w, dest, e.radius) else {
            if let Some(o) = ops_mut(w, id) {
                o.nav.autopilot = false;
            }
            continue;
        };
        let mut dir = target - e.pos;
        let dist = dir.length();
        if dist <= arrive {
            let next = {
                let o = ops_mut(w, id);
                o.and_then(|o| {
                    if o.nav.route.is_empty() {
                        o.nav.dest = Dest::None;
                        o.nav.autopilot = false;
                        None
                    } else {
                        let p = o.nav.route.remove(0);
                        o.nav.dest = Dest::Point(p);
                        Some(p)
                    }
                })
            };
            if next.is_none() {
                out.push(Addressed {
                    ship: id,
                    cmd: Command::SetThrottle(0.0),
                });
                out.push(Addressed {
                    ship: id,
                    cmd: Command::Brake(true),
                });
                msg(w, "AUTOPILOT: ARRIVED");
            }
            continue;
        }
        dir /= dist;
        // avoid solid bodies between here and there
        for (oid, s) in w.iter() {
            if oid == id || matches!(dest, Dest::Body(b) if b == oid) || s.ship.is_some() {
                continue;
            }
            if matches!(s.kind, Kind::Projectile | Kind::Debris) {
                continue;
            }
            let oc = s.pos - e.pos;
            let t = oc.dot(dir);
            if t <= 0.0 || t > dist {
                continue;
            }
            let closest = oc - dir * t;
            let clear = s.radius * 1.2 + e.radius + 140.0;
            if closest.length() < clear {
                let side = (-closest)
                    .try_normalize()
                    .unwrap_or_else(|| dir.any_orthonormal_vector());
                dir = (dir + side * 1.6).normalize();
            }
        }
        let top = e.ship.map_or(300.0, |s| s.class.max_speed);
        let throttle = ((dist - arrive) / (top * 3.0)).clamp(0.08, 1.0);
        let me = e.clone();
        out.push(Addressed {
            ship: id,
            cmd: steer(&me, dir),
        });
        out.push(Addressed {
            ship: id,
            cmd: Command::SetThrottle(throttle),
        });
        out.push(Addressed {
            ship: id,
            cmd: Command::Brake(false),
        });
    }
    out
}

/// Warp charge, mining bot and scanning for every ship.
pub fn upkeep(w: &mut World) {
    let ids: Vec<EntityId> = w
        .iter()
        .filter(|(_, e)| e.ops.is_some())
        .map(|(i, _)| i)
        .collect();
    let here = w.sector;
    for id in ids {
        let mining = w
            .get(id)
            .and_then(|e| e.ops.as_deref())
            .and_then(|o| o.mining);
        if let Some((t, prog)) = mining {
            mine_step(w, id, t, prog);
        }
        let Some(o) = ops_mut(w, id) else { continue };
        if let Some((to, c)) = o.nav.warp {
            let nc = c + DT * o.sys.eff(Sys::Warp).min(1.5) / 6.0;
            if nc >= 1.0 {
                o.nav.fuel -= here.warp_cost(to);
                o.nav.warp = None;
                w.push_event(Event::WarpJump(to));
            } else {
                o.nav.warp = Some((to, nc));
            }
        }
        super::science::upkeep(w, id);
    }
}

fn mine_step(w: &mut World, id: EntityId, target: EntityId, prog: f32) {
    let valid = match (w.get(id), w.get(target)) {
        (Some(me), Some(t)) => me.pos.distance(t.pos) - t.radius - me.radius <= MINE_RANGE * 1.5,
        _ => false,
    };
    let Some(o) = ops_mut(w, id) else { return };
    if !valid {
        o.mining = None;
        msg(w, "MINING BOT RECALLED");
        return;
    }
    let rate = 0.4 * o.sys.eff(Sys::Sensors).clamp(0.3, 1.5);
    let p = prog + rate * DT;
    if p >= 1.0 {
        if o.cargo.used() < o.cargo.capacity {
            o.cargo.hold[ORE] += 1;
            o.mining = Some((target, p - 1.0));
        } else {
            o.mining = None;
            msg(w, "HOLD FULL - MINING BOT RECALLED");
        }
    } else {
        o.mining = Some((target, p));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::combat::tests::spawn_ship;
    use crate::sim::flight::LONGSHIP;
    use crate::sim::{SectorId, new_game};

    fn cmd(w: &mut World, c: Command) -> Vec<Event> {
        let me = w.player.expect("player");
        w.step(&[Addressed { ship: me, cmd: c }])
    }

    #[test]
    fn autopilot_reaches_a_waypoint_and_stops() {
        let mut w = World::new(1);
        let p = spawn_ship(&mut w, LONGSHIP, Vec3::ZERO, Faction::Mars);
        w.player = Some(p);
        cmd(
            &mut w,
            Command::SetDest(Dest::Point(Vec3::new(1500.0, 200.0, -2500.0))),
        );
        cmd(&mut w, Command::Autopilot(true));
        let mut arrived = false;
        for _ in 0..60 * 60 {
            arrived |= w
                .step(&[])
                .iter()
                .any(|e| matches!(e, Event::Message(m) if m.contains("ARRIVED")));
            if arrived {
                break;
            }
        }
        assert!(arrived);
        let o = w.get(p).and_then(|e| e.ops.as_deref()).expect("ops");
        assert!(!o.nav.autopilot && o.nav.dest == Dest::None);
    }

    #[test]
    fn autopilot_flies_around_a_planet() {
        let mut w = new_game(1, SectorId::Luna);
        let me = w.player.expect("player");
        let luna = w
            .iter()
            .find(|(_, e)| e.label == "Luna")
            .map(|(i, _)| i)
            .expect("luna");
        let lp = w.get(luna).map(|e| e.pos).unwrap_or(Vec3::ZERO);
        let beyond = lp + Vec3::new(0.0, 0.0, -6000.0);
        cmd(&mut w, Command::SetDest(Dest::Point(beyond)));
        cmd(&mut w, Command::Autopilot(true));
        let mut bumped = false;
        for _ in 0..60 * 120 {
            bumped |= w
                .step(&[])
                .iter()
                .any(|e| matches!(e, Event::Collision { other, speed, .. } if *other == luna && *speed > 20.0));
        }
        assert!(!bumped, "autopilot hit the moon");
        let d = w.get(me).map_or(0.0, |e| e.pos.distance(beyond));
        assert!(d < 3000.0, "still {d} away");
    }

    #[test]
    fn docking_needs_range_speed_and_friendly_station() {
        let mut w = new_game(1, SectorId::MarsOrbit);
        let me = w.player.expect("player");
        let forge = w
            .iter()
            .find(|(_, e)| e.label == "Forge")
            .map(|(i, _)| i)
            .expect("forge");
        let fp = w.get(forge).map(|e| e.pos).unwrap_or(Vec3::ZERO);
        let ev = cmd(&mut w, Command::Dock(Some(forge)));
        assert!(
            ev.iter()
                .any(|e| matches!(e, Event::Message(m) if m.contains("TOO FAR")))
        );
        if let Some(e) = w.get_mut(me) {
            e.pos = fp + Vec3::new(0.0, 0.0, 400.0);
            e.vel = Vec3::ZERO;
        }
        let ev = cmd(&mut w, Command::Dock(Some(forge)));
        assert!(ev.iter().any(|e| matches!(e, Event::Docked(_))));
        let p0 = w
            .get(me)
            .map(|e| e.pos - w.get(forge).map_or(Vec3::ZERO, |f| f.pos));
        for _ in 0..600 {
            w.step(&[]);
        }
        let p1 = w
            .get(me)
            .map(|e| e.pos - w.get(forge).map_or(Vec3::ZERO, |f| f.pos));
        assert!(
            p0.zip(p1).is_some_and(|(a, b)| a.distance(b) < 1.0),
            "ship drifted off the dock"
        );
        let ev = cmd(&mut w, Command::Dock(None));
        assert!(ev.iter().any(|e| matches!(e, Event::Undocked)));
    }

    #[test]
    fn warp_charges_costs_fuel_and_jumps() {
        let mut w = new_game(1, SectorId::MarsOrbit);
        cmd(&mut w, Command::Warp(Some(SectorId::Belt)));
        let mut jumped = false;
        for _ in 0..60 * 10 {
            jumped |= w
                .step(&[])
                .iter()
                .any(|e| matches!(e, Event::WarpJump(SectorId::Belt)));
        }
        assert!(jumped);
        let fuel = w
            .player
            .and_then(|p| w.get(p))
            .and_then(|e| e.ops.as_deref())
            .map(|o| o.nav.fuel);
        assert!(fuel.is_some_and(|f| f < 100.0));
    }

    #[test]
    fn mining_fills_the_hold() {
        let mut w = new_game(1, SectorId::Belt);
        let me = w.player.expect("player");
        let rock = w
            .iter()
            .find(|(_, e)| e.kind == Kind::Asteroid)
            .map(|(i, e)| (i, e.pos, e.radius))
            .expect("rock");
        if let Some(e) = w.get_mut(me) {
            e.pos = rock.1 + Vec3::new(0.0, 0.0, rock.2 + 200.0);
            e.vel = Vec3::ZERO;
        }
        cmd(&mut w, Command::Mine(Some(rock.0)));
        for _ in 0..60 * 30 {
            w.step(&[]);
        }
        let ore = w
            .get(me)
            .and_then(|e| e.ops.as_deref())
            .map(|o| o.cargo.hold[ORE]);
        assert!(ore.is_some_and(|n| n >= 8), "{ore:?}");
    }
}
