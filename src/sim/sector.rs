//! Sol-system sectors (T06): local bodies as entities, the rest of the system as distant sky.
//!
//! Local scale is compressed: a ship is ~40 units long, planets are thousands of units across.
//! Scenery planets (Sun, Mercury … Neptune) are placed from a simple heliocentric table and drawn
//! at infinity with a boosted angular size so they stay readable.

use std::f32::consts::TAU;

use glam::Vec3;

use super::event::Event;
use super::flight::{HAULER, LONGSHIP, SKIFF, ShipClass, ShipState};
use super::world::{Entity, EntityId, Kind, Orbit, World};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SectorId {
    MarsOrbit,
    Phobos,
    Belt,
    Jupiter,
    Luna,
    EarthOrbit,
}

impl SectorId {
    pub const ALL: [SectorId; 6] = [
        SectorId::MarsOrbit,
        SectorId::Phobos,
        SectorId::Belt,
        SectorId::Jupiter,
        SectorId::Luna,
        SectorId::EarthOrbit,
    ];

    pub fn name(self) -> &'static str {
        match self {
            SectorId::MarsOrbit => "Mars Orbit",
            SectorId::Phobos => "Phobos / Deimos",
            SectorId::Belt => "Asteroid Belt",
            SectorId::Jupiter => "Jupiter / Ganymede",
            SectorId::Luna => "Luna",
            SectorId::EarthOrbit => "Earth Orbit",
        }
    }

    /// Sector flown in campaign mission `mission` (1-based; placeholder routing until T25).
    pub fn for_mission(mission: u8) -> SectorId {
        match mission {
            0 | 1 => SectorId::MarsOrbit,
            2 => SectorId::Phobos,
            3 | 4 => SectorId::Belt,
            5 | 6 => SectorId::Jupiter,
            7 | 8 => SectorId::Luna,
            _ => SectorId::EarthOrbit,
        }
    }

    /// Parse a CLI-style name (`mars`, `belt`, …).
    pub fn parse(s: &str) -> Option<SectorId> {
        match s.to_ascii_lowercase().as_str() {
            "mars" => Some(SectorId::MarsOrbit),
            "phobos" => Some(SectorId::Phobos),
            "belt" => Some(SectorId::Belt),
            "jupiter" => Some(SectorId::Jupiter),
            "luna" => Some(SectorId::Luna),
            "earth" => Some(SectorId::EarthOrbit),
            _ => None,
        }
    }

    /// Heliocentric (distance in AU, phase angle) of the sector's host body.
    fn helio(self) -> (f32, f32) {
        match self {
            SectorId::MarsOrbit | SectorId::Phobos => (1.52, 0.6),
            SectorId::Belt => (2.8, 2.4),
            SectorId::Jupiter => (5.2, 4.1),
            SectorId::Luna | SectorId::EarthOrbit => (1.0, 5.2),
        }
    }

    /// Scenery bodies hosted locally (so they are not also drawn as sky).
    fn host(self) -> &'static str {
        match self {
            SectorId::MarsOrbit | SectorId::Phobos => "Mars",
            SectorId::Belt => "",
            SectorId::Jupiter => "Jupiter",
            SectorId::Luna | SectorId::EarthOrbit => "Earth",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkyKind {
    Star,
    Planet,
}

/// A body at infinity: direction from the player and angular radius (radians).
#[derive(Debug, Clone, Copy)]
pub struct SkyBody {
    pub name: &'static str,
    pub texture: &'static str,
    pub kind: SkyKind,
    pub dir: Vec3,
    pub ang_radius: f32,
}

/// (name, texture, orbit radius AU, body radius km, phase rad)
const SOL: [(&str, &str, f32, f32, f32); 8] = [
    ("Mercury", "mercury", 0.39, 2440.0, 1.1),
    ("Venus", "venus", 0.72, 6052.0, 3.3),
    ("Earth", "earth", 1.0, 6371.0, 5.2),
    ("Mars", "mars", 1.52, 3390.0, 0.6),
    ("Jupiter", "jupiter", 5.2, 69911.0, 4.1),
    ("Saturn", "saturn", 9.54, 58232.0, 2.0),
    ("Uranus", "uranus", 19.2, 25362.0, 0.2),
    ("Neptune", "neptune", 30.1, 24622.0, 3.9),
];

const KM_PER_AU: f32 = 149_597_870.0;
const SUN_RADIUS_KM: f32 = 696_000.0;
/// Artistic boost so distant discs read at game scale.
const SKY_BOOST: f32 = 7.0;
const SKY_MIN: f32 = 0.006;

fn helio_pos(a: f32, phase: f32) -> Vec3 {
    Vec3::new(a * phase.cos(), 0.0, a * phase.sin()) * KM_PER_AU
}

fn sky_for(id: SectorId) -> Vec<SkyBody> {
    let (a, ph) = id.helio();
    let me = helio_pos(a, ph);
    let mut sky = Vec::new();
    let to_sun = -me;
    sky.push(SkyBody {
        name: "Sun",
        texture: "sun",
        kind: SkyKind::Star,
        dir: to_sun.normalize(),
        ang_radius: ((SUN_RADIUS_KM / to_sun.length()).asin() * 6.0).max(0.012),
    });
    for (name, tex, pa, r_km, pph) in SOL {
        if name == id.host() {
            continue;
        }
        let d = helio_pos(pa, pph) - me;
        if d.length() < 1.0 {
            continue;
        }
        sky.push(SkyBody {
            name,
            texture: tex,
            kind: SkyKind::Planet,
            dir: d.normalize(),
            ang_radius: ((r_km / d.length()).asin() * SKY_BOOST).max(SKY_MIN),
        });
    }
    match id {
        SectorId::Luna => sky.push(SkyBody {
            name: "Earth",
            texture: "earth",
            kind: SkyKind::Planet,
            dir: Vec3::new(-0.5, 0.2, -0.84).normalize(),
            ang_radius: 0.09,
        }),
        SectorId::EarthOrbit => sky.push(SkyBody {
            name: "Luna",
            texture: "luna",
            kind: SkyKind::Planet,
            dir: Vec3::new(0.7, 0.15, -0.7).normalize(),
            ang_radius: 0.03,
        }),
        _ => {}
    }
    sky
}

impl World {
    /// Direction toward the sun for lighting.
    pub fn sun_dir(&self) -> Vec3 {
        self.sky
            .iter()
            .find(|s| s.kind == SkyKind::Star)
            .map_or(Vec3::new(0.4, 0.5, 0.8).normalize(), |s| s.dir)
    }
}

fn body(
    w: &mut World,
    kind: Kind,
    label: &'static str,
    (model, tex): (&'static str, &'static str),
    pos: Vec3,
    radius: f32,
    spin: f32,
) -> EntityId {
    let mut e = Entity::new(kind, label, model, tex, radius);
    e.pos = pos;
    e.spin = spin;
    w.spawn(e)
}

/// Put `id` on a circular orbit around `center` through its current position at `speed`.
fn orbit_to(w: &mut World, id: EntityId, center: EntityId, speed: f32) {
    let (Some(c), Some(p)) = (w.get(center).map(|e| e.pos), w.get(id).map(|e| e.pos)) else {
        return;
    };
    let r = p - c;
    let radius = r.length().max(1.0);
    let normal = r
        .normalize()
        .cross(Vec3::new(0.25, 1.0, 0.1).normalize())
        .normalize_or_zero();
    let normal = if normal == Vec3::ZERO {
        Vec3::Y
    } else {
        normal
    };
    let u = normal.any_orthonormal_vector();
    let v = normal.cross(u);
    let phase = r.dot(v).atan2(r.dot(u));
    if let Some(e) = w.get_mut(id) {
        e.orbit = Some(Orbit {
            center,
            radius,
            period: TAU * radius / speed.max(1.0),
            phase,
            normal,
        });
    }
}

fn ship(w: &mut World, label: &'static str, class: ShipClass, pos: Vec3, rot_y: f32) -> EntityId {
    let mut e = Entity::new(Kind::Ship, label, class.model, class.texture, class.radius);
    e.pos = pos;
    e.rot = glam::Quat::from_rotation_y(rot_y);
    e.ship = Some(ShipState::new(class));
    w.spawn(e)
}

fn asteroids(w: &mut World, count: usize, inner: f32, outer: f32, squash: f32, keep_clear: Vec3) {
    const MODELS: [(&str, &str); 3] = [
        ("asteroid_a", "asteroid_a"),
        ("asteroid_b", "asteroid_b"),
        ("asteroid_c", "asteroid_c"),
    ];
    for _ in 0..count {
        let dir = w.rng.unit_vec();
        let dist = w.rng.range(inner, outer);
        let pos = Vec3::new(dir.x, dir.y * squash, dir.z) * dist;
        let radius = w.rng.range(14.0, 60.0) * if w.rng.f32() > 0.93 { 3.5 } else { 1.0 };
        if pos.distance(keep_clear) < 400.0 + radius {
            continue;
        }
        let (model, tex) = MODELS[w.rng.below(3) as usize];
        let spin = w.rng.range(-0.25, 0.25);
        let id = body(
            w,
            Kind::Asteroid,
            "Asteroid",
            (model, tex),
            pos,
            radius,
            spin,
        );
        let axis = w.rng.unit_vec();
        let ang = w.rng.range(0.0, TAU);
        if let Some(e) = w.get_mut(id) {
            e.rot = glam::Quat::from_axis_angle(axis, ang);
            e.prev_rot = e.rot;
        }
    }
}

/// Build a fresh world for `id` with the player's longship *Huginn* near the origin.
pub fn new_game(seed: u64, id: SectorId) -> World {
    let mut w = World::new(seed);
    w.sector = id;
    w.sky = sky_for(id);
    match id {
        SectorId::MarsOrbit => {
            let mars = body(
                &mut w,
                Kind::Planet,
                "Mars",
                ("sphere", "mars"),
                Vec3::new(-3500.0, -1500.0, -16000.0),
                6000.0,
                0.004,
            );
            let forge = body(
                &mut w,
                Kind::Station,
                "Forge",
                ("forge", "station_forge"),
                Vec3::new(900.0, 150.0, -2200.0),
                150.0,
                0.02,
            );
            let phobos = body(
                &mut w,
                Kind::Moon,
                "Phobos",
                ("asteroid_a", "phobos"),
                Vec3::new(3200.0, 500.0, -9000.0),
                140.0,
                0.03,
            );
            let deimos = body(
                &mut w,
                Kind::Moon,
                "Deimos",
                ("asteroid_b", "deimos"),
                Vec3::new(-12000.0, 2500.0, -22000.0),
                90.0,
                0.02,
            );
            orbit_to(&mut w, forge, mars, 55.0);
            orbit_to(&mut w, phobos, mars, 45.0);
            orbit_to(&mut w, deimos, mars, 35.0);
            for (label, class, off, ry) in [
                ("Skiff Rask", SKIFF, Vec3::new(-260.0, 60.0, -300.0), 0.5),
                ("Hauler Ymir", HAULER, Vec3::new(380.0, -90.0, 200.0), 2.0),
                ("Hauler Skuld", HAULER, Vec3::new(600.0, 220.0, -500.0), 1.2),
            ] {
                let pos = Vec3::new(900.0, 150.0, -2200.0) + off;
                let s = ship(&mut w, label, class, pos, ry);
                orbit_to(&mut w, s, mars, 55.0);
                if let Some(e) = w.get_mut(s) {
                    e.ship = None; // scenery traffic follows its orbit
                }
            }
        }
        SectorId::Phobos => {
            let mars = body(
                &mut w,
                Kind::Planet,
                "Mars",
                ("sphere", "mars"),
                Vec3::new(-15000.0, 2000.0, -5000.0),
                6000.0,
                0.004,
            );
            let phobos = body(
                &mut w,
                Kind::Moon,
                "Phobos",
                ("asteroid_a", "phobos"),
                Vec3::new(1400.0, -100.0, -3600.0),
                600.0,
                0.01,
            );
            let deimos = body(
                &mut w,
                Kind::Moon,
                "Deimos",
                ("asteroid_b", "deimos"),
                Vec3::new(-7000.0, 1500.0, -9500.0),
                280.0,
                0.01,
            );
            let relay = body(
                &mut w,
                Kind::Station,
                "Phobos Relay",
                ("forge", "station_forge"),
                Vec3::new(1400.0, 800.0, -3600.0),
                150.0,
                0.03,
            );
            orbit_to(&mut w, phobos, mars, 30.0);
            orbit_to(&mut w, deimos, mars, 24.0);
            orbit_to(&mut w, relay, phobos, 25.0);
            asteroids(&mut w, 40, 2500.0, 5000.0, 0.3, Vec3::ZERO);
        }
        SectorId::Belt => {
            body(
                &mut w,
                Kind::Asteroid,
                "Ceres",
                ("asteroid_a", "asteroid_a"),
                Vec3::new(1800.0, -300.0, -4200.0),
                1500.0,
                0.004,
            );
            body(
                &mut w,
                Kind::Station,
                "Belt Outpost",
                ("forge", "station_forge"),
                Vec3::new(500.0, 60.0, -1800.0),
                150.0,
                0.03,
            );
            asteroids(&mut w, 360, 600.0, 7500.0, 0.35, Vec3::ZERO);
        }
        SectorId::Jupiter => {
            let jup = body(
                &mut w,
                Kind::Planet,
                "Jupiter",
                ("sphere", "jupiter"),
                Vec3::new(9000.0, 2500.0, -42000.0),
                16000.0,
                0.01,
            );
            let gany = body(
                &mut w,
                Kind::Moon,
                "Ganymede",
                ("sphere", "luna"),
                Vec3::new(-1500.0, 300.0, -7500.0),
                1800.0,
                0.006,
            );
            let port = body(
                &mut w,
                Kind::Station,
                "Ganymede Port",
                ("forge", "station_forge"),
                Vec3::new(-1500.0, 300.0, -4000.0),
                150.0,
                0.03,
            );
            orbit_to(&mut w, gany, jup, 70.0);
            orbit_to(&mut w, port, gany, 22.0);
        }
        SectorId::Luna => {
            let luna = body(
                &mut w,
                Kind::Moon,
                "Luna",
                ("sphere", "luna"),
                Vec3::new(2500.0, -1200.0, -9000.0),
                3400.0,
                0.003,
            );
            let fort = body(
                &mut w,
                Kind::Station,
                "Tycho Fort",
                ("fort", "station_fort"),
                Vec3::new(500.0, 200.0, -2000.0),
                170.0,
                0.02,
            );
            orbit_to(&mut w, fort, luna, 30.0);
        }
        SectorId::EarthOrbit => {
            let earth = body(
                &mut w,
                Kind::Planet,
                "Earth",
                ("sphere", "earth"),
                Vec3::new(2000.0, -2500.0, -22000.0),
                8500.0,
                0.006,
            );
            let f1 = body(
                &mut w,
                Kind::Station,
                "Concord Fort Alpha",
                ("fort", "station_fort"),
                Vec3::new(700.0, 100.0, -2600.0),
                170.0,
                0.02,
            );
            let f2 = body(
                &mut w,
                Kind::Station,
                "Concord Fort Beta",
                ("fort", "station_fort"),
                Vec3::new(-4200.0, 900.0, -9000.0),
                170.0,
                0.02,
            );
            let dock = body(
                &mut w,
                Kind::Station,
                "Geneva Dock",
                ("forge", "station_forge"),
                Vec3::new(5200.0, -300.0, -7000.0),
                150.0,
                0.03,
            );
            for s in [f1, f2, dock] {
                orbit_to(&mut w, s, earth, 60.0);
            }
        }
    }
    let spawn_pos = Vec3::ZERO;
    let player = ship(&mut w, "Huginn", LONGSHIP, spawn_pos, 0.0);
    w.player = Some(player);
    w.push_event(Event::SectorLoaded(id));
    w
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::command::{Addressed, Command};

    #[test]
    fn every_sector_loads_with_player_sun_and_sky() {
        for id in SectorId::ALL {
            let w = new_game(1, id);
            assert!(w.player.and_then(|p| w.get(p)).is_some(), "{id:?}");
            assert!(w.sky.iter().any(|s| s.kind == SkyKind::Star));
            assert!(w.iter().filter(|(_, e)| e.kind != Kind::Ship).count() >= 2);
            assert!((w.sun_dir().length() - 1.0).abs() < 1e-4);
            for (_, e) in w.iter() {
                assert!(e.pos.is_finite() && e.radius > 0.0);
            }
        }
    }

    #[test]
    fn orbits_start_where_placed_and_keep_radius() {
        let mut w = new_game(1, SectorId::MarsOrbit);
        let forge = w.iter().find(|(_, e)| e.label == "Forge").map(|(i, _)| i);
        let forge = forge.expect("forge exists");
        let p0 = w.get(forge).map(|e| e.pos);
        w.step(&[]);
        let p1 = w.get(forge).map(|e| e.pos);
        let moved = p0.zip(p1).map(|(a, b)| a.distance(b));
        assert!(moved.is_some_and(|d| d < 5.0), "forge jumped {moved:?}");
        let mars = w.iter().find(|(_, e)| e.label == "Mars").map(|(i, _)| i);
        let mars_pos = mars.and_then(|m| w.get(m)).map(|e| e.pos);
        let r0 = p0.zip(mars_pos).map(|(a, b)| a.distance(b));
        for _ in 0..3000 {
            w.step(&[]);
        }
        let r1 = w
            .get(forge)
            .map(|e| e.pos)
            .zip(mars_pos)
            .map(|(a, b)| a.distance(b));
        assert!((r0.unwrap_or(0.0) - r1.unwrap_or(1.0)).abs() < 1.0);
    }

    #[test]
    fn determinism_same_seed_same_commands_same_hash() {
        let run = || {
            let mut w = new_game(42, SectorId::Belt);
            let me = w.player.expect("player");
            for i in 0..10_000u32 {
                let cmds = [
                    Addressed {
                        ship: me,
                        cmd: Command::SetThrottle(0.6),
                    },
                    Addressed {
                        ship: me,
                        cmd: Command::Turn {
                            pitch: 0.2,
                            yaw: if i % 200 < 100 { 0.5 } else { -0.5 },
                            roll: 0.1,
                        },
                    },
                ];
                w.step(&cmds);
            }
            w.state_hash()
        };
        assert_eq!(run(), run());
        let mut other = new_game(43, SectorId::Belt);
        other.step(&[]);
        assert_ne!(run(), other.state_hash());
    }

    #[test]
    fn ship_cannot_fly_through_a_planet() {
        let mut w = new_game(1, SectorId::Luna);
        let me = w.player.expect("player");
        let luna = w
            .iter()
            .find(|(_, e)| e.label == "Luna")
            .map(|(i, _)| i)
            .expect("luna");
        if let Some(e) = w.get_mut(me) {
            e.pos = Vec3::new(2500.0, -1200.0, -9000.0 + 3400.0 + 400.0);
            e.vel = Vec3::new(0.0, 0.0, -300.0);
        }
        let mut hit = false;
        for _ in 0..600 {
            hit |= w
                .step(&[])
                .iter()
                .any(|e| matches!(e, Event::Collision { .. }));
        }
        let (p, l) = (w.get(me).map(|e| e.pos), w.get(luna).map(|e| e.pos));
        let d = p.zip(l).map(|(a, b)| a.distance(b)).unwrap_or(0.0);
        assert!(hit && d >= 3400.0 + 22.0 - 1.0, "dist {d}");
    }

    #[test]
    fn stale_ids_do_not_resolve() {
        let mut w = World::new(1);
        let a = w.spawn(Entity::new(
            Kind::Asteroid,
            "a",
            "asteroid_a",
            "asteroid_a",
            1.0,
        ));
        assert!(w.despawn(a));
        let b = w.spawn(Entity::new(
            Kind::Asteroid,
            "b",
            "asteroid_a",
            "asteroid_a",
            1.0,
        ));
        assert_eq!(a.index, b.index);
        assert!(w.get(a).is_none() && w.get(b).is_some());
    }
}
