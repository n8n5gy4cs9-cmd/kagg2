//! Crew AI (T10): every station can be left on AUTO; the crew then issues the same commands a
//! player would. Skill (`0.2..=1.0`) scales how often and how well.

use super::ai::{Mode, steer};
use super::command::{Addressed, Command};
use super::ops::{Dest, Faction};
use super::systems::{POWERED, Sys};
use super::world::{EntityId, Kind, World};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Station {
    Helm,
    Weapons,
    Engineering,
    Damage,
    Science,
    Comms,
    Map,
}

impl Station {
    pub const ALL: [Station; 7] = [
        Station::Helm,
        Station::Weapons,
        Station::Engineering,
        Station::Damage,
        Station::Science,
        Station::Comms,
        Station::Map,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Station::Helm => "HELM",
            Station::Weapons => "WEAPONS",
            Station::Engineering => "ENGINEERING",
            Station::Damage => "DAMAGE",
            Station::Science => "SCIENCE",
            Station::Comms => "COMMS",
            Station::Map => "MAP",
        }
    }

    /// Whether a crew AI can run this station.
    pub fn automatable(self) -> bool {
        !matches!(self, Station::Comms | Station::Map)
    }
}

#[derive(Debug, Clone)]
pub struct Crew {
    pub auto: [bool; 7],
    pub skill: f32,
}

impl Default for Crew {
    fn default() -> Self {
        Crew {
            auto: [false; 7],
            skill: 0.7,
        }
    }
}

impl Crew {
    pub fn is_auto(&self, s: Station) -> bool {
        self.auto[s as usize]
    }

    pub fn toggle(&mut self, s: Station) {
        if s.automatable() {
            self.auto[s as usize] = !self.auto[s as usize];
        }
    }
}

fn hostiles(w: &World, me: EntityId) -> Vec<(EntityId, f32)> {
    let Some(m) = w.get(me) else {
        return Vec::new();
    };
    let mut v: Vec<(EntityId, f32)> = w
        .iter()
        .filter(|(_, e)| {
            e.kind == Kind::Ship
                && e.faction.hostile_to(m.faction)
                && e.ops.as_ref().is_some_and(|o| !o.dead && !o.disabled)
        })
        .map(|(i, e)| (i, e.pos.distance(m.pos)))
        .collect();
    v.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
    v
}

/// Commands from the crew stations that are on AUTO.
pub fn think(w: &mut World) -> Vec<Addressed> {
    let mut out = Vec::new();
    let Some(me) = w.player else { return out };
    let Some(e) = w.get(me) else { return out };
    let Some(o) = e.ops.as_deref() else {
        return out;
    };
    if o.dead {
        return out;
    }
    let crew = w.crew.clone();
    let tick = w.tick;
    // a less skilled crew reacts less often
    let period = (90.0 / crew.skill.max(0.2)) as u64;
    let foes = hostiles(w, me);
    let near = foes.first().copied();
    let mut push = |cmd: Command| out.push(Addressed { ship: me, cmd });

    if crew.is_auto(Station::Engineering) && tick.is_multiple_of(period) {
        let threat = near.is_some_and(|f| f.1 < 4500.0);
        let warping = o.nav.warp.is_some();
        push(Command::PowerPreset(if threat {
            2
        } else if warping || o.nav.autopilot {
            3
        } else {
            1
        }));
        let heats = o.sys.heat;
        for (i, s) in Sys::ALL.iter().take(POWERED).enumerate() {
            push(Command::SetCoolant(
                *s,
                ((heats[i] + 15.0) / 100.0).clamp(0.1, 1.0),
            ));
        }
    }
    if crew.is_auto(Station::Damage) && tick.is_multiple_of(period) {
        // worst three systems get the three teams (reactor, life support first on ties)
        let mut order: Vec<Sys> = Sys::ALL.to_vec();
        order.sort_by(|a, b| {
            o.sys.health[a.idx()]
                .total_cmp(&o.sys.health[b.idx()])
                .then(b.cmp(a))
        });
        for (t, s) in order.iter().take(3).enumerate() {
            let sys = (o.sys.health[s.idx()] < 100.0).then_some(*s);
            push(Command::AssignRepair { team: t as u8, sys });
        }
    }
    if crew.is_auto(Station::Science) && tick.is_multiple_of(period) {
        let pick = near.map(|n| n.0).or(o.target);
        if let Some(t) = pick
            && o.scan.target != Some(t)
            && !o.scan.done.contains(&t)
        {
            push(Command::Scan(Some(t)));
        }
    }
    if crew.is_auto(Station::Weapons) {
        if o.target.is_none_or(|t| w.get(t).is_none())
            && tick.is_multiple_of(period)
            && let Some((t, _)) = near
        {
            push(Command::SetTarget(Some(t)));
        }
        if let Some(t) = o.target.and_then(|t| w.get(t)) {
            let to = t.pos - e.pos;
            let dist = to.length();
            let align = e.forward().angle_between(to);
            let gate = (tick / 8) % 100 < (30.0 + 70.0 * crew.skill) as u64;
            if t.faction.hostile_to(Faction::Mars) || t.faction.hostile_to(e.faction) {
                if dist < super::combat::LANCE_RANGE && align < 0.06 && gate {
                    push(Command::FireLance);
                }
                if o.torpedoes > 0
                    && align < 0.1
                    && (500.0..2000.0).contains(&dist)
                    && t.radius > 15.0
                {
                    push(Command::FireTorpedo);
                }
                if o.missiles > 0 && dist > 800.0 && dist < 6000.0 && tick.is_multiple_of(180) {
                    push(Command::FireMissile);
                }
            }
        }
    }
    if crew.is_auto(Station::Helm) && tick.is_multiple_of(period) {
        if o.nav.dest == Dest::None
            && let Some((t, _)) = near
        {
            push(Command::SetDest(Dest::Body(t)));
        }
        if o.nav.dest != Dest::None && !o.nav.autopilot {
            push(Command::Autopilot(true));
        }
    }
    let _ = (steer, Mode::Patrol);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::combat::tests::spawn_ship;
    use crate::sim::flight::{CUTTER, LONGSHIP};
    use glam::Vec3;

    fn setup() -> (World, EntityId, EntityId) {
        let mut w = World::new(3);
        let p = spawn_ship(&mut w, LONGSHIP, Vec3::ZERO, Faction::Mars);
        let f = spawn_ship(
            &mut w,
            CUTTER,
            Vec3::new(0.0, 0.0, -1500.0),
            Faction::Minion,
        );
        w.player = Some(p);
        (w, p, f)
    }

    #[test]
    fn engineering_switches_to_battle_when_threatened() {
        let (mut w, p, _) = setup();
        w.crew.auto[Station::Engineering as usize] = true;
        for _ in 0..200 {
            w.step(&[]);
        }
        let o = w.get(p).and_then(|e| e.ops.as_deref()).expect("ops");
        assert!(o.sys.power[Sys::Lances.idx()] > 1.4);
    }

    #[test]
    fn damage_crew_assigns_teams_to_worst_systems() {
        let (mut w, p, _) = setup();
        w.crew.auto[Station::Damage as usize] = true;
        if let Some(o) = w.get_mut(p).and_then(|e| e.ops.as_deref_mut()) {
            o.sys.damage(Sys::Engines, 50.0);
            o.sys.damage(Sys::Shields, 30.0);
        }
        for _ in 0..200 {
            w.step(&[]);
        }
        let o = w.get(p).and_then(|e| e.ops.as_deref()).expect("ops");
        assert!(
            o.sys.teams.contains(&Some(Sys::Engines)) && o.sys.teams.contains(&Some(Sys::Shields))
        );
    }

    #[test]
    fn science_and_weapons_crew_work_the_nearest_threat() {
        let (mut w, p, f) = setup();
        w.crew.auto[Station::Science as usize] = true;
        w.crew.auto[Station::Weapons as usize] = true;
        // face the enemy (ship forward is -Z already)
        let mut fired = false;
        for _ in 0..60 * 15 {
            fired |= w.step(&[]).iter().any(
                |e| matches!(e, crate::sim::Event::LanceFired { shooter, .. } if *shooter == p),
            );
        }
        assert!(fired);
        let o = w.get(p).and_then(|e| e.ops.as_deref()).expect("ops");
        assert!(o.scan.done.contains(&f) || w.get(f).is_none());
    }
}
