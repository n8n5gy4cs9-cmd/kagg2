//! Science station (T13): target scanning, shield-frequency reveal, long-range sweep.

use super::clock::DT;
use super::command::Command;
use super::event::Event;
use super::ops::{FACING_NAMES, Faction, Ops};
use super::systems::Sys;
use super::world::{EntityId, Kind, World};

/// Detection range of the sensors in world units.
pub fn sensor_range(o: &Ops) -> f32 {
    let boost = if o.scan.sweep > 0.0 { 2.5 } else { 1.0 };
    14_000.0 * o.sys.eff(Sys::Sensors).clamp(0.3, 1.5) * boost
}

pub fn handle(w: &mut World, ship: EntityId, cmd: Command) {
    let Some(o) = w.get_mut(ship).and_then(|e| e.ops.as_deref_mut()) else {
        return;
    };
    match cmd {
        Command::Scan(t) => {
            if o.scan.target != t {
                o.scan.progress = 0.0;
            }
            o.scan.target = t;
        }
        Command::Sweep if o.scan.sweep_cd <= 0.0 => {
            o.scan.sweep = 8.0;
            o.scan.sweep_cd = 25.0;
            w.push_event(Event::Message("LONG-RANGE SWEEP ACTIVE".into()));
        }
        _ => {}
    }
}

/// Scan progress and sweep timers for one ship.
pub fn upkeep(w: &mut World, id: EntityId) {
    let Some(e) = w.get(id) else { return };
    let Some(o) = e.ops.as_deref() else { return };
    let target = o.scan.target;
    let range = sensor_range(o);
    let eff = o.sys.eff(Sys::Sensors).clamp(0.0, 1.5);
    let pos = e.pos;
    let tinfo = target
        .and_then(|t| w.get(t))
        .map(|t| (t.pos.distance(pos), t.radius, t.label));
    let Some(o) = w.get_mut(id).and_then(|e| e.ops.as_deref_mut()) else {
        return;
    };
    o.scan.sweep = (o.scan.sweep - DT).max(0.0);
    o.scan.sweep_cd = (o.scan.sweep_cd - DT).max(0.0);
    let (Some(t), Some((dist, radius, label))) = (target, tinfo) else {
        o.scan.target = None;
        return;
    };
    if o.scan.done.contains(&t) || dist > range {
        return;
    }
    o.scan.progress += DT * eff / (4.0 + radius / 25.0);
    if o.scan.progress >= 1.0 {
        o.scan.progress = 1.0;
        o.scan.done.push(t);
        w.push_event(Event::Message(format!(
            "SCAN COMPLETE: {}",
            label.to_uppercase()
        )));
    }
}

/// What science knows about a contact.
#[derive(Debug, Clone)]
pub struct Report {
    pub name: &'static str,
    pub class: &'static str,
    pub faction: Faction,
    pub hull: f32,
    pub max_hull: f32,
    pub shields: [f32; 4],
    pub max_shield: f32,
    pub shield_freq: u8,
    pub lance_freq: u8,
    pub weakest: usize,
    pub cargo: u16,
    pub boarding: u8,
    pub disabled: bool,
    pub hint: String,
}

/// Full report for a scanned ship; `None` while unscanned or not a ship.
pub fn report(w: &World, viewer: EntityId, target: EntityId) -> Option<Report> {
    let scanned = w
        .get(viewer)
        .and_then(|e| e.ops.as_deref())
        .is_some_and(|o| o.scan.done.contains(&target));
    if !scanned {
        return None;
    }
    let t = w.get(target)?;
    if t.kind != Kind::Ship {
        return None;
    }
    let o = t.ops.as_deref()?;
    let weakest = (0..4)
        .min_by(|&a, &b| o.shields[a].total_cmp(&o.shields[b]))
        .unwrap_or(0);
    let hint = if o.disabled {
        "DISABLED - SAFE TO BOARD".to_owned()
    } else if o.sys.eff(Sys::Shields) < 0.3 {
        "SHIELD GENERATOR FAILING - HIT THE HULL".to_owned()
    } else {
        format!(
            "TUNE LANCES TO {} (+/-1) FOR DOUBLE SHIELD DAMAGE. WEAKEST FACING: {}",
            o.shield_freq, FACING_NAMES[weakest]
        )
    };
    Some(Report {
        name: t.label,
        class: o.class_name,
        faction: t.faction,
        hull: o.hull,
        max_hull: o.max_hull,
        shields: o.shields,
        max_shield: o.max_shield,
        shield_freq: o.shield_freq,
        lance_freq: o.lance_freq,
        weakest,
        cargo: o.cargo.used(),
        boarding: o.boarding,
        disabled: o.disabled,
        hint,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::combat::tests::spawn_ship;
    use crate::sim::flight::{CUTTER, LONGSHIP};
    use crate::sim::{Addressed, Command};
    use glam::Vec3;

    #[test]
    fn scan_takes_time_then_reveals_frequency() {
        let mut w = World::new(1);
        let a = spawn_ship(&mut w, LONGSHIP, Vec3::ZERO, Faction::Mars);
        let b = spawn_ship(
            &mut w,
            CUTTER,
            Vec3::new(0.0, 0.0, -2000.0),
            Faction::Minion,
        );
        w.player = Some(a);
        w.step(&[Addressed {
            ship: a,
            cmd: Command::Scan(Some(b)),
        }]);
        assert!(report(&w, a, b).is_none());
        for _ in 0..60 * 12 {
            w.step(&[]);
        }
        let r = report(&w, a, b).expect("scanned");
        assert_eq!(r.class, "Cutter");
        assert!(r.hint.contains("TUNE LANCES") || r.hint.contains("FAILING"));
    }

    #[test]
    fn out_of_range_contacts_do_not_scan() {
        let mut w = World::new(1);
        let a = spawn_ship(&mut w, LONGSHIP, Vec3::ZERO, Faction::Mars);
        let b = spawn_ship(
            &mut w,
            CUTTER,
            Vec3::new(0.0, 0.0, -40_000.0),
            Faction::Minion,
        );
        w.player = Some(a);
        w.step(&[Addressed {
            ship: a,
            cmd: Command::Scan(Some(b)),
        }]);
        for _ in 0..60 * 20 {
            w.step(&[]);
        }
        assert!(report(&w, a, b).is_none());
    }

    #[test]
    fn sweep_extends_range_and_cools_down() {
        let mut w = World::new(1);
        let a = spawn_ship(&mut w, LONGSHIP, Vec3::ZERO, Faction::Mars);
        w.player = Some(a);
        let base = w
            .get(a)
            .and_then(|e| e.ops.as_deref())
            .map(sensor_range)
            .unwrap_or(0.0);
        w.step(&[Addressed {
            ship: a,
            cmd: Command::Sweep,
        }]);
        let boosted = w
            .get(a)
            .and_then(|e| e.ops.as_deref())
            .map(sensor_range)
            .unwrap_or(0.0);
        assert!(boosted > base * 2.0);
        w.step(&[Addressed {
            ship: a,
            cmd: Command::Sweep,
        }]);
        assert!(
            w.get(a)
                .and_then(|e| e.ops.as_deref())
                .is_some_and(|o| o.scan.sweep_cd > 20.0)
        );
    }
}
