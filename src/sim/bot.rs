//! Headless bridge bot (T30): flies the player's ship toward the current mission goal, shoots
//! hostiles, docks, mines, scans and asks to board. It speaks only `Command`s (plus the same
//! direct dock-screen purchases the UI makes), so it also exercises the crew AI and nav code.

use glam::Vec3;

use super::ai::steer;
use super::combat::LANCE_RANGE;
use super::command::{Addressed, Command};
use super::crew::Station;
use super::econ::{self, Upgrade};
use super::mission::Goal;
use super::nav::ORE;
use super::ops::{Dest, Faction};
use super::systems::Sys;
use super::world::{EntityId, Kind, World};

/// Boarding range the scene demands (gap between hulls).
pub const BOARD_RANGE: f32 = 450.0;

/// Range at which the bot stops closing on a target and brakes.
const STANDOFF: f32 = 450.0;

/// What the bot wants this tick.
#[derive(Debug, Default)]
pub struct Decision {
    pub cmds: Vec<Addressed>,
    /// A boardable hostile is in range and the mission (or the bot) wants it boarded.
    pub board: Option<EntityId>,
}

#[derive(Debug, Clone)]
pub struct SpaceBot {
    /// Seconds of game time between lance bursts allowed on a "preserve" target.
    pub fire_gate: u64,
}

impl Default for SpaceBot {
    fn default() -> Self {
        SpaceBot { fire_gate: 1 }
    }
}

fn is_prefix_goal(goal: Option<Goal>, label: &str) -> bool {
    match goal {
        Some(Goal::Disabled(p) | Goal::ClearBut(p) | Goal::Board { ship: p, .. }) => {
            label.starts_with(p)
        }
        _ => false,
    }
}

impl SpaceBot {
    /// Put the crew stations the bot does not run itself on AUTO.
    pub fn set_crew(&self, w: &mut World) {
        for s in [Station::Engineering, Station::Damage] {
            w.crew.auto[s as usize] = true;
        }
        w.crew.skill = 0.9;
    }

    /// Decide this tick's commands for the player's ship given the mission `goal`.
    pub fn think(&mut self, w: &mut World, goal: Option<Goal>) -> Decision {
        let mut d = Decision::default();
        let Some(me) = w.player else { return d };
        let Some(e) = w.get(me) else { return d };
        let Some(o) = e.ops.as_deref() else { return d };
        if o.dead {
            return d;
        }
        let (pos, tick) = (e.pos, w.tick);
        let docked = o.docked.map(|x| x.0);
        macro_rules! push {
            ($c:expr) => {
                d.cmds.push(Addressed { ship: me, cmd: $c })
            };
        }

        if let Some(st) = docked {
            // services, then undock unless the goal wants us docked
            let stay = matches!(goal, Some(Goal::DockAt(_)));
            let station = w.get(st).map(|s| (s.label, s.faction));
            if let (Some((label, fac)), Some(ops)) =
                (station, w.get_mut(me).and_then(|x| x.ops.as_deref_mut()))
            {
                let mult = econ::service_mult(label, fac);
                let _ = econ::refuel(ops, mult);
                let _ = econ::repair(ops, mult);
                let _ = econ::rearm(ops, mult);
                if matches!(goal, Some(Goal::Upgrade)) {
                    let pick = Upgrade::ALL
                        .iter()
                        .filter_map(|u| u.cost(ops).map(|c| (c, *u)))
                        .min_by_key(|(c, _)| *c);
                    if let Some((_, u)) = pick {
                        let _ = econ::upgrade(ops, u);
                    }
                }
            }
            if !stay {
                d.cmds.push(Addressed {
                    ship: me,
                    cmd: Command::Dock(None),
                });
            }
            return d;
        }

        // hostiles, nearest first
        let mut foes: Vec<(EntityId, f32)> = w
            .iter()
            .filter(|(_, x)| {
                x.kind == Kind::Ship
                    && x.faction.hostile_to(Faction::Mars)
                    && x.ops
                        .as_deref()
                        .is_some_and(|ops| !ops.dead && !ops.disabled)
            })
            .map(|(i, x)| (i, x.pos.distance(pos)))
            .collect();
        foes.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
        let wants_kill = matches!(
            goal,
            Some(
                Goal::Clear
                    | Goal::ClearBut(_)
                    | Goal::Gone(_)
                    | Goal::Disabled(_)
                    | Goal::Board { .. }
            )
        );
        // a crippled preserve target (boardable) is not shot at any more
        let preserved = |id: EntityId| {
            w.get(id).is_some_and(|t| {
                is_prefix_goal(goal, t.label)
                    && (matches!(goal, Some(Goal::ClearBut(_)))
                        || t.ops.as_deref().is_some_and(|x| x.disabled || x.crippled()))
            })
        };
        let prefix_of = |id: EntityId| w.get(id).is_some_and(|t| is_prefix_goal(goal, t.label));
        // boarding: a crippled prefix target within range
        if matches!(goal, Some(Goal::Board { .. }))
            && let Some(t) = w.iter().find(|(id, t)| {
                preserved(*id)
                    && t.faction.hostile_to(Faction::Mars)
                    && t.ops.as_deref().is_some_and(|x| !x.dead)
            })
        {
            let gap = t.1.pos.distance(pos) - t.1.radius - e.radius;
            if gap <= BOARD_RANGE {
                d.board = Some(t.0);
            }
        }
        let live: Vec<(EntityId, f32)> = foes
            .iter()
            .copied()
            .filter(|(id, _)| !preserved(*id))
            .collect();
        // shoot whoever threatens us before the prey; hunt the prey when alone
        let engage = live
            .iter()
            .copied()
            .find(|(id, dist)| !prefix_of(*id) && *dist < 3500.0)
            .or_else(|| {
                live.first()
                    .copied()
                    .filter(|(_, dist)| wants_kill || *dist < 3500.0)
            });

        if let Some((tid, dist)) = engage {
            let Some(t) = w.get(tid) else { return d };
            let Some(me_e) = w.get(me) else { return d };
            let to = t.pos - me_e.pos;
            let align = me_e.forward().angle_between(to);
            let hull_frac = t
                .ops
                .as_deref()
                .map_or(1.0, |x| x.hull / x.max_hull.max(1.0));
            let prey = prefix_of(tid);
            let ops = me_e.ops.as_deref();
            if ops.is_some_and(|x| x.nav.autopilot) {
                push!(Command::Autopilot(false));
            }
            if ops.and_then(|x| x.target) != Some(tid) {
                push!(Command::SetTarget(Some(tid)));
            }
            push!(steer(me_e, to));
            // hold a stand-off range instead of ramming the target
            if dist < STANDOFF {
                push!(Command::SetThrottle(0.0));
                push!(Command::Brake(true));
            } else {
                push!(Command::SetThrottle(
                    ((dist - STANDOFF) / 900.0).clamp(0.1, 1.0)
                ));
                push!(Command::Brake(false));
            }
            let cool = ops.is_some_and(|x| x.sys.heat[Sys::Lances.idx()] < 85.0);
            if dist < LANCE_RANGE && align < 0.06 && cool {
                push!(Command::FireLance);
            }
            let torp_ok = !prey || hull_frac > 0.8;
            if torp_ok
                && ops.is_some_and(|x| x.torpedoes > 0)
                && align < 0.1
                && (500.0..2000.0).contains(&dist)
                && t.radius > 15.0
            {
                push!(Command::FireTorpedo);
            }
            if torp_ok
                && ops.is_some_and(|x| x.missiles > 0)
                && (800.0..6000.0).contains(&dist)
                && tick.is_multiple_of(180)
            {
                push!(Command::FireMissile);
            }
            return d;
        }

        let me_e = match w.get(me) {
            Some(x) => x,
            None => return d,
        };
        let ops = me_e.ops.as_deref();
        let nav = |b: EntityId, out: &mut Vec<Addressed>| {
            let o = ops;
            if o.is_none_or(|x| x.nav.dest != Dest::Body(b)) {
                out.push(Addressed {
                    ship: me,
                    cmd: Command::SetDest(Dest::Body(b)),
                });
            }
            if o.is_none_or(|x| !x.nav.autopilot) {
                out.push(Addressed {
                    ship: me,
                    cmd: Command::Autopilot(true),
                });
            }
        };
        let find = |label: &str| w.iter().find(|(_, x)| x.label == label).map(|(i, _)| i);
        let nearest_where = |pred: &dyn Fn(&crate::sim::Entity) -> bool| {
            w.iter()
                .filter(|(i, x)| *i != me && pred(x))
                .min_by(|a, b| {
                    a.1.pos
                        .distance(pos)
                        .total_cmp(&b.1.pos.distance(pos))
                        .then(a.0.cmp(&b.0))
                })
                .map(|(i, x)| (i, x.pos, x.radius))
        };
        match goal {
            Some(Goal::Throttle(f)) => d.cmds.push(Addressed {
                ship: me,
                cmd: Command::SetThrottle(f + 0.2),
            }),
            Some(Goal::Target) => {
                if let Some((t, _, _)) = nearest_where(&|x| x.kind != Kind::Projectile) {
                    d.cmds.push(Addressed {
                        ship: me,
                        cmd: Command::SetTarget(Some(t)),
                    });
                }
            }
            Some(Goal::Reach { body, .. }) => {
                if let Some(b) = find(body) {
                    nav(b, &mut d.cmds);
                }
            }
            Some(Goal::DockAt(name)) => {
                if let Some(b) = find(name) {
                    let s = w.get(b);
                    let gap = s.map_or(1e9, |s| s.pos.distance(me_e.pos) - s.radius - me_e.radius);
                    let slow = s.is_some_and(|s| (me_e.vel - s.vel).length() < 55.0);
                    if gap < 480.0 && slow {
                        d.cmds.push(Addressed {
                            ship: me,
                            cmd: Command::Dock(Some(b)),
                        });
                    } else {
                        nav(b, &mut d.cmds);
                    }
                }
            }
            Some(Goal::Upgrade) => {
                let st = nearest_where(&|x| {
                    x.kind == Kind::Station && !x.faction.hostile_to(Faction::Mars)
                });
                if let Some((b, _, _)) = st {
                    let s = w.get(b);
                    let gap = s.map_or(1e9, |s| s.pos.distance(me_e.pos) - s.radius - me_e.radius);
                    let slow = s.is_some_and(|s| (me_e.vel - s.vel).length() < 55.0);
                    if gap < 480.0 && slow {
                        d.cmds.push(Addressed {
                            ship: me,
                            cmd: Command::Dock(Some(b)),
                        });
                    } else {
                        nav(b, &mut d.cmds);
                    }
                }
            }
            Some(Goal::Scan(_)) => {
                let done = ops.map(|x| x.scan.done.clone()).unwrap_or_default();
                let cand = nearest_where(&|x| {
                    matches!(
                        x.kind,
                        Kind::Asteroid | Kind::Moon | Kind::Planet | Kind::Station
                    )
                });
                let pick = w
                    .iter()
                    .filter(|(i, x)| {
                        *i != me
                            && !done.contains(i)
                            && matches!(x.kind, Kind::Asteroid | Kind::Moon | Kind::Station)
                    })
                    .min_by(|a, b| {
                        a.1.pos
                            .distance(pos)
                            .total_cmp(&b.1.pos.distance(pos))
                            .then(a.0.cmp(&b.0))
                    })
                    .map(|(i, _)| i)
                    .or(cand.map(|c| c.0));
                if let Some(t) = pick
                    && ops.is_some_and(|x| x.scan.target != Some(t))
                {
                    d.cmds.push(Addressed {
                        ship: me,
                        cmd: Command::Scan(Some(t)),
                    });
                }
            }
            Some(Goal::Cargo { commodity, .. }) if commodity == ORE => {
                if ops.is_some_and(|x| x.mining.is_none())
                    && let Some((rock, rpos, rr)) = nearest_where(&|x| x.kind == Kind::Asteroid)
                {
                    let gap = rpos.distance(me_e.pos) - rr - me_e.radius;
                    if gap < 550.0 && me_e.vel.length() < 40.0 {
                        d.cmds.push(Addressed {
                            ship: me,
                            cmd: Command::Mine(Some(rock)),
                        });
                    } else {
                        nav(rock, &mut d.cmds);
                    }
                }
            }
            Some(Goal::Escort { dest, .. }) => {
                let hauler = nearest_where(&|x| x.label.starts_with("Hauler")).map(|h| h.0);
                if let Some(b) = hauler.or_else(|| find(dest)) {
                    nav(b, &mut d.cmds);
                }
            }
            Some(
                Goal::Wait(_)
                | Goal::Cargo { .. }
                | Goal::Clear
                | Goal::ClearBut(_)
                | Goal::Gone(_)
                | Goal::Disabled(_)
                | Goal::Board { .. },
            )
            | None => {
                // nothing to shoot: run down the prey (it may still be drifting or fleeing)
                if let Some((_, p)) = w.iter().find(|(i, x)| prefix_of(*i) && x.ops.is_some()) {
                    let to = p.pos - me_e.pos;
                    let top = me_e.ship.map_or(300.0, |s| s.class.max_speed);
                    if ops.is_some_and(|x| x.nav.autopilot) {
                        d.cmds.push(Addressed {
                            ship: me,
                            cmd: Command::Autopilot(false),
                        });
                    }
                    d.cmds.push(Addressed {
                        ship: me,
                        cmd: steer(me_e, to),
                    });
                    d.cmds.push(Addressed {
                        ship: me,
                        cmd: Command::SetThrottle(
                            (p.vel.length() / top + (to.length() - 300.0) / 1500.0).clamp(0.0, 1.0),
                        ),
                    });
                    d.cmds.push(Addressed {
                        ship: me,
                        cmd: Command::Brake(false),
                    });
                }
            }
        }
        let _ = Vec3::ZERO;
        d
    }
}
