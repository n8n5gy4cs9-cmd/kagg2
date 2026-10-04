//! Headless boarding bot (T30): walks a lock-aware path to the objective, shoots what it sees,
//! picks up keys/health/weapons on the way and returns to the airlock. Used by the playtest
//! harness and by the solvability tests; it never touches `macroquad`.

use std::collections::VecDeque;

use glam::Vec3;

use super::level::{ItemKind, Level, Objective, Tile};
use super::pmove::Input;
use super::weapons::{Ammo, Weapon};
use super::world::{BoardWorld, Outcome};

/// Longest detour (tiles) the bot takes for a wanted pickup.
const DETOUR: i32 = 40;

/// Fixed step the scene uses (60 Hz).
const DT: f32 = 1.0 / 60.0;

/// Result of one bot-played boarding action.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BotRun {
    pub outcome: Outcome,
    pub objective_done: bool,
    pub kills: u32,
    pub damage_taken: f32,
    /// Game seconds the action took.
    pub time: f32,
}

type Tile2 = (i32, i32);

/// Breadth-first distances (in tiles) from `from`; locked doors without the key block.
fn distances(level: &Level, from: Tile2, keys: [bool; 3]) -> Vec<i32> {
    let mut dist = vec![-1; (level.w * level.h) as usize];
    let mut q = VecDeque::new();
    dist[(from.1 * level.w + from.0) as usize] = 0;
    q.push_back(from);
    while let Some((x, y)) = q.pop_front() {
        let d = dist[(y * level.w + x) as usize];
        for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            let (nx, ny) = (x + dx, y + dy);
            let i = (ny * level.w + nx) as usize;
            if level.tile(nx, ny) != Tile::Floor || dist[i] >= 0 {
                continue;
            }
            if let Some(di) = level.door_at(nx, ny)
                && level.doors[di].lock.is_some_and(|k| !keys[k as usize])
            {
                continue;
            }
            dist[i] = d + 1;
            q.push_back((nx, ny));
        }
    }
    dist
}

/// The neighbour of `from` that is one step closer to the BFS source of `dist`.
fn step_toward(level: &Level, dist: &[i32], from: Tile2) -> Option<Tile2> {
    let here = dist[(from.1 * level.w + from.0) as usize];
    [(1, 0), (-1, 0), (0, 1), (0, -1)]
        .iter()
        .map(|(dx, dy)| (from.0 + dx, from.1 + dy))
        .find(|&(x, y)| {
            level.tile(x, y) == Tile::Floor && {
                let d = dist[(y * level.w + x) as usize];
                d >= 0 && d < here
            }
        })
}

/// A scripted player. `skill` (`0..=1`) scales aim error.
#[derive(Debug, Clone)]
pub struct BoardBot {
    pub skill: f32,
    tick: u32,
}

impl BoardBot {
    pub fn new(skill: f32) -> BoardBot {
        BoardBot {
            skill: skill.clamp(0.0, 1.0),
            tick: 0,
        }
    }

    fn pick_weapon(w: &BoardWorld, dist: f32) -> Option<usize> {
        let p = &w.player;
        let has = |wp: Weapon| p.weapons[wp.idx()];
        let ammo = |a: Ammo| p.ammo_of(a) > 0;
        let choice = if has(Weapon::Rattler) && ammo(Ammo::Bullets) {
            Weapon::Rattler
        } else if has(Weapon::Scatter) && ammo(Ammo::Shells) && dist < 14.0 {
            Weapon::Scatter
        } else if has(Weapon::Valkyrie) && ammo(Ammo::Slugs) {
            Weapon::Valkyrie
        } else if has(Weapon::Scatter) && ammo(Ammo::Shells) {
            Weapon::Scatter
        } else if has(Weapon::Mjolnir) && ammo(Ammo::Rockets) && dist > 9.0 {
            Weapon::Mjolnir
        } else {
            Weapon::Blaster
        };
        (choice != p.current).then_some(choice.idx())
    }

    /// Where to go next: `(world position, wants_use)`.
    fn destination(&self, w: &BoardWorld, here: Tile2) -> (Vec3, bool) {
        let l = &w.level;
        let p = &w.player;
        let airlock = l.centre(l.rooms[0].center().0, l.rooms[0].center().1);
        if w.progress.done {
            return (airlock, true);
        }
        let reach = distances(l, here, p.keys);
        let steps = |pos: Vec3| {
            let t = l.tile_of(pos);
            reach[(t.1 * l.w + t.0) as usize]
        };
        let nearest = |cands: Vec<Vec3>| {
            cands
                .into_iter()
                .filter(|&c| steps(c) >= 0)
                .min_by_key(|&c| steps(c))
        };
        let goals: Vec<Vec3> = match l.objective {
            Objective::Rescue => w
                .prisoners
                .iter()
                .filter(|q| !q.freed)
                .map(|q| q.pos)
                .collect(),
            Objective::StealData => w
                .items
                .iter()
                .filter(|i| !i.taken && i.kind == ItemKind::DataChip)
                .map(|i| i.pos)
                .collect(),
            _ => vec![l.centre(l.goal.0, l.goal.1)],
        };
        // the objective, or - when it is locked away - the nearest reachable key
        let (goal, is_goal) = match nearest(goals) {
            Some(g) => (g, true),
            None => {
                let keys_at = w
                    .items
                    .iter()
                    .filter(|i| !i.taken && matches!(i.kind, ItemKind::Key(_)))
                    .map(|i| i.pos)
                    .collect();
                (nearest(keys_at).unwrap_or(airlock), false)
            }
        };
        // detours: health when hurt, a missing weapon, armour, low ammo - only if close
        let wanted = |k: ItemKind| match k {
            ItemKind::HealthSmall | ItemKind::HealthLarge => p.health < p.max_health - 12.0,
            ItemKind::Weapon(wp) => !p.weapons[wp.idx()],
            ItemKind::ArmorJacket | ItemKind::ArmorBody => p.armor < 100,
            ItemKind::Shells => p.ammo_of(Ammo::Shells) < 40,
            ItemKind::Bullets => p.ammo_of(Ammo::Bullets) < 100,
            ItemKind::Rockets => p.ammo_of(Ammo::Rockets) < 10,
            ItemKind::Slugs => p.ammo_of(Ammo::Slugs) < 10,
            _ => false,
        };
        let extras = w
            .items
            .iter()
            .filter(|i| !i.taken && wanted(i.kind) && (0..=DETOUR).contains(&steps(i.pos)))
            .map(|i| i.pos)
            .collect();
        match nearest(extras) {
            Some(e) => (e, false),
            None => (goal, is_goal),
        }
    }

    /// One 60 Hz decision: returns the input and turns the player (yaw/pitch are set directly).
    pub fn act(&mut self, w: &mut BoardWorld) -> Input {
        self.tick += 1;
        let here = w.level.tile_of(w.player.pos);
        let mut input = Input::default();
        // fight: nearest living monster in line of sight
        let eye = w.player.eye();
        let foe = w
            .monsters
            .iter()
            .filter(|m| m.alive() && w.level.los(eye, m.centre()))
            .map(|m| (m.centre(), m.centre().distance(eye)))
            .filter(|&(_, d)| d < 30.0)
            .min_by(|a, b| a.1.total_cmp(&b.1));
        let (dest_pos, wants_use) = self.destination(w, here);
        let dest = w.level.tile_of(dest_pos);
        let dist_map = distances(&w.level, dest, w.player.keys);
        let next = step_toward(&w.level, &dist_map, here);
        let heading_to = |from: Vec3, to: Vec3| {
            let d = to - from;
            (-d.x).atan2(-d.z)
        };
        if let Some((fpos, fd)) = foe {
            let jitter = (1.0 - self.skill) * 0.06;
            let n = (self.tick.wrapping_mul(2_654_435_761) >> 8) as f32 / (1u32 << 24) as f32;
            w.player.yaw = heading_to(eye, fpos) + (n - 0.5) * jitter;
            let flat = Vec3::new(fpos.x - eye.x, 0.0, fpos.z - eye.z)
                .length()
                .max(0.1);
            w.player.pitch = ((fpos.y - eye.y).atan2(flat) + (n - 0.5) * jitter).clamp(-1.2, 1.2);
            input.weapon = Self::pick_weapon(w, fd);
            // scatter shells are wasted at range; the blaster is fine anywhere
            let cur = input
                .weapon
                .and_then(|i| Weapon::ALL.get(i))
                .copied()
                .unwrap_or(w.player.current);
            input.fire = cur != Weapon::Scatter || fd < 16.0;
            // strafe while fighting; close the gap only when the target is far
            input.right = if (self.tick / 40).is_multiple_of(2) {
                1.0
            } else {
                -1.0
            };
            input.forward = if fd > 12.0 { 1.0 } else { 0.0 };
        } else {
            // in the destination tile walk to the exact spot (pickups have a small radius)
            let target = next.map_or(dest_pos, |n| w.level.centre(n.0, n.1));
            w.player.yaw = heading_to(w.player.pos, target);
            w.player.pitch = 0.0;
            input.forward = 1.0;
        }
        let at_dest = dest_pos.distance(w.player.pos) < 2.4;
        if wants_use && (at_dest || w.progress.done) && self.tick.is_multiple_of(20) {
            input.use_key = true;
        }
        input
    }
}

/// Play one boarding action to its end (or until `max_time` game seconds pass).
pub fn play(level: Level, seed: u64, difficulty: f32, skill: f32, max_time: f32) -> BotRun {
    play_team(level, seed, difficulty, skill, max_time, 0)
}

/// Like [`play`], with `mates` bot-driven co-op partners beside the host (T33).
pub fn play_team(
    level: Level,
    seed: u64,
    difficulty: f32,
    skill: f32,
    max_time: f32,
    mates: usize,
) -> BotRun {
    let mut w = BoardWorld::new(level, seed);
    w.difficulty = difficulty;
    w.add_mates(mates);
    let health0 = w.player.health;
    let mut bots: Vec<BoardBot> = (0..=mates).map(|_| BoardBot::new(skill)).collect();
    while w.outcome == Outcome::InProgress && w.time < max_time {
        let inputs: Vec<Input> = bots
            .iter_mut()
            .enumerate()
            .map(|(k, bot)| w.with_player(k, |w| bot.act(w)))
            .collect();
        w.step_team(DT, &inputs);
    }
    BotRun {
        outcome: w.outcome,
        objective_done: w.progress.done,
        kills: w.progress.kills,
        damage_taken: (health0 - w.player.health).max(0.0),
        time: w.time,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::level::{Theme, generate};

    fn wins(theme: Theme, objective: Objective, size: u32, difficulty: f32, seeds: u64) -> u64 {
        (1..=seeds)
            .filter(|&seed| {
                let l = generate(seed, theme, objective, size);
                let r = play(l, seed, difficulty, 0.85, 420.0);
                r.outcome == Outcome::Won && r.objective_done
            })
            .count() as u64
    }

    #[test]
    fn bot_wins_most_small_corvette_boardings_on_karl() {
        let n = wins(Theme::Corvette, Objective::StealData, 1, 1.0, 8);
        assert!(n >= 6, "bot only won {n}/8");
    }

    #[test]
    fn harder_difficulty_does_not_help_the_bot() {
        let thrall = wins(Theme::Corvette, Objective::CaptureBridge, 3, 0.7, 8);
        let jarl = wins(Theme::Corvette, Objective::CaptureBridge, 3, 1.4, 8);
        assert!(thrall >= jarl, "thrall {thrall} < jarl {jarl}");
    }

    #[test]
    fn sabotage_fuse_leaves_time_to_walk_back() {
        // the fuse scales with the level, so the bot can finish deep sabotage runs
        let l = generate(2, Theme::Corvette, Objective::Sabotage, 3);
        let r = play(l, 2, 0.7, 0.9, 420.0);
        assert!(r.outcome != Outcome::InProgress, "bot got stuck: {r:?}");
    }
}

#[cfg(test)]
mod coop_tests {
    use super::*;
    use crate::board::level::{Theme, generate};

    #[test]
    fn a_party_of_two_does_at_least_as_well_as_one_on_a_mid_level() {
        let wins = |mates: usize| {
            (1..=8u64)
                .filter(|&seed| {
                    let l = generate(seed, Theme::Corvette, Objective::CaptureBridge, 3);
                    let r = play_team(l, seed, 1.0, 0.85, 420.0, mates);
                    r.outcome == Outcome::Won && r.objective_done
                })
                .count()
        };
        let (solo, duo) = (wins(0), wins(1));
        assert!(duo >= solo, "duo {duo} < solo {solo}");
    }
}
