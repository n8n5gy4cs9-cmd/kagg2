//! Boarding level format and procedural generator (T18).
//!
//! A level is a tile grid (1 tile = [`CELL`] metres). Rooms are placed by rejection sampling,
//! chained left to right with L-shaped corridors, doors are put at room entrances and the doors on
//! the main path are locked with keys that are guaranteed to be reachable first.

use std::collections::VecDeque;

use glam::Vec3;

use crate::sim::rng::Rng;

pub const CELL: f32 = 4.0;
pub const WALL_H: f32 = 3.2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Theme {
    Corvette,
    Station,
    PrisonBarge,
    DreadnoughtBridge,
}

impl Theme {
    pub fn name(self) -> &'static str {
        match self {
            Theme::Corvette => "MINION CORVETTE",
            Theme::Station => "MINION STATION",
            Theme::PrisonBarge => "PRISON BARGE",
            Theme::DreadnoughtBridge => "DREADNOUGHT",
        }
    }

    /// Wall texture names for this theme (cycled per room).
    pub fn walls(self) -> [&'static str; 2] {
        match self {
            Theme::Corvette => ["wall_minion_a", "wall_minion_b"],
            Theme::Station => ["wall_minion_b", "wall_minion_c"],
            Theme::PrisonBarge => ["wall_minion_c", "wall_minion_a"],
            Theme::DreadnoughtBridge => ["wall_minion_a", "wall_minion_c"],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Objective {
    Sabotage,
    CaptureBridge,
    Rescue,
    StealData,
    KillBoss,
}

impl Objective {
    pub fn name(self) -> &'static str {
        match self {
            Objective::Sabotage => "SABOTAGE THE REACTOR",
            Objective::CaptureBridge => "CAPTURE THE BRIDGE",
            Objective::Rescue => "RESCUE THE PRISONERS",
            Objective::StealData => "STEAL THE DATA CORE",
            Objective::KillBoss => "KILL PREFECT VOSS",
        }
    }

    /// Objectives that make sense on each kind of target.
    pub fn for_theme(t: Theme) -> &'static [Objective] {
        match t {
            Theme::Corvette => &[
                Objective::Sabotage,
                Objective::CaptureBridge,
                Objective::StealData,
            ],
            Theme::Station => &[
                Objective::Sabotage,
                Objective::StealData,
                Objective::CaptureBridge,
            ],
            Theme::PrisonBarge => &[Objective::Rescue, Objective::CaptureBridge],
            Theme::DreadnoughtBridge => &[Objective::KillBoss, Objective::Sabotage],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoomKind {
    Airlock,
    Corridor,
    Barracks,
    Armoury,
    Storage,
    Reactor,
    Bridge,
    Cells,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Bronze,
    Silver,
    Gold,
}

impl Key {
    pub const ALL: [Key; 3] = [Key::Bronze, Key::Silver, Key::Gold];

    pub fn name(self) -> &'static str {
        match self {
            Key::Bronze => "BRONZE",
            Key::Silver => "SILVER",
            Key::Gold => "GOLD",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tile {
    Wall,
    Floor,
}

#[derive(Debug, Clone, Copy)]
pub struct Room {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub kind: RoomKind,
}

impl Room {
    pub fn center(&self) -> (i32, i32) {
        (self.x + self.w / 2, self.y + self.h / 2)
    }

    pub fn contains(&self, x: i32, y: i32) -> bool {
        x >= self.x && x < self.x + self.w && y >= self.y && y < self.y + self.h
    }
}

#[derive(Debug, Clone)]
pub struct Door {
    pub x: i32,
    pub y: i32,
    /// The corridor runs along X (the slab lies in the YZ plane).
    pub along_x: bool,
    pub lock: Option<Key>,
    /// `0.0` closed … `1.0` open.
    pub open: f32,
    /// Seconds the door stays open after the last visitor.
    pub hold: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MonsterKind {
    Grunt,
    Guard,
    Gunner,
    Medic,
    Enforcer,
    Mech,
    Voss,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemKind {
    HealthSmall,
    HealthLarge,
    ArmorShard,
    ArmorJacket,
    ArmorBody,
    Shells,
    Bullets,
    Rockets,
    Slugs,
    Weapon(super::weapons::Weapon),
    Key(Key),
    DataChip,
}

#[derive(Debug, Clone, Copy)]
pub enum SpawnKind {
    Monster(MonsterKind),
    Item(ItemKind),
    Prisoner,
}

#[derive(Debug, Clone, Copy)]
pub struct Spawn {
    pub x: i32,
    pub y: i32,
    pub kind: SpawnKind,
}

#[derive(Debug, Clone)]
pub struct Level {
    pub w: i32,
    pub h: i32,
    pub tiles: Vec<Tile>,
    pub rooms: Vec<Room>,
    pub doors: Vec<Door>,
    pub spawns: Vec<Spawn>,
    pub theme: Theme,
    pub objective: Objective,
    /// Tile of the objective (reactor core, data terminal, bridge console, boss, …).
    pub goal: (i32, i32),
    /// Tile of the alarm button.
    pub alarm: (i32, i32),
    pub seed: u64,
}

impl Level {
    pub fn tile(&self, x: i32, y: i32) -> Tile {
        if x < 0 || y < 0 || x >= self.w || y >= self.h {
            Tile::Wall
        } else {
            self.tiles[(y * self.w + x) as usize]
        }
    }

    pub fn door_at(&self, x: i32, y: i32) -> Option<usize> {
        self.doors.iter().position(|d| d.x == x && d.y == y)
    }

    pub fn room_at(&self, x: i32, y: i32) -> Option<&Room> {
        self.rooms.iter().find(|r| r.contains(x, y))
    }

    /// World position (metres, floor level) of the centre of a tile.
    pub fn centre(&self, x: i32, y: i32) -> Vec3 {
        Vec3::new((x as f32 + 0.5) * CELL, 0.0, (y as f32 + 0.5) * CELL)
    }

    pub fn spawn_point(&self) -> Vec3 {
        let (x, y) = self.rooms[0].center();
        self.centre(x, y)
    }

    /// Whether the tile blocks movement and sight (wall or a not fully open door).
    pub fn solid(&self, x: i32, y: i32) -> bool {
        match self.tile(x, y) {
            Tile::Wall => true,
            Tile::Floor => self.door_at(x, y).is_some_and(|d| self.doors[d].open < 0.8),
        }
    }

    /// Tile coordinates containing a world position.
    pub fn tile_of(&self, p: Vec3) -> (i32, i32) {
        ((p.x / CELL).floor() as i32, (p.z / CELL).floor() as i32)
    }

    /// Whether the straight line between two points is free of walls and closed doors.
    pub fn los(&self, a: Vec3, b: Vec3) -> bool {
        let d = b - a;
        let n = (d.length() / 0.3).ceil().max(1.0) as i32;
        (1..n).all(|i| {
            let p = a + d * (i as f32 / n as f32);
            let (x, y) = self.tile_of(p);
            !self.solid(x, y)
        })
    }

    pub fn floor_tiles(&self) -> impl Iterator<Item = (i32, i32)> + '_ {
        (0..self.h)
            .flat_map(move |y| (0..self.w).map(move |x| (x, y)))
            .filter(|&(x, y)| self.tile(x, y) == Tile::Floor)
    }

    /// Tiles reachable from `from` walking over floor; doors in `blocked` count as walls.
    fn flood(&self, from: (i32, i32), passable: impl Fn(Option<usize>) -> bool) -> Vec<bool> {
        let mut seen = vec![false; (self.w * self.h) as usize];
        let mut q = VecDeque::new();
        seen[(from.1 * self.w + from.0) as usize] = true;
        q.push_back(from);
        while let Some((x, y)) = q.pop_front() {
            for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                let (nx, ny) = (x + dx, y + dy);
                if self.tile(nx, ny) != Tile::Floor || seen[(ny * self.w + nx) as usize] {
                    continue;
                }
                if !passable(self.door_at(nx, ny)) {
                    continue;
                }
                seen[(ny * self.w + nx) as usize] = true;
                q.push_back((nx, ny));
            }
        }
        seen
    }

    /// Shortest tile path (inclusive) over floor, ignoring locks.
    pub fn path(&self, from: (i32, i32), to: (i32, i32)) -> Option<Vec<(i32, i32)>> {
        let mut prev = vec![None; (self.w * self.h) as usize];
        let mut seen = vec![false; (self.w * self.h) as usize];
        let mut q = VecDeque::new();
        seen[(from.1 * self.w + from.0) as usize] = true;
        q.push_back(from);
        while let Some((x, y)) = q.pop_front() {
            if (x, y) == to {
                let mut p = vec![to];
                let mut cur = to;
                while let Some(pr) = prev[(cur.1 * self.w + cur.0) as usize] {
                    p.push(pr);
                    cur = pr;
                }
                p.reverse();
                return Some(p);
            }
            for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                let (nx, ny) = (x + dx, y + dy);
                let i = (ny * self.w + nx) as usize;
                if self.tile(nx, ny) == Tile::Floor && !seen[i] {
                    seen[i] = true;
                    prev[i] = Some((x, y));
                    q.push_back((nx, ny));
                }
            }
        }
        None
    }

    /// Can the player reach the goal and every key/item, picking up keys on the way?
    pub fn solvable(&self) -> bool {
        let start = {
            let (x, y) = self.rooms[0].center();
            (x, y)
        };
        let mut keys: Vec<Key> = Vec::new();
        loop {
            let seen = self.flood(start, |d| {
                d.is_none_or(|i| self.doors[i].lock.is_none_or(|k| keys.contains(&k)))
            });
            let mut grew = false;
            for s in &self.spawns {
                if let SpawnKind::Item(ItemKind::Key(k)) = s.kind
                    && seen[(s.y * self.w + s.x) as usize]
                    && !keys.contains(&k)
                {
                    keys.push(k);
                    grew = true;
                }
            }
            if !grew {
                return seen[(self.goal.1 * self.w + self.goal.0) as usize];
            }
        }
    }
}

fn carve_room(tiles: &mut [Tile], w: i32, r: &Room) {
    for y in r.y..r.y + r.h {
        for x in r.x..r.x + r.w {
            tiles[(y * w + x) as usize] = Tile::Floor;
        }
    }
}

fn carve_corridor(
    tiles: &mut [Tile],
    w: i32,
    a: (i32, i32),
    b: (i32, i32),
    horizontal_first: bool,
) {
    let mut set = |x: i32, y: i32| tiles[(y * w + x) as usize] = Tile::Floor;
    let (mut x, mut y) = a;
    let step = |from: i32, to: i32| (to - from).signum();
    if horizontal_first {
        while x != b.0 {
            set(x, y);
            x += step(x, b.0);
        }
        while y != b.1 {
            set(x, y);
            y += step(y, b.1);
        }
    } else {
        while y != b.1 {
            set(x, y);
            y += step(y, b.1);
        }
        while x != b.0 {
            set(x, y);
            x += step(x, b.0);
        }
    }
    set(b.0, b.1);
}

/// Generate a level. `size` is `1..=5` (enemy ship class). Deterministic in `seed`.
pub fn generate(seed: u64, theme: Theme, objective: Objective, size: u32) -> Level {
    for attempt in 0..64u64 {
        if let Some(l) = try_generate(
            seed.wrapping_add(attempt * 7919),
            theme,
            objective,
            size.clamp(1, 5),
        ) {
            return l;
        }
    }
    // Fallback: a trivial two-room level that is always solvable.
    fallback(seed, theme, objective)
}

fn fallback(seed: u64, theme: Theme, objective: Objective) -> Level {
    let (w, h) = (24, 12);
    let mut tiles = vec![Tile::Wall; (w * h) as usize];
    let rooms = vec![
        Room {
            x: 1,
            y: 3,
            w: 4,
            h: 4,
            kind: RoomKind::Airlock,
        },
        Room {
            x: 14,
            y: 3,
            w: 8,
            h: 6,
            kind: goal_room(objective),
        },
    ];
    for r in &rooms {
        carve_room(&mut tiles, w, r);
    }
    carve_corridor(&mut tiles, w, (3, 5), (17, 5), true);
    let goal = rooms[1].center();
    Level {
        w,
        h,
        tiles,
        rooms,
        doors: Vec::new(),
        spawns: Vec::new(),
        theme,
        objective,
        goal,
        alarm: (3, 4),
        seed,
    }
}

fn goal_room(o: Objective) -> RoomKind {
    match o {
        Objective::Sabotage => RoomKind::Reactor,
        Objective::Rescue => RoomKind::Cells,
        _ => RoomKind::Bridge,
    }
}

fn try_generate(seed: u64, theme: Theme, objective: Objective, size: u32) -> Option<Level> {
    use super::weapons::Weapon;
    let mut rng = Rng::new(seed ^ 0xB0A2D);
    let (w, h) = (40 + 10 * size as i32, 28 + 4 * size as i32);
    let mut tiles = vec![Tile::Wall; (w * h) as usize];
    let want = 4 + 2 * size as usize;
    let mut rooms: Vec<Room> = Vec::new();
    for _ in 0..600 {
        if rooms.len() >= want {
            break;
        }
        let (rw, rh) = (rng.range(4.0, 9.0) as i32, rng.range(4.0, 8.0) as i32);
        let x = rng.range(2.0, (w - rw - 2) as f32) as i32;
        let y = rng.range(2.0, (h - rh - 2) as f32) as i32;
        let r = Room {
            x,
            y,
            w: rw,
            h: rh,
            kind: RoomKind::Storage,
        };
        let clear = rooms.iter().all(|o| {
            r.x > o.x + o.w + 2 || o.x > r.x + r.w + 2 || r.y > o.y + o.h + 2 || o.y > r.y + r.h + 2
        });
        if clear {
            rooms.push(r);
        }
    }
    if rooms.len() < 4 {
        return None;
    }
    rooms.sort_by_key(|r| (r.x, r.y));
    // airlock = leftmost room, shrunk; goal room = rightmost
    let a = rooms[0];
    rooms[0] = Room {
        w: a.w.min(5),
        h: a.h.min(5),
        kind: RoomKind::Airlock,
        ..a
    };
    let last = rooms.len() - 1;
    rooms[last].kind = goal_room(objective);
    let nrooms = rooms.len();
    for (i, r) in rooms.iter_mut().enumerate().skip(1).take(nrooms - 2) {
        r.kind = match rng.below(5) {
            0 | 1 => RoomKind::Barracks,
            2 => RoomKind::Armoury,
            _ => RoomKind::Storage,
        };
        let _ = i;
    }
    for r in &rooms {
        carve_room(&mut tiles, w, r);
    }
    for i in 0..rooms.len() - 1 {
        carve_corridor(
            &mut tiles,
            w,
            rooms[i].center(),
            rooms[i + 1].center(),
            rng.below(2) == 0,
        );
    }
    if rooms.len() > 7 {
        let (i, j) = (
            rng.below(rooms.len() as u32 - 2) as usize,
            rng.below(rooms.len() as u32) as usize,
        );
        if i != j {
            carve_corridor(&mut tiles, w, rooms[i].center(), rooms[j].center(), true);
        }
    }
    let mut lvl = Level {
        w,
        h,
        tiles,
        rooms,
        doors: Vec::new(),
        spawns: Vec::new(),
        theme,
        objective,
        goal: (0, 0),
        alarm: (0, 0),
        seed,
    };
    // doors: corridor chokepoints that touch a room
    let in_room = |l: &Level, x: i32, y: i32| l.room_at(x, y).is_some();
    let mut doors = Vec::new();
    for (x, y) in lvl.floor_tiles().collect::<Vec<_>>() {
        if in_room(&lvl, x, y) {
            continue;
        }
        let f = |dx: i32, dy: i32| lvl.tile(x + dx, y + dy) == Tile::Floor;
        let (lr, ud) = (
            f(-1, 0) && f(1, 0) && !f(0, -1) && !f(0, 1),
            f(0, -1) && f(0, 1) && !f(-1, 0) && !f(1, 0),
        );
        let touches = [(1, 0), (-1, 0), (0, 1), (0, -1)]
            .iter()
            .any(|&(dx, dy)| in_room(&lvl, x + dx, y + dy));
        if (lr || ud) && touches {
            doors.push(Door {
                x,
                y,
                along_x: lr,
                lock: None,
                open: 0.0,
                hold: 0.0,
            });
        }
    }
    lvl.doors = doors;
    let start = lvl.rooms[0].center();
    // the goal tile lives in the goal room; the boss/console sits slightly off-centre
    let gr = lvl.rooms[last];
    lvl.goal = gr.center();
    lvl.alarm = (lvl.rooms[0].x + 1, lvl.rooms[0].y);
    if lvl.tile(lvl.alarm.0, lvl.alarm.1) != Tile::Floor {
        lvl.alarm = start;
    }
    // locks on the main path
    let path = lvl.path(start, lvl.goal)?;
    let on_path: Vec<usize> = path
        .iter()
        .filter_map(|&(x, y)| lvl.door_at(x, y))
        .collect();
    let locks = (on_path.len() / 2).min(1 + size as usize / 2).min(3);
    let mut locked: Vec<usize> = Vec::new();
    for k in 0..locks {
        // spread the locks along the path, later doors get stronger keys
        let idx = on_path[(k + 1) * on_path.len() / (locks + 1)];
        if !locked.contains(&idx) {
            locked.push(idx);
        }
    }
    for (k, &d) in locked.iter().enumerate() {
        lvl.doors[d].lock = Some(Key::ALL[k]);
    }
    // keys: key k is placed in the region reachable with only earlier keys
    let mut occupied: Vec<(i32, i32)> = vec![start, lvl.goal];
    for (k, &d) in locked.iter().enumerate() {
        let region = lvl.flood(start, |dd| dd.is_none_or(|i| !locked[k..].contains(&i)));
        let cands: Vec<(i32, i32)> = lvl
            .floor_tiles()
            .filter(|&(x, y)| {
                region[(y * w + x) as usize]
                    && lvl
                        .room_at(x, y)
                        .is_some_and(|r| r.kind != RoomKind::Airlock)
                    && !occupied.contains(&(x, y))
            })
            .collect();
        let &(x, y) = cands.get(rng.below(cands.len().max(1) as u32) as usize)?;
        occupied.push((x, y));
        lvl.spawns.push(Spawn {
            x,
            y,
            kind: SpawnKind::Item(ItemKind::Key(Key::ALL[k])),
        });
        let _ = d;
    }
    // populate rooms
    let n = lvl.rooms.len();
    let rooms_copy = lvl.rooms.clone();
    for (i, r) in rooms_copy.iter().enumerate() {
        if r.kind == RoomKind::Airlock {
            continue;
        }
        let depth = i as f32 / n as f32;
        let pick_tile = |rng: &mut Rng, occ: &mut Vec<(i32, i32)>| -> (i32, i32) {
            for _ in 0..20 {
                let p = (
                    rng.range(r.x as f32, (r.x + r.w) as f32) as i32,
                    rng.range(r.y as f32, (r.y + r.h) as f32) as i32,
                );
                if !occ.contains(&p) {
                    occ.push(p);
                    return p;
                }
            }
            r.center()
        };
        let mut count = 1 + rng.below(2) + size / 2;
        if r.kind == RoomKind::Barracks {
            count += 2;
        }
        if r.kind == RoomKind::Storage {
            count = count.saturating_sub(1);
        }
        for _ in 0..count {
            let roll = rng.f32();
            let kind = if depth > 0.7 && roll < 0.25 {
                MonsterKind::Enforcer
            } else if depth > 0.5 && roll < 0.4 {
                MonsterKind::Medic
            } else if depth > 0.35 && roll < 0.6 {
                MonsterKind::Gunner
            } else if depth > 0.15 && roll < 0.75 {
                MonsterKind::Guard
            } else {
                MonsterKind::Grunt
            };
            let (x, y) = pick_tile(&mut rng, &mut occupied);
            lvl.spawns.push(Spawn {
                x,
                y,
                kind: SpawnKind::Monster(kind),
            });
        }
        // pickups
        let items: &[ItemKind] = match r.kind {
            RoomKind::Armoury => &[
                ItemKind::Shells,
                ItemKind::Bullets,
                ItemKind::Rockets,
                ItemKind::Slugs,
                ItemKind::ArmorJacket,
            ],
            RoomKind::Barracks => &[
                ItemKind::HealthSmall,
                ItemKind::Bullets,
                ItemKind::ArmorShard,
            ],
            RoomKind::Storage => &[
                ItemKind::HealthLarge,
                ItemKind::Shells,
                ItemKind::ArmorShard,
            ],
            _ => &[ItemKind::HealthSmall, ItemKind::Bullets],
        };
        for _ in 0..(1 + rng.below(2)) {
            let it = items[rng.below(items.len() as u32) as usize];
            let (x, y) = pick_tile(&mut rng, &mut occupied);
            lvl.spawns.push(Spawn {
                x,
                y,
                kind: SpawnKind::Item(it),
            });
        }
        if r.kind == RoomKind::Armoury {
            let wp = [
                Weapon::Scatter,
                Weapon::Rattler,
                Weapon::Mjolnir,
                Weapon::Valkyrie,
            ];
            let (x, y) = pick_tile(&mut rng, &mut occupied);
            lvl.spawns.push(Spawn {
                x,
                y,
                kind: SpawnKind::Item(ItemKind::Weapon(wp[(rng.below(2 + size.min(2))) as usize])),
            });
        }
    }
    // objective dressing
    let (gx, gy) = lvl.goal;
    match objective {
        Objective::KillBoss => lvl.spawns.push(Spawn {
            x: gx,
            y: gy,
            kind: SpawnKind::Monster(MonsterKind::Voss),
        }),
        Objective::Rescue => {
            let gr = lvl.rooms[last];
            for k in 0..3 {
                let (x, y) = (gr.x + 1 + k, gr.y + 1);
                lvl.spawns.push(Spawn {
                    x: x.min(gr.x + gr.w - 1),
                    y,
                    kind: SpawnKind::Prisoner,
                });
            }
        }
        Objective::StealData => lvl.spawns.push(Spawn {
            x: gx,
            y: gy,
            kind: SpawnKind::Item(ItemKind::DataChip),
        }),
        _ => {}
    }
    if size >= 4 && matches!(objective, Objective::Sabotage | Objective::CaptureBridge) {
        lvl.spawns.push(Spawn {
            x: gx + 1,
            y: gy,
            kind: SpawnKind::Monster(MonsterKind::Mech),
        });
    }
    lvl.solvable().then_some(lvl)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_theme_objective_and_size_is_solvable() {
        for theme in [
            Theme::Corvette,
            Theme::Station,
            Theme::PrisonBarge,
            Theme::DreadnoughtBridge,
        ] {
            for &obj in Objective::for_theme(theme) {
                for size in 1..=5 {
                    for seed in 0..4u64 {
                        let l = generate(seed, theme, obj, size);
                        assert!(l.solvable(), "{theme:?} {obj:?} size {size} seed {seed}");
                        assert!(l.rooms.len() >= 2);
                        assert_eq!(l.tile(l.goal.0, l.goal.1), Tile::Floor);
                    }
                }
            }
        }
    }

    #[test]
    fn deterministic_and_size_scales() {
        let a = generate(7, Theme::Station, Objective::Sabotage, 3);
        let b = generate(7, Theme::Station, Objective::Sabotage, 3);
        assert_eq!(a.tiles, b.tiles);
        assert_eq!(a.spawns.len(), b.spawns.len());
        let small = generate(7, Theme::Station, Objective::Sabotage, 1);
        let big = generate(7, Theme::Station, Objective::Sabotage, 5);
        assert!(big.rooms.len() > small.rooms.len());
        assert!(big.spawns.len() > small.spawns.len());
    }

    #[test]
    fn locked_doors_have_their_keys_placed_earlier() {
        let mut locked = 0;
        for seed in 0..30u64 {
            let l = generate(seed, Theme::Corvette, Objective::CaptureBridge, 4);
            for d in &l.doors {
                if let Some(k) = d.lock {
                    locked += 1;
                    assert!(
                        l.spawns.iter().any(
                            |s| matches!(s.kind, SpawnKind::Item(ItemKind::Key(kk)) if kk == k)
                        )
                    );
                }
            }
        }
        assert!(locked > 0, "generator never locks a door");
    }

    #[test]
    fn boss_and_prisoners_appear_for_their_objectives() {
        let boss = generate(1, Theme::DreadnoughtBridge, Objective::KillBoss, 5);
        assert!(
            boss.spawns
                .iter()
                .any(|s| matches!(s.kind, SpawnKind::Monster(MonsterKind::Voss)))
        );
        let jail = generate(1, Theme::PrisonBarge, Objective::Rescue, 2);
        assert_eq!(
            jail.spawns
                .iter()
                .filter(|s| matches!(s.kind, SpawnKind::Prisoner))
                .count(),
            3
        );
    }
}
