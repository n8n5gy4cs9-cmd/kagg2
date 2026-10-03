//! Entity arena with generational ids, plus the fixed-step `World::step`.

use std::f32::consts::TAU;

use glam::{Quat, Vec3};

use super::clock::DT;
use super::command::{Addressed, Command};
use super::event::Event;
use super::flight::{self, ShipState};
use super::rng::Rng;
use super::sector::{SectorId, SkyBody};

/// Stable handle to an entity; stale handles (despawned slots) never resolve.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EntityId {
    pub index: u32,
    pub generation: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Ship,
    Planet,
    Moon,
    Asteroid,
    Station,
}

/// Circular orbit around another entity.
#[derive(Debug, Clone, Copy)]
pub struct Orbit {
    pub center: EntityId,
    pub radius: f32,
    pub period: f32,
    pub phase: f32,
    pub normal: Vec3,
}

#[derive(Debug, Clone)]
pub struct Entity {
    pub kind: Kind,
    pub label: &'static str,
    /// Mesh name from `meshgen` and texture name from `texgen`.
    pub model: &'static str,
    pub texture: &'static str,
    pub pos: Vec3,
    pub vel: Vec3,
    pub rot: Quat,
    /// Angular velocity in the body frame (rad/s).
    pub ang_vel: Vec3,
    /// Spin about the local Y axis (rad/s) for scenery.
    pub spin: f32,
    pub radius: f32,
    pub ship: Option<ShipState>,
    pub orbit: Option<Orbit>,
    /// Previous-step transform for render interpolation.
    pub prev_pos: Vec3,
    pub prev_rot: Quat,
}

impl Entity {
    pub fn new(
        kind: Kind,
        label: &'static str,
        model: &'static str,
        texture: &'static str,
        radius: f32,
    ) -> Entity {
        Entity {
            kind,
            label,
            model,
            texture,
            pos: Vec3::ZERO,
            vel: Vec3::ZERO,
            rot: Quat::IDENTITY,
            ang_vel: Vec3::ZERO,
            spin: 0.0,
            radius,
            ship: None,
            orbit: None,
            prev_pos: Vec3::ZERO,
            prev_rot: Quat::IDENTITY,
        }
    }

    pub fn forward(&self) -> Vec3 {
        self.rot * flight::FORWARD
    }
}

#[derive(Debug)]
struct Slot {
    generation: u32,
    ent: Option<Entity>,
}

#[derive(Debug)]
pub struct World {
    slots: Vec<Slot>,
    free: Vec<u32>,
    pub rng: Rng,
    pub tick: u64,
    pub time: f32,
    pub player: Option<EntityId>,
    pub sector: SectorId,
    /// Distant scenery (sun, far planets) drawn at infinity.
    pub sky: Vec<SkyBody>,
    events: Vec<Event>,
}

impl World {
    pub fn new(seed: u64) -> World {
        World {
            slots: Vec::new(),
            free: Vec::new(),
            rng: Rng::new(seed),
            tick: 0,
            time: 0.0,
            player: None,
            sector: SectorId::MarsOrbit,
            sky: Vec::new(),
            events: Vec::new(),
        }
    }

    pub fn spawn(&mut self, mut ent: Entity) -> EntityId {
        ent.prev_pos = ent.pos;
        ent.prev_rot = ent.rot;
        let id = match self.free.pop() {
            Some(index) => {
                let slot = &mut self.slots[index as usize];
                slot.ent = Some(ent);
                EntityId {
                    index,
                    generation: slot.generation,
                }
            }
            None => {
                self.slots.push(Slot {
                    generation: 0,
                    ent: Some(ent),
                });
                EntityId {
                    index: self.slots.len() as u32 - 1,
                    generation: 0,
                }
            }
        };
        self.events.push(Event::Spawned(id));
        id
    }

    pub fn despawn(&mut self, id: EntityId) -> bool {
        match self.slots.get_mut(id.index as usize) {
            Some(slot) if slot.generation == id.generation && slot.ent.is_some() => {
                slot.ent = None;
                slot.generation += 1;
                self.free.push(id.index);
                self.events.push(Event::Despawned(id));
                true
            }
            _ => false,
        }
    }

    pub fn get(&self, id: EntityId) -> Option<&Entity> {
        self.slots
            .get(id.index as usize)
            .filter(|s| s.generation == id.generation)
            .and_then(|s| s.ent.as_ref())
    }

    pub fn get_mut(&mut self, id: EntityId) -> Option<&mut Entity> {
        self.slots
            .get_mut(id.index as usize)
            .filter(|s| s.generation == id.generation)
            .and_then(|s| s.ent.as_mut())
    }

    /// Live entities in index order (deterministic).
    pub fn iter(&self) -> impl Iterator<Item = (EntityId, &Entity)> {
        self.slots.iter().enumerate().filter_map(|(i, s)| {
            s.ent.as_ref().map(|e| {
                (
                    EntityId {
                        index: i as u32,
                        generation: s.generation,
                    },
                    e,
                )
            })
        })
    }

    pub fn len(&self) -> usize {
        self.slots.iter().filter(|s| s.ent.is_some()).count()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn push_event(&mut self, e: Event) {
        self.events.push(e);
    }

    /// Take the events accumulated since the last call.
    pub fn drain_events(&mut self) -> Vec<Event> {
        std::mem::take(&mut self.events)
    }

    /// Entities nearest to `from` (excluding it), at most `n`, optionally within `range`.
    pub fn nearest(&self, from: EntityId, n: usize, range: f32) -> Vec<EntityId> {
        let Some(origin) = self.get(from).map(|e| e.pos) else {
            return Vec::new();
        };
        let mut v: Vec<(f32, EntityId)> = self
            .iter()
            .filter(|(id, _)| *id != from)
            .map(|(id, e)| (e.pos.distance(origin), id))
            .filter(|(d, _)| *d <= range)
            .collect();
        v.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
        v.into_iter().take(n).map(|(_, id)| id).collect()
    }

    /// Advance one fixed step with the given commands.
    pub fn step(&mut self, cmds: &[Addressed]) -> Vec<Event> {
        for slot in &mut self.slots {
            if let Some(e) = slot.ent.as_mut() {
                e.prev_pos = e.pos;
                e.prev_rot = e.rot;
            }
        }
        for a in cmds {
            if let Some(ship) = self.get_mut(a.ship).and_then(|e| e.ship.as_mut()) {
                ship.apply(a.cmd);
            }
        }
        self.time += DT;
        self.tick += 1;
        self.step_orbits();
        for slot in &mut self.slots {
            if let Some(e) = slot.ent.as_mut() {
                if e.ship.is_some() {
                    flight::integrate(e, DT);
                } else if e.orbit.is_none() {
                    e.pos += e.vel * DT;
                }
                if e.spin != 0.0 {
                    e.rot = (e.rot * Quat::from_rotation_y(e.spin * DT)).normalize();
                }
            }
        }
        self.resolve_collisions();
        self.drain_events()
    }

    fn step_orbits(&mut self) {
        let t = self.time;
        let updates: Vec<(usize, Vec3, Vec3)> = self
            .iter()
            .filter_map(|(id, e)| {
                let o = e.orbit?;
                let c = self.get(o.center)?.pos;
                let n = o.normal.normalize_or_zero();
                let u = n.any_orthonormal_vector();
                let v = n.cross(u);
                let w = TAU / o.period;
                let a = o.phase + w * t;
                let pos = c + (u * a.cos() + v * a.sin()) * o.radius;
                let vel = (-u * a.sin() + v * a.cos()) * (o.radius * w);
                Some((id.index as usize, pos, vel))
            })
            .collect();
        for (i, pos, vel) in updates {
            if let Some(e) = self.slots[i].ent.as_mut() {
                e.pos = pos;
                e.vel = vel;
            }
        }
    }

    /// Ships bounce off solid bodies (sphere approximation).
    fn resolve_collisions(&mut self) {
        let ships: Vec<EntityId> = self
            .iter()
            .filter(|(_, e)| e.ship.is_some())
            .map(|(i, _)| i)
            .collect();
        let solids: Vec<(EntityId, Vec3, f32, Vec3)> = self
            .iter()
            .filter(|(_, e)| e.ship.is_none())
            .map(|(i, e)| (i, e.pos, e.radius, e.vel))
            .collect();
        for sid in ships {
            for &(oid, opos, orad, ovel) in &solids {
                let Some(s) = self.get_mut(sid) else { break };
                let delta = s.pos - opos;
                let min = s.radius + orad;
                let d2 = delta.length_squared();
                if d2 >= min * min {
                    continue;
                }
                let n = if d2 > 1e-6 {
                    delta / d2.sqrt()
                } else {
                    Vec3::Y
                };
                s.pos = opos + n * min;
                let rel = s.vel - ovel;
                let vn = rel.dot(n);
                let speed = vn.abs();
                if vn < 0.0 {
                    s.vel -= n * vn * 1.4; // inelastic bounce
                }
                self.events.push(Event::Collision {
                    ship: sid,
                    other: oid,
                    speed,
                });
            }
        }
    }

    /// Hash of the full dynamic state — equal for identical seeds and command streams.
    pub fn state_hash(&self) -> u64 {
        let mut h = 0xcbf2_9ce4_8422_2325u64;
        let mut mix = |v: f32| {
            h = (h ^ v.to_bits() as u64).wrapping_mul(0x0000_0100_0000_01b3);
        };
        for (_, e) in self.iter() {
            for v in [
                e.pos.x, e.pos.y, e.pos.z, e.vel.x, e.vel.y, e.vel.z, e.rot.x, e.rot.y, e.rot.z,
                e.rot.w,
            ] {
                mix(v);
            }
        }
        h
    }

    /// Convenience for UI code: queue-free direct command to the player's ship.
    pub fn player_cmd(&self, cmd: Command) -> Option<Addressed> {
        self.player.map(|ship| Addressed { ship, cmd })
    }
}
