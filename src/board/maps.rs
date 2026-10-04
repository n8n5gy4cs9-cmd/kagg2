//! Hand-authored boarding maps in the spirit of Quake 2's base levels (no Quake data is used:
//! these layouts are original ASCII art, textured with our generated tiles).
//!
//! Each map has two layers of the same size. **Layout:** `#` wall, `.` corridor, a letter = a room
//! floor (`L` airlock, `B` barracks, `A` armoury, `S` storage, `R` reactor, `D` bridge, `C` cells,
//! `T` the target room whose kind follows the objective), `+` door, `1`/`2`/`3` bronze/silver/gold
//! locked door. **Things:** `@` start, `!` alarm, `G` goal, `m` monster, `M` tougher monster,
//! `X` mini-boss (Mech on big ships), `p` prisoner, `x`/`y`/`z` bronze/silver/gold key,
//! `h` small / `H` large health, `s` shard, `a` jacket, `A` body armour, `e` shells, `u` bullets,
//! `r` rockets, `l` slugs, `w` scatter-gun, `n` rattler, `o` Mjolnir, `v` Valkyrie.
//! Maps are mirrored by seed for variety. See `ASSETS.md` ("Level layouts") for what to replace.

use std::collections::VecDeque;

use super::level::{
    Door, ItemKind, Key, Level, MonsterKind, Objective, Room, RoomKind, Spawn, SpawnKind, Theme,
    Tile,
};
use super::weapons::Weapon;
use crate::sim::rng::Rng;

/// One authored map.
pub struct Template {
    pub name: &'static str,
    /// Enemy ship sizes (`1..=5`) this map is used for.
    pub sizes: std::ops::RangeInclusive<u32>,
    layout: &'static [&'static str],
    things: &'static [&'static str],
}

pub static TEMPLATES: [Template; 3] = [
    Template {
        name: "Outer Base",
        sizes: 2..=3,
        layout: OUTPOST_LAYOUT,
        things: OUTPOST_THINGS,
    },
    Template {
        name: "Cargo Deck",
        sizes: 3..=4,
        layout: CARGO_LAYOUT,
        things: CARGO_THINGS,
    },
    Template {
        name: "Citadel",
        sizes: 5..=5,
        layout: CITADEL_LAYOUT,
        things: CITADEL_THINGS,
    },
];

/// Templates that can serve ships of `size`.
pub fn eligible(size: u32) -> Vec<&'static Template> {
    TEMPLATES
        .iter()
        .filter(|t| t.sizes.contains(&size))
        .collect()
}

fn room_kind(c: u8) -> Option<RoomKind> {
    Some(match c {
        b'L' => RoomKind::Airlock,
        b'B' => RoomKind::Barracks,
        b'A' => RoomKind::Armoury,
        b'S' => RoomKind::Storage,
        b'R' => RoomKind::Reactor,
        b'D' => RoomKind::Bridge,
        b'C' => RoomKind::Cells,
        b'T' => RoomKind::Storage, // replaced by the objective's room kind
        _ => return None,
    })
}

fn goal_room(o: Objective) -> RoomKind {
    match o {
        Objective::Sabotage => RoomKind::Reactor,
        Objective::Rescue => RoomKind::Cells,
        _ => RoomKind::Bridge,
    }
}

impl Template {
    /// Build a playable level. Deterministic in `seed`; `None` if the map data is malformed.
    pub fn build(&self, seed: u64, theme: Theme, objective: Objective, size: u32) -> Option<Level> {
        let (w, h) = (self.layout.first()?.len() as i32, self.layout.len() as i32);
        let mut rng = Rng::new(seed ^ 0x007E_3A91);
        let (flip_x, flip_y) = (rng.below(2) == 1, rng.below(2) == 1);
        let at = |rows: &[&str], x: i32, y: i32| -> u8 {
            let sx = if flip_x { w - 1 - x } else { x };
            let sy = if flip_y { h - 1 - y } else { y };
            rows.get(sy as usize)
                .and_then(|r| r.as_bytes().get(sx as usize))
                .copied()
                .unwrap_or(b'#')
        };
        let mut tiles = vec![Tile::Wall; (w * h) as usize];
        for y in 0..h {
            for x in 0..w {
                if at(self.layout, x, y) != b'#' {
                    tiles[(y * w + x) as usize] = Tile::Floor;
                }
            }
        }
        // rooms = bounding boxes of connected same-letter floor
        let mut seen = vec![false; (w * h) as usize];
        let mut rooms: Vec<Room> = Vec::new();
        for y in 0..h {
            for x in 0..w {
                let c = at(self.layout, x, y);
                let Some(kind) = room_kind(c) else { continue };
                if seen[(y * w + x) as usize] {
                    continue;
                }
                let (mut x0, mut y0, mut x1, mut y1) = (x, y, x, y);
                let mut q = VecDeque::from([(x, y)]);
                seen[(y * w + x) as usize] = true;
                while let Some((cx, cy)) = q.pop_front() {
                    (x0, y0, x1, y1) = (x0.min(cx), y0.min(cy), x1.max(cx), y1.max(cy));
                    for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                        let (nx, ny) = (cx + dx, cy + dy);
                        if nx >= 0
                            && ny >= 0
                            && nx < w
                            && ny < h
                            && at(self.layout, nx, ny) == c
                            && !seen[(ny * w + nx) as usize]
                        {
                            seen[(ny * w + nx) as usize] = true;
                            q.push_back((nx, ny));
                        }
                    }
                }
                let kind = if c == b'T' {
                    goal_room(objective)
                } else {
                    kind
                };
                rooms.push(Room {
                    x: x0,
                    y: y0,
                    w: x1 - x0 + 1,
                    h: y1 - y0 + 1,
                    kind,
                });
            }
        }
        let airlock = rooms.iter().position(|r| r.kind == RoomKind::Airlock)?;
        rooms.swap(0, airlock);
        // doors
        let floor = |x: i32, y: i32| {
            x >= 0 && y >= 0 && x < w && y < h && tiles[(y * w + x) as usize] == Tile::Floor
        };
        let mut doors = Vec::new();
        for y in 0..h {
            for x in 0..w {
                let lock = match at(self.layout, x, y) {
                    b'+' => None,
                    b'1' => Some(Key::Bronze),
                    b'2' => Some(Key::Silver),
                    b'3' => Some(Key::Gold),
                    _ => continue,
                };
                doors.push(Door {
                    x,
                    y,
                    along_x: floor(x - 1, y) && floor(x + 1, y),
                    lock,
                    open: 0.0,
                    hold: 0.0,
                });
            }
        }
        let mut lvl = Level {
            w,
            h,
            tiles,
            rooms,
            doors,
            spawns: Vec::new(),
            theme,
            objective,
            goal: (0, 0),
            alarm: (0, 0),
            seed,
        };
        let start = lvl.rooms[0].center();
        // tile of each thing glyph
        let mut marks: Vec<(i32, i32, u8)> = Vec::new();
        for y in 0..h {
            for x in 0..w {
                let c = at(self.things, x, y);
                if c != b'.' && lvl.tile(x, y) == Tile::Floor {
                    marks.push((x, y, c));
                }
            }
        }
        let find = |g: u8| marks.iter().find(|m| m.2 == g).map(|m| (m.0, m.1));
        lvl.goal = find(b'G')?;
        lvl.alarm = find(b'!').unwrap_or(start);
        // depth of every tile from the start (ignoring locks) for monster tiers
        let dist = distances(&lvl, start);
        let far = dist.iter().copied().max().unwrap_or(1).max(1) as f32;
        let big = size >= 4 && matches!(objective, Objective::Sabotage | Objective::CaptureBridge);
        for &(x, y, g) in &marks {
            let depth = dist[(y * w + x) as usize].max(0) as f32 / far;
            let kind = match g {
                b'm' => Some(SpawnKind::Monster(tier(depth, &mut rng))),
                b'M' => Some(SpawnKind::Monster(tier(depth + 0.25, &mut rng))),
                b'X' if big => Some(SpawnKind::Monster(MonsterKind::Mech)),
                b'X' => Some(SpawnKind::Monster(MonsterKind::Enforcer)),
                b'p' => Some(SpawnKind::Prisoner),
                b'x' => Some(SpawnKind::Item(ItemKind::Key(Key::Bronze))),
                b'y' => Some(SpawnKind::Item(ItemKind::Key(Key::Silver))),
                b'z' => Some(SpawnKind::Item(ItemKind::Key(Key::Gold))),
                b'h' => Some(SpawnKind::Item(ItemKind::HealthSmall)),
                b'H' => Some(SpawnKind::Item(ItemKind::HealthLarge)),
                b's' => Some(SpawnKind::Item(ItemKind::ArmorShard)),
                b'a' => Some(SpawnKind::Item(ItemKind::ArmorJacket)),
                b'A' => Some(SpawnKind::Item(ItemKind::ArmorBody)),
                b'e' => Some(SpawnKind::Item(ItemKind::Shells)),
                b'u' => Some(SpawnKind::Item(ItemKind::Bullets)),
                b'r' => Some(SpawnKind::Item(ItemKind::Rockets)),
                b'l' => Some(SpawnKind::Item(ItemKind::Slugs)),
                b'w' => Some(SpawnKind::Item(ItemKind::Weapon(Weapon::Scatter))),
                b'n' => Some(SpawnKind::Item(ItemKind::Weapon(Weapon::Rattler))),
                b'o' => Some(SpawnKind::Item(ItemKind::Weapon(Weapon::Mjolnir))),
                b'v' => Some(SpawnKind::Item(ItemKind::Weapon(Weapon::Valkyrie))),
                _ => None,
            };
            if let Some(kind) = kind {
                lvl.spawns.push(Spawn { x, y, kind });
            }
        }
        let (gx, gy) = lvl.goal;
        match objective {
            Objective::KillBoss => lvl.spawns.push(Spawn {
                x: gx,
                y: gy,
                kind: SpawnKind::Monster(MonsterKind::Voss),
            }),
            Objective::StealData => lvl.spawns.push(Spawn {
                x: gx,
                y: gy,
                kind: SpawnKind::Item(ItemKind::DataChip),
            }),
            Objective::Rescue => {
                let have = lvl
                    .spawns
                    .iter()
                    .filter(|s| matches!(s.kind, SpawnKind::Prisoner))
                    .count();
                let mut near: Vec<(i32, i32)> = lvl
                    .floor_tiles()
                    .filter(|&(x, y)| {
                        lvl.room_at(x, y).is_some_and(|r| r.kind == RoomKind::Cells)
                            && (x, y) != lvl.goal
                            && !lvl.spawns.iter().any(|s| (s.x, s.y) == (x, y))
                    })
                    .collect();
                near.sort_by_key(|&(x, y)| (x - gx).abs() + (y - gy).abs());
                for &(x, y) in near.iter().take(3usize.saturating_sub(have)) {
                    lvl.spawns.push(Spawn {
                        x,
                        y,
                        kind: SpawnKind::Prisoner,
                    });
                }
            }
            _ => {}
        }
        lvl.solvable().then_some(lvl)
    }
}

/// Monster kind for a depth `0..=1+` into the level (same ramp as the random generator).
fn tier(depth: f32, rng: &mut Rng) -> MonsterKind {
    let roll = rng.f32();
    if depth > 0.7 && roll < 0.25 {
        MonsterKind::Enforcer
    } else if depth > 0.5 && roll < 0.4 {
        MonsterKind::Medic
    } else if depth > 0.35 && roll < 0.6 {
        MonsterKind::Gunner
    } else if depth > 0.15 && roll < 0.75 {
        MonsterKind::Guard
    } else {
        MonsterKind::Grunt
    }
}

/// Walking distance (tiles) from `from` to every floor tile, doors ignored; `-1` = unreachable.
fn distances(l: &Level, from: (i32, i32)) -> Vec<i32> {
    let mut d = vec![-1; (l.w * l.h) as usize];
    let mut q = VecDeque::from([from]);
    d[(from.1 * l.w + from.0) as usize] = 0;
    while let Some((x, y)) = q.pop_front() {
        let here = d[(y * l.w + x) as usize];
        for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            let (nx, ny) = (x + dx, y + dy);
            if l.tile(nx, ny) == Tile::Floor && d[(ny * l.w + nx) as usize] < 0 {
                d[(ny * l.w + nx) as usize] = here + 1;
                q.push_back((nx, ny));
            }
        }
    }
    d
}

const OUTPOST_LAYOUT: &[&str] = &[
    "############################################",
    "###########BBBBBBBBBB#####BBBBBBBB##########",
    "###########BBBBBBBBBB#####BBBBBBBB##########",
    "###########BBBBBBBBBB..+..BBBBBBBB##########",
    "###########BBBBBBBBBB#####BBBBBBBB##########",
    "###########BBBBBBBBBB#####BBBBBBBB##########",
    "###############+#############.######TTTTTT##",
    "###############.#############+######TTTTTT##",
    "###########SSSSSSSSSS########.######TTTTTT##",
    "###########SSSSSSSSSS#####SSSSSSS###TTTTTT##",
    "##LLLLL####SS#SSSS#SS#####SSSSSSS###TTTTTT##",
    "##LLLLL####SSSSSSSSSS#####SS#SSSS###TTTTTT##",
    "##LLLLL.+..SSSSSSSSSS..1..SSSSSSS.2.TTTTTT##",
    "##LLLLL####SSSSSSSSSS#####SSSS#SS###TTTTTT##",
    "##LLLLL####SS#SSSS#SS#####SSSSSSS###TTTTTT##",
    "###########SSSSSSSSSS#####SSSSSSS###TTTTTT##",
    "###########SSSSSSSSSS###############TTTTTT##",
    "###############.####################TTTTTT##",
    "###############+####################TTTTTT##",
    "###########AAAAAAAAAA#######################",
    "###########AAAAAAAAAA#######################",
    "###########AAAAAAAAAA#######################",
    "###########AAAAAAAAAA#######################",
    "###########AAAAAAAAAA#######################",
    "############################################",
    "############################################",
];
const OUTPOST_THINGS: &[&str] = &[
    "............................................",
    "............................................",
    "...............h................y...........",
    ".............m....m.........m..M............",
    "...................u.......H................",
    "............................................",
    "............................................",
    "............................................",
    "......................................m.....",
    "..............e.........................e...",
    "...!.......................h...M............",
    "............................................",
    "....@...........m...........m..........GM...",
    "............................................",
    "...............................u............",
    "....................s.......................",
    "......................................m.w...",
    ".....................................a......",
    "............................................",
    "............................................",
    "...................a........................",
    "..............m.............................",
    "............x.....e.........................",
    "............................................",
    "............................................",
    "............................................",
];
const CARGO_LAYOUT: &[&str] = &[
    "############################################################",
    "############################BBBBBBBBBB######################",
    "###########BBBBBBBBBBBB#####BBBBBBBBBB##########SSSSSSSSS###",
    "###########BBBBBBBBBBBB#####BBBBBBBBBB....+.....SSSSSSSSS###",
    "###########BBBBBBBBBBBB..+..BBBBBBBBBB##########SSSSSSSSS###",
    "###########BBBBBBBBBBBB#####BBBBBBBBBB##########SSSSSSSSS###",
    "###########BBBBBBBBBBBB#########.###############SSSSSSSSS###",
    "###########BBBBBBBBBBBB#########+###############SSSSSSSSS###",
    "################.###############.###############SSSSSSSSS###",
    "################+###############.###################.#######",
    "################.###########SSSSSSSSSSSSSSSS########2#######",
    "################.###########SSSSSSSSSSSSSSSS########.#######",
    "###########SSSSSSSSSSSS#####SSSSSSSSSSSSSSSS######TTTTTTTT##",
    "###########SSSSSSSSSSSS#####SSS#SSS#SSS#SSSS######TTTTTTTT##",
    "##LLLLL####SSSSSSSSSSSS#####SSSSSSSSSSSSSSSS######TTTTTTTT##",
    "##LLLLL####SSSSSSSSSSSS#####SSSSSSSSSSSSSSSS######TTTTTTTT##",
    "##LLLLL.+..SSSSSSSSSSSS..+..SSSSSSSSSSSSSSSS..3...TTTTTTTT##",
    "##LLLLL####SSSSSSSSSSSS#####SSS#SSS#SSS#SSSS######TTTTTTTT##",
    "##LLLLL####SSSSSSSSSSSS#####SSSSSSSSSSSSSSSS######TTTTTTTT##",
    "###########SSSSSSSSSSSS#####SSSSSSSSSSSSSSSS######TTTTTTTT##",
    "###########SSSSSSSSSSSS#####SSSSSSSSSSSSSSSS######TTTTTTTT##",
    "###########SSSSSSSSSSSS#####SSS#SSS#SSS#SSSS######TTTTTTTT##",
    "################.###########SSSSSSSSSSSSSSSS########.#######",
    "################+###########SSSSSSSSSSSSSSSS########2#######",
    "################.###############.###################.#######",
    "################.###############.###############SSSSSSSSS###",
    "###########AAAAAAAAAAAA#########+###############SSSSSSSSS###",
    "###########AAAAAAAAAAAA#########.###############SSSSSSSSS###",
    "###########AAAAAAAAAAAA#####AAAAAAAAAA##########SSSSSSSSS###",
    "###########AAAAAAAAAAAA..+..AAAAAAAAAA##########SSSSSSSSS###",
    "###########AAAAAAAAAAAA#####AAAAAAAAAA....+.....SSSSSSSSS###",
    "###########AAAAAAAAAAAA#####AAAAAAAAAA##########SSSSSSSSS###",
    "############################AAAAAAAAAA######################",
    "############################################################",
];
const CARGO_THINGS: &[&str] = &[
    "............................................................",
    "............................................................",
    ".............................y..............................",
    "............s........H........m....M..................h.....",
    ".............m......m...............e.............m.........",
    "............................................................",
    "...............u.......................................M....",
    "....................................................z.......",
    "............................................................",
    "............................................................",
    "............................................................",
    "..................................H.........................",
    "..............................m...........M.................",
    ".......................................................X....",
    "...!..........m......h......................................",
    ".....................................u..............m.......",
    "....@.......................................................",
    ".....................................................G......",
    "............................................................",
    "....................m...............m...................m...",
    ".............e........................................M.....",
    "............................................................",
    "..............................m..........M..................",
    ".................................r..........................",
    "............................................................",
    "............................................................",
    "......................................................u.....",
    "..................................................m.........",
    ".............m......M.......................................",
    "..................a...............A....................M....",
    "............x........o........m....M................v.......",
    "....................................l.......................",
    "............................................................",
    "............................................................",
];
const CITADEL_LAYOUT: &[&str] = &[
    "##################################################################",
    "##########################BBBBBBBBBBBBBB##########################",
    "########AAAAAAAAAAAA######BBBBBBBBBBBBBB######SSSSSSSSSS##########",
    "########AAAAAAAAAAAA######BBBBBBBBBBBBBB######SSSSSSSSSS##########",
    "########AAAAAAAAAAAA..+...BBBBBBBBBBBBBB######SSSSSSSSSS##########",
    "########AAAAAAAAAAAA######BBBBBBBBBBBBBB..+...SSSSSSSSSS##########",
    "########AAAAAAAAAAAA######BBBBBBBBBBBBBB######SSSSSSSSSS##########",
    "########AAAAAAAAAAAA############.#############SSSSSSSSSS##########",
    "########AAAAAAAAAAAA############.#############SSSSSSSSSS##########",
    "###############.################+#############SSSSSSSSSS##########",
    "###############.################.#################.###############",
    "###############+################.#################1###############",
    "###############.################.#################.###############",
    "###############.##########SSSSSSSSSSSSSS##########.###############",
    "###########SSSSSSSSSS#####SSSSS#SSS#SSSS##########.###TTTTTTTTTT##",
    "###########SSSSSSSSSS#####SSSSSSSSSSSSSS##########.###TTTTTTTTTT##",
    "##LLLLL####SSSSSSSSSS#####SSS#SSS#SSS#SS##########....TTTTTTTTTT##",
    "##LLLLL####SSSSSSSSSS#####SSSSSSSSSSSSSS##########.###TTTTTTTTTT##",
    "##LLLLL.+..SSSSSSSSSS..+..SSSSSSSSSSSSSS##########.###TTTTTTTTTT##",
    "##LLLLL####SSSSSSSSSS#####SSSSSSSSSSSSSS....3......###TTTTTTTTTT##",
    "##LLLLL####SSSSSSSSSS#####SSSSSSSSSSSSSS##########.###TTTTTTTTTT##",
    "###########SSSSSSSSSS#####SSS#SSS#SSS#SS##########....TTTTTTTTTT##",
    "###########SSSSSSSSSS#####SSSSSSSSSSSSSS##########.###TTTTTTTTTT##",
    "###########SSSSSSSSSS#####SSSSS#SSS#SSSS##########.###TTTTTTTTTT##",
    "###############.##########SSSSSSSSSSSSSS##########.###############",
    "###############.################.#################.###############",
    "###############+################.#################2###############",
    "###############.################+#################.###############",
    "###############.################.#############SSSSSSSSSS##########",
    "########AAAAAAAAAAAA############.#############SSSSSSSSSS##########",
    "########AAAAAAAAAAAA############.#############SSSSSSSSSS##########",
    "########AAAAAAAAAAAA######BBBBBBBBBBBBBB######SSSSSSSSSS##########",
    "########AAAAAAAAAAAA######BBBBBBBBBBBBBB######SSSSSSSSSS##########",
    "########AAAAAAAAAAAA..+...BBBBBBBBBBBBBB######SSSSSSSSSS##########",
    "########AAAAAAAAAAAA######BBBBBBBBBBBBBB..+...SSSSSSSSSS##########",
    "########AAAAAAAAAAAA######BBBBBBBBBBBBBB######SSSSSSSSSS##########",
    "##########################BBBBBBBBBBBBBB##########################",
    "##################################################################",
];
const CITADEL_THINGS: &[&str] = &[
    "..................................................................",
    "..................................................................",
    "......................................y...........................",
    "...................x........m........M.................H..........",
    "..........m.....................................m.................",
    ".................................u..................m.............",
    ".................M................................................",
    "............o.........................................M...........",
    "...............................................M..z...............",
    "..................................................................",
    "..................................................................",
    "..................................................................",
    "..................................................................",
    "..................................................................",
    ".................................m................................",
    "...................h........m.........M.......................X...",
    "...!.........m..........................................m.........",
    "..................................................................",
    "....@..............M..............u...M.....................G.....",
    "..............................m...........................m.......",
    "..................m........m......................................",
    ".............................................................M....",
    "............m.e.....................M......................M......",
    "..................................m...............................",
    "...........................H...........r..........................",
    "..................................................................",
    "..................................................................",
    "..................................................................",
    "..................................................................",
    ".......................................................h..........",
    "...................a............................m.................",
    "..........m....................................m..................",
    ".....................................................M............",
    ".................M..........m........M................M...........",
    "............v.....................................l...............",
    ".................................e................................",
    "...........................A......................................",
    "..................................................................",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_are_rectangular_and_layers_match() {
        for t in &TEMPLATES {
            let w = t.layout[0].len();
            assert!(t.layout.iter().all(|r| r.len() == w), "{} layout", t.name);
            assert_eq!(t.layout.len(), t.things.len(), "{}", t.name);
            assert!(t.things.iter().all(|r| r.len() == w), "{} things", t.name);
        }
    }

    #[test]
    fn every_map_builds_solvable_for_every_objective_and_flip() {
        for t in &TEMPLATES {
            for &obj in &[
                Objective::Sabotage,
                Objective::CaptureBridge,
                Objective::Rescue,
                Objective::StealData,
                Objective::KillBoss,
            ] {
                for seed in 0..8u64 {
                    let l = t
                        .build(seed, Theme::Station, obj, *t.sizes.end())
                        .unwrap_or_else(|| panic!("{} {obj:?} seed {seed}", t.name));
                    assert!(l.solvable());
                    assert_eq!(l.rooms[0].kind, RoomKind::Airlock);
                    let (sx, sy) = l.rooms[0].center();
                    assert_eq!(l.tile(sx, sy), Tile::Floor, "{} start", t.name);
                    assert!(!l.doors.is_empty());
                    if obj == Objective::Rescue {
                        let n = l
                            .spawns
                            .iter()
                            .filter(|s| matches!(s.kind, SpawnKind::Prisoner))
                            .count();
                        assert_eq!(n, 3);
                    }
                }
            }
        }
    }

    #[test]
    fn locked_doors_exist_and_their_keys_are_on_the_map() {
        for t in &TEMPLATES {
            let l = t
                .build(1, Theme::Station, Objective::StealData, 5)
                .expect("builds");
            for d in l.doors.iter().filter_map(|d| d.lock) {
                assert!(
                    l.spawns
                        .iter()
                        .any(|s| matches!(s.kind, SpawnKind::Item(ItemKind::Key(k)) if k == d)),
                    "{}: no {d:?} key",
                    t.name
                );
            }
        }
    }
}
