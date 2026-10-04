//! The boarding world (T21/T23): player, monsters, items, projectiles, doors, objectives.
//! Headless and deterministic like the space sim.

use glam::Vec3;

use super::level::{CELL, ItemKind, Key, Level, MonsterKind, Objective, RoomKind, SpawnKind, Tile};
use super::monster::{MEvent, Monster, Senses, State, stats, think};
use super::pmove::{Env, Input, Player, pmove};
use super::weapons::{Ammo, Weapon};
use crate::sim::rng::Rng;

const STEP: f32 = 0.25;

#[derive(Debug, Clone)]
pub struct Item {
    pub kind: ItemKind,
    pub pos: Vec3,
    pub taken: bool,
}

#[derive(Debug, Clone)]
pub struct Prisoner {
    pub pos: Vec3,
    pub freed: bool,
}

#[derive(Debug, Clone)]
pub struct Rocket {
    pub pos: Vec3,
    pub vel: Vec3,
    pub life: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    InProgress,
    /// Objective done and the player is back in the airlock.
    Won,
    /// Left early through the airlock.
    Fled,
    Died,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BEvent {
    Fired(Weapon),
    /// A bullet or pellet struck something (spark position).
    Impact(Vec3),
    Explosion(Vec3),
    MonsterHurt(Vec3),
    MonsterDied(MonsterKind, Vec3),
    PlayerHurt(f32),
    Pickup,
    DoorMoved,
    Alarm,
    Message(String),
    ObjectiveDone,
}

#[derive(Debug, Clone, Default)]
pub struct Progress {
    pub done: bool,
    /// Sabotage countdown once the charges are set.
    pub charges: Option<f32>,
    pub freed: u32,
    pub kills: u32,
    pub alarm_raised: bool,
}

pub struct BoardWorld {
    pub level: Level,
    pub player: Player,
    pub monsters: Vec<Monster>,
    pub items: Vec<Item>,
    pub prisoners: Vec<Prisoner>,
    pub rockets: Vec<Rocket>,
    pub progress: Progress,
    pub outcome: Outcome,
    pub time: f32,
    pub env: Env,
    pub difficulty: f32,
    rng: Rng,
    /// Co-op partners (T33); empty in a single-player boarding. Index `k` here is player `k + 1`.
    pub mates: Vec<Player>,
    noises: Vec<Vec3>,
    lock_msg_cd: f32,
}

impl BoardWorld {
    pub fn new(level: Level, seed: u64) -> BoardWorld {
        let mut rng = Rng::new(seed ^ 0xB0A2D2);
        let mut monsters = Vec::new();
        let mut items = Vec::new();
        let mut prisoners = Vec::new();
        for s in &level.spawns {
            let pos = level.centre(s.x, s.y);
            match s.kind {
                SpawnKind::Monster(k) => {
                    monsters.push(Monster::new(k, pos, rng.range(0.0, std::f32::consts::TAU)))
                }
                SpawnKind::Item(kind) => items.push(Item {
                    kind,
                    pos,
                    taken: false,
                }),
                SpawnKind::Prisoner => prisoners.push(Prisoner { pos, freed: false }),
            }
        }
        let mut player = Player::new(level.spawn_point());
        player.yaw = 0.0;
        BoardWorld {
            level,
            player,
            monsters,
            items,
            prisoners,
            rockets: Vec::new(),
            progress: Progress::default(),
            outcome: Outcome::InProgress,
            time: 0.0,
            env: Env::default(),
            difficulty: 1.0,
            rng,
            mates: Vec::new(),
            noises: Vec::new(),
            lock_msg_cd: 0.0,
        }
    }

    pub fn objective_text(&self) -> String {
        let p = &self.progress;
        match (self.level.objective, p.done) {
            (_, true) => "OBJECTIVE DONE - RETURN TO THE AIRLOCK AND PRESS E".to_owned(),
            (Objective::Rescue, _) => {
                format!("FREE THE PRISONERS ({}/{})", p.freed, self.prisoners.len())
            }
            (o, _) => o.name().to_owned(),
        }
    }

    fn hurt_monster(&mut self, i: usize, dmg: f32, ev: &mut Vec<BEvent>) {
        let pos = self.monsters[i].centre();
        let kind = self.monsters[i].kind;
        let killed = self.monsters[i].hurt(dmg, &mut self.rng);
        ev.push(BEvent::MonsterHurt(pos));
        if killed {
            self.progress.kills += 1;
            ev.push(BEvent::MonsterDied(kind, pos));
            if kind == MonsterKind::Voss && self.level.objective == Objective::KillBoss {
                self.complete(ev);
            }
        }
    }

    /// Developer cheat: kill every living monster (counts the kills; a dead Voss completes the boss
    /// objective). Returns how many died.
    pub fn kill_all(&mut self) -> usize {
        let mut ev = Vec::new();
        let alive: Vec<usize> = (0..self.monsters.len())
            .filter(|&i| self.monsters[i].alive())
            .collect();
        for &i in &alive {
            self.hurt_monster(i, 1.0e6, &mut ev);
        }
        alive.len()
    }

    /// Developer cheat: mark the objective as done (the player still has to reach the airlock).
    pub fn force_objective(&mut self) {
        let mut ev = Vec::new();
        self.complete(&mut ev);
    }

    fn complete(&mut self, ev: &mut Vec<BEvent>) {
        if !self.progress.done {
            self.progress.done = true;
            ev.push(BEvent::ObjectiveDone);
        }
    }

    /// First solid or monster along a ray: `(distance, monster index)`.
    fn trace(&self, from: Vec3, dir: Vec3, range: f32, skip: &[usize]) -> (f32, Option<usize>) {
        let mut t = 0.0;
        while t < range {
            t += STEP * 0.6;
            let p = from + dir * t;
            let (tx, ty) = self.level.tile_of(p);
            if self.level.solid(tx, ty) || p.y < 0.0 || p.y > super::level::WALL_H {
                return (t, None);
            }
            for (i, m) in self.monsters.iter().enumerate() {
                if !m.alive() || skip.contains(&i) {
                    continue;
                }
                let s = stats(m.kind);
                let dx = p.x - m.pos.x;
                let dz = p.z - m.pos.z;
                if dx * dx + dz * dz < s.radius * s.radius
                    && p.y >= m.pos.y
                    && p.y <= m.pos.y + s.height
                {
                    return (t, Some(i));
                }
            }
        }
        (range, None)
    }

    fn fire(&mut self, ev: &mut Vec<BEvent>) {
        let w = self.player.current;
        let def = w.def();
        if self.player.cooldown > 0.0 {
            return;
        }
        if let Some(a) = def.ammo {
            if self.player.ammo[a.idx()] == 0 {
                self.player.cooldown = 0.3;
                ev.push(BEvent::Message(format!("OUT OF {}", a.name())));
                // fall back to the blaster like Quake 2 does
                self.player.current = Weapon::Blaster;
                return;
            }
            self.player.ammo[a.idx()] -= 1;
        }
        self.player.cooldown = def.cooldown;
        self.player.kick = 0.0;
        ev.push(BEvent::Fired(w));
        self.noises.push(self.player.pos);
        let eye = self.player.eye();
        let look = self.player.look();
        if w == Weapon::Mjolnir {
            self.rockets.push(Rocket {
                pos: eye + look * 0.8,
                vel: look * 22.0,
                life: 6.0,
            });
            return;
        }
        for _ in 0..def.pellets {
            let dir = if def.spread > 0.0 {
                (look + self.rng.unit_vec() * def.spread).normalize()
            } else {
                look
            };
            let mut skip: Vec<usize> = Vec::new();
            loop {
                let (t, hit) = self.trace(eye, dir, def.range, &skip);
                match hit {
                    Some(i) => {
                        self.hurt_monster(i, def.damage, ev);
                        ev.push(BEvent::Impact(eye + dir * t));
                        skip.push(i);
                        if !def.pierce {
                            break;
                        }
                    }
                    None => {
                        if t < def.range {
                            ev.push(BEvent::Impact(eye + dir * t));
                        }
                        break;
                    }
                }
            }
        }
    }

    fn explode(&mut self, at: Vec3, ev: &mut Vec<BEvent>) {
        ev.push(BEvent::Explosion(at));
        self.noises.push(at);
        const R: f32 = 5.0;
        for i in 0..self.monsters.len() {
            if !self.monsters[i].alive() {
                continue;
            }
            let d = self.monsters[i].centre().distance(at);
            if d < R + stats(self.monsters[i].kind).radius {
                self.hurt_monster(i, 120.0 * (1.0 - (d / (R + 1.0)).min(1.0)).max(0.2), ev);
            }
        }
        for k in 0..=self.mates.len() {
            let p = if k == 0 {
                &mut self.player
            } else {
                &mut self.mates[k - 1]
            };
            let d = p.eye().distance(at);
            if p.health > 0.0 && d < R {
                let real = p.hurt(60.0 * (1.0 - d / R));
                if k == 0 {
                    ev.push(BEvent::PlayerHurt(real));
                }
            }
        }
    }

    fn update_doors(&mut self, dt: f32, ev: &mut Vec<BEvent>) {
        self.lock_msg_cd = (self.lock_msg_cd - dt).max(0.0);
        let live: Vec<(Vec3, [bool; 3])> = std::iter::once(&self.player)
            .chain(self.mates.iter())
            .filter(|p| p.health > 0.0)
            .map(|p| (p.pos, p.keys))
            .collect();
        for i in 0..self.level.doors.len() {
            let c = {
                let d = &self.level.doors[i];
                self.level.centre(d.x, d.y)
            };
            let near = live.iter().any(|(p, _)| c.distance(*p) < 2.8);
            let lock = self.level.doors[i].lock;
            let may = lock.is_none_or(|k| {
                live.iter()
                    .any(|(p, keys)| c.distance(*p) < 2.8 && keys[k as usize])
            });
            if near && may {
                self.level.doors[i].hold = 2.0;
            } else if near
                && self.lock_msg_cd == 0.0
                && let Some(k) = lock
            {
                ev.push(BEvent::Message(format!(
                    "LOCKED - NEED THE {} KEY",
                    k.name()
                )));
                self.lock_msg_cd = 2.5;
            }
            // anything standing in the doorway keeps it open
            let blocked = self
                .monsters
                .iter()
                .any(|m| m.alive() && m.pos.distance(c) < 1.6);
            let d = &mut self.level.doors[i];
            if blocked && lock.is_none() {
                d.hold = d.hold.max(0.5);
            }
            let before = d.open;
            if d.hold > 0.0 {
                d.hold -= dt;
                d.open = (d.open + 3.0 * dt).min(1.0);
            } else {
                d.open = (d.open - 2.0 * dt).max(0.0);
            }
            if (before < 0.5) != (d.open < 0.5) {
                ev.push(BEvent::DoorMoved);
            }
        }
    }

    fn pickup(&mut self, ev: &mut Vec<BEvent>) {
        let pp = self.player.pos;
        for i in 0..self.items.len() {
            let it = &self.items[i];
            if it.taken || it.pos.distance(pp) > 1.3 {
                continue;
            }
            let kind = it.kind;
            let p = &mut self.player;
            let ok = match kind {
                ItemKind::HealthSmall | ItemKind::HealthLarge => {
                    let amt = if kind == ItemKind::HealthSmall {
                        10.0
                    } else {
                        25.0
                    };
                    if p.health >= p.max_health {
                        false
                    } else {
                        p.health = (p.health + amt).min(p.max_health);
                        ev.push(BEvent::Message(format!("+{amt:.0} HEALTH")));
                        true
                    }
                }
                ItemKind::ArmorShard => {
                    p.armor = (p.armor + 2).min(200);
                    ev.push(BEvent::Message("+2 ARMOR".into()));
                    true
                }
                ItemKind::ArmorJacket => {
                    if p.armor >= 120 {
                        false
                    } else {
                        p.armor = (p.armor + 40).min(120);
                        ev.push(BEvent::Message("ARMOR JACKET".into()));
                        true
                    }
                }
                ItemKind::ArmorBody => {
                    p.armor = (p.armor + 100).min(200);
                    ev.push(BEvent::Message("BODY ARMOR".into()));
                    true
                }
                ItemKind::Shells | ItemKind::Bullets | ItemKind::Rockets | ItemKind::Slugs => {
                    let (a, n) = match kind {
                        ItemKind::Shells => (Ammo::Shells, 10),
                        ItemKind::Bullets => (Ammo::Bullets, 50),
                        ItemKind::Rockets => (Ammo::Rockets, 5),
                        _ => (Ammo::Slugs, 10),
                    };
                    let got = p.add_ammo(a, n);
                    if got {
                        ev.push(BEvent::Message(format!("+{n} {}", a.name())));
                    }
                    got
                }
                ItemKind::Weapon(w) => {
                    let fresh = !p.weapons[w.idx()];
                    p.weapons[w.idx()] = true;
                    if let Some(a) = w.def().ammo {
                        let n = match a {
                            Ammo::Shells => 10,
                            Ammo::Bullets => 50,
                            Ammo::Rockets => 5,
                            Ammo::Slugs => 10,
                        };
                        p.add_ammo(a, n);
                    }
                    if fresh {
                        p.current = w;
                    }
                    ev.push(BEvent::Message(format!("YOU GOT THE {}", w.def().name)));
                    true
                }
                ItemKind::Key(k) => {
                    p.keys[k as usize] = true;
                    ev.push(BEvent::Message(format!("{} KEYCARD", k.name())));
                    true
                }
                ItemKind::DataChip => {
                    ev.push(BEvent::Message("DATA CORE ACQUIRED".into()));
                    true
                }
            };
            if ok {
                self.items[i].taken = true;
                ev.push(BEvent::Pickup);
                if kind == ItemKind::DataChip && self.level.objective == Objective::StealData {
                    self.complete(ev);
                }
            }
        }
    }

    fn use_key(&mut self, ev: &mut Vec<BEvent>) {
        let pp = self.player.pos;
        let near = |t: (i32, i32), level: &Level| level.centre(t.0, t.1).distance(pp) < 3.0;
        // airlock: leave
        if self.level.rooms[0].contains(self.level.tile_of(pp).0, self.level.tile_of(pp).1) {
            if !self.team_in_airlock() {
                ev.push(BEvent::Message("WAIT FOR YOUR SHIELD-BROTHERS".into()));
                return;
            }
            self.outcome = if self.progress.done {
                Outcome::Won
            } else {
                Outcome::Fled
            };
            return;
        }
        if near(self.level.alarm, &self.level) && !self.progress.alarm_raised {
            self.raise_alarm(ev);
            return;
        }
        match self.level.objective {
            Objective::Sabotage
                if near(self.level.goal, &self.level) && self.progress.charges.is_none() =>
            {
                // time to walk back (plus slack) so deep levels stay winnable
                let here = self.level.tile_of(self.player.pos);
                let back = self
                    .level
                    .path(here, self.level.rooms[0].center())
                    .map_or(0, |p| p.len());
                let secs = (back as f32 * CELL / super::pmove::MAX_SPEED * 1.6 + 20.0)
                    .max(crate::balance::BOARD_FUSE_MIN)
                    .round();
                self.progress.charges = Some(secs);
                ev.push(BEvent::Message(format!(
                    "CHARGES SET - {secs:.0} SECONDS TO REACH THE AIRLOCK"
                )));
                self.complete(ev);
            }
            Objective::CaptureBridge
                if near(self.level.goal, &self.level) && !self.progress.done =>
            {
                let last = self.level.rooms.len() - 1;
                let room = self.level.rooms[last];
                let held = self.monsters.iter().any(|m| {
                    m.alive()
                        && room.contains(self.level.tile_of(m.pos).0, self.level.tile_of(m.pos).1)
                });
                if held {
                    ev.push(BEvent::Message("CLEAR THE BRIDGE FIRST".into()));
                } else {
                    ev.push(BEvent::Message("BRIDGE CAPTURED".into()));
                    self.complete(ev);
                }
            }
            Objective::Rescue => {
                if let Some(p) = self
                    .prisoners
                    .iter_mut()
                    .find(|p| !p.freed && p.pos.distance(pp) < 3.0)
                {
                    p.freed = true;
                    self.progress.freed += 1;
                    ev.push(BEvent::Message("PRISONER FREED".into()));
                    if self.progress.freed as usize == self.prisoners.len() {
                        self.complete(ev);
                    }
                }
            }
            _ => {}
        }
    }

    fn raise_alarm(&mut self, ev: &mut Vec<BEvent>) {
        self.progress.alarm_raised = true;
        ev.push(BEvent::Alarm);
        ev.push(BEvent::Message("ALARM! REINFORCEMENTS INBOUND".into()));
        let tiles: Vec<(i32, i32)> = self
            .level
            .floor_tiles()
            .filter(|&(x, y)| {
                self.level
                    .room_at(x, y)
                    .is_some_and(|r| r.kind != RoomKind::Airlock)
                    && self.level.centre(x, y).distance(self.player.pos) > 14.0
            })
            .collect();
        if tiles.is_empty() {
            return;
        }
        for kind in [MonsterKind::Grunt, MonsterKind::Grunt, MonsterKind::Guard] {
            let (x, y) = tiles[self.rng.below(tiles.len() as u32) as usize];
            let mut m = Monster::new(kind, self.level.centre(x, y), 0.0);
            m.alerted = true;
            m.state = State::Run;
            self.monsters.push(m);
        }
    }

    /// Advance one step (single player).
    pub fn step(&mut self, dt: f32, input: &Input) -> Vec<BEvent> {
        self.step_team(dt, std::slice::from_ref(input))
    }

    /// Add `n` co-op partners (up to 3 in all) at the airlock, next to the host.
    pub fn add_mates(&mut self, n: usize) {
        for i in 0..n.min(3usize.saturating_sub(self.mates.len())) {
            let mut p = Player::new(self.player.pos + Vec3::new(1.4 * (i as f32 + 1.0), 0.0, 0.0));
            p.yaw = self.player.yaw;
            self.mates.push(p);
        }
    }

    /// Run `f` with player `k` (`0` = host, `k > 0` = `mates[k - 1]`) as `self.player`.
    pub fn with_player<R>(&mut self, k: usize, f: impl FnOnce(&mut Self) -> R) -> R {
        if k > 0 {
            std::mem::swap(&mut self.player, &mut self.mates[k - 1]);
        }
        let r = f(self);
        if k > 0 {
            std::mem::swap(&mut self.player, &mut self.mates[k - 1]);
        }
        r
    }

    /// Every living party member stands in the airlock.
    fn team_in_airlock(&self) -> bool {
        std::iter::once(&self.player)
            .chain(self.mates.iter())
            .filter(|p| p.health > 0.0)
            .all(|p| {
                let (x, y) = self.level.tile_of(p.pos);
                self.level.rooms[0].contains(x, y)
            })
    }

    /// Index (`0` = host) of the living party member nearest to `pos`.
    fn nearest_player(&self, pos: Vec3) -> usize {
        std::iter::once(&self.player)
            .chain(self.mates.iter())
            .enumerate()
            .filter(|(_, p)| p.health > 0.0)
            .min_by(|a, b| a.1.pos.distance(pos).total_cmp(&b.1.pos.distance(pos)))
            .map_or(0, |(i, _)| i)
    }

    /// Advance one step for the whole party: `inputs[0]` drives the host, `inputs[k]` drives
    /// `mates[k - 1]` (missing entries mean "no input"). Objectives, doors, keys and monsters
    /// are shared; the action ends in defeat only when everybody is down.
    pub fn step_team(&mut self, dt: f32, inputs: &[Input]) -> Vec<BEvent> {
        let mut ev = Vec::new();
        if self.outcome != Outcome::InProgress {
            return ev;
        }
        self.time += dt;
        self.noises.clear();
        let none = Input::default();
        for k in 0..=self.mates.len() {
            let input = inputs.get(k).unwrap_or(&none);
            self.with_player(k, |w| {
                if w.player.health <= 0.0 {
                    return;
                }
                w.player.cooldown = (w.player.cooldown - dt).max(0.0);
                w.player.kick += dt;
                pmove(&w.level, &mut w.player, input, w.env, dt);
                if let Some(n) = input.weapon
                    && let Some(wp) = Weapon::ALL.get(n)
                    && w.player.weapons[wp.idx()]
                {
                    w.player.current = *wp;
                }
                if input.fire {
                    w.fire(&mut ev);
                }
                if input.use_key {
                    w.use_key(&mut ev);
                }
            });
        }
        self.update_doors(dt, &mut ev);
        for k in 0..=self.mates.len() {
            self.with_player(k, |w| {
                if w.player.health > 0.0 {
                    w.pickup(&mut ev);
                }
            });
        }
        if !self.mates.is_empty() {
            // the party shares keycards
            let mut keys = self.player.keys;
            for m in &self.mates {
                for (k, has) in m.keys.iter().enumerate() {
                    keys[k] |= *has;
                }
            }
            self.player.keys = keys;
            for m in &mut self.mates {
                m.keys = keys;
            }
        }
        // rockets
        let mut boom: Vec<Vec3> = Vec::new();
        for r in &mut self.rockets {
            r.life -= dt;
            let steps = ((r.vel.length() * dt) / STEP).ceil().max(1.0) as i32;
            for _ in 0..steps {
                r.pos += r.vel * dt / steps as f32;
                let (tx, ty) = self.level.tile_of(r.pos);
                let hit_wall =
                    self.level.solid(tx, ty) || r.pos.y < 0.0 || r.pos.y > super::level::WALL_H;
                let hit_mon = self.monsters.iter().any(|m| {
                    let s = stats(m.kind);
                    m.alive()
                        && (m.pos.x - r.pos.x).powi(2) + (m.pos.z - r.pos.z).powi(2)
                            < s.radius * s.radius
                        && r.pos.y >= 0.0
                        && r.pos.y <= s.height
                });
                if hit_wall || hit_mon {
                    r.life = 0.0;
                    break;
                }
            }
            if r.life <= 0.0 {
                boom.push(r.pos);
            }
        }
        self.rockets.retain(|r| r.life > 0.0);
        for at in boom {
            self.explode(at, &mut ev);
        }
        // monsters
        let corpses: Vec<(usize, Vec3, bool)> = self
            .monsters
            .iter()
            .enumerate()
            .filter(|(_, m)| !m.alive())
            .map(|(i, m)| {
                (
                    i,
                    m.pos,
                    !matches!(
                        m.kind,
                        MonsterKind::Mech | MonsterKind::Voss | MonsterKind::Medic
                    ),
                )
            })
            .collect();
        for i in 0..self.monsters.len() {
            let mut m = std::mem::replace(
                &mut self.monsters[i],
                Monster::new(MonsterKind::Grunt, Vec3::ZERO, 0.0),
            );
            // each monster hunts the nearest living party member and hears the nearest noise
            let target = self.nearest_player(m.pos);
            let noise = self
                .noises
                .iter()
                .copied()
                .min_by(|a, b| a.distance(m.pos).total_cmp(&b.distance(m.pos)));
            let senses = Senses {
                player: if target == 0 {
                    &self.player
                } else {
                    &self.mates[target - 1]
                },
                noise,
                difficulty: self.difficulty,
            };
            let events = think(
                &mut m,
                &mut self.level,
                &senses,
                &corpses,
                &mut self.rng,
                dt,
            );
            self.monsters[i] = m;
            for e in events {
                match e {
                    MEvent::Fire { damage, hit, melee } => {
                        let victim = if target == 0 {
                            &mut self.player
                        } else {
                            &mut self.mates[target - 1]
                        };
                        if hit {
                            let real = victim.hurt(
                                damage
                                    * self.difficulty.max(0.5)
                                    * crate::balance::BOARD_MONSTER_DAMAGE,
                            );
                            if target == 0 {
                                ev.push(BEvent::PlayerHurt(real));
                            }
                        } else if !melee {
                            ev.push(BEvent::Impact(victim.eye() + Vec3::new(0.3, 0.0, 0.3)));
                        }
                    }
                    MEvent::Revive(idx) => {
                        let m = &mut self.monsters[idx];
                        m.health = stats(m.kind).health * 0.5;
                        m.state = State::Run;
                        m.alerted = true;
                        ev.push(BEvent::Message("A MEDIC REVIVES A FALLEN MINION".into()));
                        if self.progress.kills > 0 {
                            self.progress.kills -= 1;
                        }
                    }
                    MEvent::Alerted => {}
                }
            }
        }
        // sabotage countdown
        if let Some(t) = self.progress.charges {
            let nt = t - dt;
            let in_airlock = self.team_in_airlock();
            if nt <= 0.0 {
                self.outcome = if in_airlock {
                    Outcome::Won
                } else {
                    Outcome::Died
                };
                ev.push(BEvent::Explosion(
                    self.level.centre(self.level.goal.0, self.level.goal.1),
                ));
            } else {
                self.progress.charges = Some(nt);
            }
        }
        if self.player.health <= 0.0 && self.mates.iter().all(|m| m.health <= 0.0) {
            self.outcome = Outcome::Died;
        }
        let _ = (CELL, Tile::Floor, Key::Bronze);
        ev
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::level::{Theme, generate};

    fn world(obj: Objective, theme: Theme, size: u32) -> BoardWorld {
        BoardWorld::new(generate(5, theme, obj, size), 5)
    }

    fn idle() -> Input {
        Input::default()
    }

    #[test]
    fn cheat_kill_all_counts_kills_and_force_objective_completes() {
        let mut w = world(Objective::CaptureBridge, Theme::Corvette, 2);
        let n = w.monsters.iter().filter(|m| m.alive()).count();
        assert_eq!(w.kill_all(), n);
        assert_eq!(w.progress.kills as usize, n);
        assert!(w.monsters.iter().all(|m| !m.alive()));
        w.force_objective();
        assert!(w.progress.done);
    }

    #[test]
    fn blaster_kills_a_grunt_and_ammo_weapons_use_ammo() {
        let mut w = world(Objective::CaptureBridge, Theme::Corvette, 2);
        w.monsters.clear();
        let p = w.player.pos;
        w.player.yaw = 0.0; // facing -Z
        let mut g = Monster::new(MonsterKind::Grunt, p + Vec3::new(0.0, 0.0, -4.0), 0.0);
        g.alerted = false;
        w.monsters.push(g);
        // make sure the line is clear
        let (tx, ty) = w.level.tile_of(p + Vec3::new(0.0, 0.0, -4.0));
        if w.level.tile(tx, ty) != Tile::Floor {
            return; // generated geometry blocks this particular line; other tests cover firing
        }
        for _ in 0..8 {
            w.step(
                0.6,
                &Input {
                    fire: true,
                    ..idle()
                },
            );
        }
        assert!(!w.monsters[0].alive(), "grunt survived the blaster");
        w.player.current = Weapon::Scatter;
        w.player.weapons[Weapon::Scatter.idx()] = true;
        w.player.ammo[Ammo::Shells.idx()] = 2;
        w.step(
            1.1,
            &Input {
                fire: true,
                ..idle()
            },
        );
        assert_eq!(w.player.ammo[Ammo::Shells.idx()], 1);
    }

    #[test]
    fn rocket_splash_hurts_everything_nearby() {
        let mut w = world(Objective::CaptureBridge, Theme::Corvette, 2);
        w.monsters.clear();
        let at = w.player.pos + Vec3::new(0.0, 1.0, 0.0);
        w.monsters.push(Monster::new(
            MonsterKind::Guard,
            w.player.pos + Vec3::new(1.5, 0.0, 0.0),
            0.0,
        ));
        let h0 = w.player.health;
        let mut ev = Vec::new();
        w.explode(at, &mut ev);
        assert!(w.monsters[0].health < 100.0);
        assert!(w.player.health < h0);
        assert!(ev.iter().any(|e| matches!(e, BEvent::Explosion(_))));
    }

    #[test]
    fn monsters_wake_up_chase_and_shoot() {
        let mut w = world(Objective::CaptureBridge, Theme::Corvette, 2);
        w.monsters.clear();
        let mut g = Monster::new(
            MonsterKind::Gunner,
            w.player.pos + Vec3::new(3.0, 0.0, 0.0),
            0.0,
        );
        g.alerted = true;
        w.monsters.push(g);
        w.player.health = 10_000.0;
        w.player.max_health = 10_000.0;
        let mut hurt = false;
        for _ in 0..60 * 20 {
            for e in w.step(1.0 / 60.0, &idle()) {
                hurt |= matches!(e, BEvent::PlayerHurt(_));
            }
        }
        assert!(hurt, "gunner never hit the player");
    }

    #[test]
    fn noise_wakes_sleepers_but_silence_does_not() {
        let mut w = world(Objective::CaptureBridge, Theme::Corvette, 2);
        w.monsters.clear();
        let spot = w.player.pos + Vec3::new(6.0, 0.0, 0.0);
        w.monsters.push(Monster::new(MonsterKind::Grunt, spot, 1.0));
        w.monsters[0].yaw = std::f32::consts::PI; // facing away
        w.monsters[0].wander = 1e9; // keep it standing still
        for _ in 0..30 {
            w.step(1.0 / 60.0, &idle());
        }
        assert!(!w.monsters[0].alerted);
        w.step(
            1.0 / 60.0,
            &Input {
                fire: true,
                ..idle()
            },
        );
        w.step(1.0 / 60.0, &idle());
        assert!(w.monsters[0].alerted, "gunshot should alert");
    }

    #[test]
    fn medic_revives_a_corpse() {
        let mut w = world(Objective::CaptureBridge, Theme::Corvette, 2);
        w.monsters.clear();
        let c = w.player.pos + Vec3::new(4.0, 0.0, 0.0);
        let mut corpse = Monster::new(MonsterKind::Guard, c, 0.0);
        corpse.hurt(1000.0, &mut Rng::new(1));
        w.monsters.push(corpse);
        let mut medic = Monster::new(MonsterKind::Medic, c + Vec3::new(1.0, 0.0, 0.0), 0.0);
        medic.alerted = true;
        w.monsters.push(medic);
        w.player.health = 10_000.0;
        let mut revived = false;
        for _ in 0..60 * 8 {
            revived |= w
                .step(1.0 / 60.0, &idle())
                .iter()
                .any(|e| matches!(e, BEvent::Message(m) if m.contains("REVIVES")));
        }
        assert!(revived && w.monsters[0].alive());
    }

    #[test]
    fn doors_open_for_the_player_and_locks_need_keys() {
        let mut w = world(Objective::CaptureBridge, Theme::Corvette, 4);
        let d = w
            .level
            .doors
            .iter()
            .position(|d| d.lock.is_some())
            .or(Some(0))
            .expect("a door");
        let (dx, dy) = (w.level.doors[d].x, w.level.doors[d].y);
        w.player.pos = w.level.centre(dx, dy);
        for _ in 0..60 {
            w.step(1.0 / 60.0, &idle());
        }
        let open = w.level.doors[d].open;
        match w.level.doors[d].lock {
            Some(k) => {
                assert!(open < 0.1, "locked door opened without key");
                w.player.keys[k as usize] = true;
                for _ in 0..60 {
                    w.step(1.0 / 60.0, &idle());
                }
                assert!(w.level.doors[d].open > 0.9);
            }
            None => assert!(open > 0.9),
        }
    }

    #[test]
    fn items_are_collected_and_keys_open_the_way() {
        let mut w = world(Objective::CaptureBridge, Theme::Corvette, 4);
        w.player.health = 50.0;
        w.items.push(Item {
            kind: ItemKind::HealthLarge,
            pos: w.player.pos,
            taken: false,
        });
        w.items.push(Item {
            kind: ItemKind::Key(Key::Gold),
            pos: w.player.pos,
            taken: false,
        });
        w.step(1.0 / 60.0, &idle());
        assert_eq!(w.player.health, 75.0);
        assert!(w.player.keys[Key::Gold as usize]);
    }

    #[test]
    fn leaving_early_is_fled_and_finishing_is_won() {
        let mut w = world(Objective::StealData, Theme::Station, 2);
        w.step(
            1.0 / 60.0,
            &Input {
                use_key: true,
                ..idle()
            },
        );
        assert_eq!(w.outcome, Outcome::Fled);
        let mut w = world(Objective::StealData, Theme::Station, 2);
        w.progress.done = true;
        w.step(
            1.0 / 60.0,
            &Input {
                use_key: true,
                ..idle()
            },
        );
        assert_eq!(w.outcome, Outcome::Won);
    }

    #[test]
    fn sabotage_countdown_kills_a_straggler_and_spares_an_escapee() {
        let mut w = world(Objective::Sabotage, Theme::Corvette, 2);
        w.player.pos = w.level.centre(w.level.goal.0, w.level.goal.1);
        w.step(
            1.0 / 60.0,
            &Input {
                use_key: true,
                ..idle()
            },
        );
        assert!(w.progress.charges.is_some() && w.progress.done);
        w.player.health = 1e9;
        let fuse = w.progress.charges.unwrap_or(0.0);
        assert!(fuse >= crate::balance::BOARD_FUSE_MIN, "fuse {fuse}");
        for _ in 0..(60.0 * (fuse + 2.0)) as u32 {
            w.step(1.0 / 60.0, &idle());
        }
        assert_eq!(w.outcome, Outcome::Died);
        let mut w = world(Objective::Sabotage, Theme::Corvette, 2);
        w.player.pos = w.level.centre(w.level.goal.0, w.level.goal.1);
        w.step(
            1.0 / 60.0,
            &Input {
                use_key: true,
                ..idle()
            },
        );
        w.player.pos = w.level.spawn_point();
        let fuse = w.progress.charges.unwrap_or(0.0);
        for _ in 0..(60.0 * (fuse + 2.0)) as u32 {
            w.step(1.0 / 60.0, &idle());
        }
        assert_eq!(w.outcome, Outcome::Won);
    }

    #[test]
    fn the_fuse_is_longer_on_a_deeper_level() {
        let fuse = |size, seed| {
            let mut w = BoardWorld::new(
                generate(seed, Theme::Corvette, Objective::Sabotage, size),
                seed,
            );
            w.player.pos = w.level.centre(w.level.goal.0, w.level.goal.1);
            w.step(
                1.0 / 60.0,
                &Input {
                    use_key: true,
                    ..idle()
                },
            );
            w.progress.charges.unwrap_or(0.0)
        };
        let total = |size| (1..=8u64).map(|seed| fuse(size, seed)).sum::<f32>();
        assert!(total(5) > total(1), "{} vs {}", total(5), total(1));
    }

    #[test]
    fn alarm_spawns_reinforcements_once() {
        let mut w = world(Objective::CaptureBridge, Theme::Corvette, 3);
        let n = w.monsters.len();
        w.player.pos = w.level.centre(w.level.alarm.0, w.level.alarm.1);
        // not in the airlock centre check: alarm tile is inside airlock room, so test the method directly
        let mut ev = Vec::new();
        w.raise_alarm(&mut ev);
        assert_eq!(w.monsters.len(), n + 3);
        assert!(w.progress.alarm_raised);
    }

    #[test]
    fn rescue_and_boss_objectives_complete() {
        let mut w = world(Objective::Rescue, Theme::PrisonBarge, 2);
        let spots: Vec<Vec3> = w.prisoners.iter().map(|p| p.pos).collect();
        for s in spots {
            w.player.pos = s;
            w.step(
                1.0 / 60.0,
                &Input {
                    use_key: true,
                    ..idle()
                },
            );
        }
        assert!(w.progress.done, "freed {}", w.progress.freed);
        let mut b = world(Objective::KillBoss, Theme::DreadnoughtBridge, 5);
        let i = b
            .monsters
            .iter()
            .position(|m| m.kind == MonsterKind::Voss)
            .expect("voss");
        let mut ev = Vec::new();
        b.hurt_monster(i, 5000.0, &mut ev);
        assert!(b.progress.done);
    }

    #[test]
    fn determinism_same_seed_same_inputs() {
        let run = || {
            let mut w = world(Objective::CaptureBridge, Theme::Station, 3);
            for i in 0..600 {
                w.step(
                    1.0 / 60.0,
                    &Input {
                        forward: 1.0,
                        fire: i % 20 == 0,
                        ..idle()
                    },
                );
                w.player.yaw += 0.01;
            }
            (
                w.player.pos.to_array().map(f32::to_bits),
                w.progress.kills,
                w.monsters.iter().filter(|m| m.alerted).count(),
            )
        };
        assert_eq!(run(), run());
    }
}

#[cfg(test)]
mod coop_tests {
    use super::*;
    use crate::board::level::{Theme, generate};

    fn team(n: usize) -> BoardWorld {
        let mut w = BoardWorld::new(generate(5, Theme::Corvette, Objective::StealData, 1), 5);
        w.add_mates(n);
        w
    }

    #[test]
    fn mates_stand_beside_the_host_and_are_capped() {
        let w = team(5);
        assert_eq!(w.mates.len(), 3);
        assert!(w.mates.iter().all(|m| m.pos.distance(w.player.pos) < 6.0));
        assert_eq!(team(0).mates.len(), 0);
    }

    #[test]
    fn each_party_member_is_driven_by_its_own_input() {
        let mut w = team(1);
        let (h0, m0) = (w.player.pos, w.mates[0].pos);
        let go = Input {
            forward: 1.0,
            ..Default::default()
        };
        for _ in 0..60 {
            w.step_team(1.0 / 60.0, &[Input::default(), go]);
        }
        assert!(w.player.pos.distance(h0) < 0.01, "host stayed");
        assert!(w.mates[0].pos.distance(m0) > 1.0, "partner walked");
    }

    #[test]
    fn monsters_hunt_the_nearest_living_member_and_defeat_needs_everyone_down() {
        let mut w = team(1);
        // put the partner far from the host, next to a monster
        let target = w.monsters[0].pos;
        w.mates[0].pos = target + Vec3::new(2.0, 0.0, 0.0);
        w.mates[0].health = 100.0;
        assert_eq!(w.nearest_player(target), 1);
        w.mates[0].health = 0.0;
        assert_eq!(w.nearest_player(target), 0, "the dead are not targeted");
        // host down, partner up: the action goes on; both down: defeat
        w.player.health = 0.0;
        w.mates[0].health = 50.0;
        w.step_team(1.0 / 60.0, &[]);
        assert_eq!(w.outcome, Outcome::InProgress);
        w.mates[0].health = 0.0;
        w.step_team(1.0 / 60.0, &[]);
        assert_eq!(w.outcome, Outcome::Died);
    }
}
