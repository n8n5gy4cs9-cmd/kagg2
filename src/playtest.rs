//! Headless playtest harness (T30): the bots play whole campaign missions through the same sim,
//! mission engine and boarding code the game uses, and report win rates, times and losses.
//! Run `cargo run --release --bin playtest`.

use crate::board::bot as board_bot;
use crate::board::{Objective, Outcome, generate, theme_for};
use crate::save::Difficulty;
use crate::sim::bot::SpaceBot;
use crate::sim::mission::{Carry, MissionState};
use crate::sim::{EntityId, Event, SectorId, boarding, new_game};

/// Longest a mission may run before the bot is declared stuck (game seconds).
pub const MAX_MISSION_TIME: f32 = 1200.0;

/// One bot-played mission.
#[derive(Debug, Clone)]
pub struct Report {
    pub mission: u8,
    pub won: bool,
    /// Loss reason, or `"STUCK"` when the time cap hit, empty on a win.
    pub reason: String,
    pub time: f32,
    pub stage: (usize, usize),
    /// Hull left at the end, `0..=100`.
    pub hull_pct: f32,
    pub kills: u32,
    pub boardings: u32,
    pub boarding_wins: u32,
    pub carry: Carry,
}

/// Play mission `mission` with `seed` at `difficulty`, starting from `carry`.
pub fn run_mission(mission: u8, seed: u64, difficulty: Difficulty, carry: &Carry) -> Report {
    let mut w = new_game(seed, SectorId::for_mission(mission));
    w.difficulty = difficulty.scale();
    carry.apply(&mut w);
    let mut m = MissionState::new(mission);
    m.begin(&mut w);
    let mut bot = SpaceBot::default();
    bot.set_crew(&mut w);
    let (mut kills, mut boardings, mut board_wins) = (0, 0, 0);
    let mut stuck = "";
    let mut attempts: Vec<(EntityId, u32)> = Vec::new();
    let trace = std::env::var_os("KAGG2_TRACE").is_some();
    while m.result().is_none() {
        if w.time > MAX_MISSION_TIME {
            stuck = "STUCK";
            break;
        }
        let goal = m.goal();
        if trace && w.tick.is_multiple_of(60 * 20) {
            let me = w.player.and_then(|p| w.get(p));
            let o = me.and_then(|e| e.ops.as_deref());
            eprintln!(
                "t={:.0} stage={:?} goal={goal:?} pos={:?} hull={:.0} dest={:?} ap={:?} dock={:?} foes={}",
                w.time,
                m.progress(),
                me.map(|e| e.pos),
                o.map_or(0.0, |o| o.hull),
                o.map(|o| o.nav.dest),
                o.map(|o| o.nav.autopilot),
                o.map(|o| o.docked.is_some()),
                w.iter()
                    .filter(|(_, e)| e.faction == crate::sim::ops::Faction::Minion
                        && e.kind == crate::sim::Kind::Ship)
                    .count()
            );
            for (_, f) in w.iter().filter(|(_, e)| {
                e.faction == crate::sim::ops::Faction::Minion && e.kind == crate::sim::Kind::Ship
            }) {
                let me_pos = me.map_or(glam::Vec3::ZERO, |e| e.pos);
                let o = f.ops.as_deref();
                eprintln!(
                    "   foe {} dist={:.0} hull={:?} sh={:?} dis={:?} ai={:?} align={:.3} heat={:?} tgt={:?}",
                    f.label,
                    f.pos.distance(me_map(me_pos)),
                    o.map(|o| o.hull),
                    o.map(|o| o.shields),
                    o.map(|o| o.disabled),
                    f.ai.map(|a| a.mode),
                    me.map_or(0.0, |e| e.forward().angle_between(f.pos - e.pos)),
                    me.and_then(|e| e.ops.as_deref()).map(|o| o.sys.heat),
                    me.and_then(|e| e.ops.as_deref())
                        .map(|o| o.target.is_some())
                );
            }
        }
        let dec = bot.think(&mut w, goal);
        let events = w.step(&dec.cmds);
        kills += events
            .iter()
            .filter(|e| matches!(e, Event::Destroyed { label, .. } if !label.starts_with("Hauler") && *label != "Skiff"))
            .count() as u32;
        m.update(&mut w, &events);
        let alive = w
            .player
            .and_then(|p| w.get(p))
            .and_then(|e| e.ops.as_deref())
            .is_some_and(|o| !o.dead);
        if !alive {
            m.fail("YOUR SHIP WAS DESTROYED");
        }
        let Some(target) = dec.board else { continue };
        let tries = attempts.iter().position(|a| a.0 == target);
        if tries.is_some_and(|i| attempts[i].1 >= 3) {
            continue;
        }
        match tries {
            Some(i) => attempts[i].1 += 1,
            None => attempts.push((target, 1)),
        }
        let Some((label, class_name)) = w
            .get(target)
            .and_then(|t| Some((t.label, t.ops.as_deref()?.class_name)))
        else {
            continue;
        };
        let bseed = w.tick ^ (u64::from(target.index) << 32);
        let (theme, size) = theme_for(label, class_name, bseed);
        let objective = m
            .board_wanted(label)
            .unwrap_or_else(|| Objective::for_theme(theme)[0]);
        let level = generate(bseed, theme, objective, size);
        // capital ships are boarded with AI shield-brothers (T33): one on size 3, two from size 4
        let mates = size.saturating_sub(2).min(2) as usize;
        let run = board_bot::play_team(level, bseed, difficulty.scale(), 0.85, 420.0, mates);
        if trace {
            eprintln!("boarding {label} {theme:?} size {size} {objective:?}: {run:?}");
        }
        let won = run.outcome == Outcome::Won;
        let credits = run.kills as i32 * 15 + if won && run.objective_done { 300 } else { 0 };
        m.on_boarding(label, objective, won && run.objective_done);
        boarding::apply(&mut w, target, won, credits, run.damage_taken);
        boardings += 1;
        board_wins += u32::from(won && run.objective_done);
    }
    let hull_pct = w
        .player
        .and_then(|p| w.get(p))
        .and_then(|e| e.ops.as_deref())
        .map_or(0.0, |o| 100.0 * o.hull.max(0.0) / o.max_hull.max(1.0));
    let won = m.result() == Some(true);
    let reason = if won {
        ""
    } else if m.reason().is_empty() {
        stuck
    } else {
        m.reason()
    };
    Report {
        mission,
        won,
        reason: reason.to_owned(),
        time: w.time,
        stage: m.progress(),
        hull_pct,
        kills,
        boardings,
        boarding_wins: board_wins,
        carry: Carry::from_world(&w).unwrap_or_default(),
    }
}

/// A player who docks at a starbase between missions and spends everything: upgrades are bought
/// round-robin (shields, lances, hull, reactor, bays) at 400 / 900 credits per level, keeping
/// `keep` credits back (mission 6 needs one purchase in its own shipyard stage).
pub fn spend_credits(carry: &mut Carry, keep: i32) {
    const ORDER: [usize; 5] = [1, 0, 3, 2, 4];
    let mut bought = true;
    while bought {
        bought = false;
        for i in ORDER {
            let cost = match carry.up[i] {
                1 => 400,
                2 => 900,
                _ => continue,
            };
            if carry.credits - keep >= cost {
                carry.credits -= cost;
                carry.up[i] += 1;
                bought = true;
            }
        }
    }
}

/// The ship a player typically brings to `mission` (used by the per-mission survey).
pub fn typical_carry(mission: u8) -> Carry {
    let up = match mission {
        1..=3 => [1, 1, 1, 1, 1],
        4 | 5 => [2, 2, 1, 1, 1],
        6 | 7 => [2, 2, 1, 2, 1],
        8 | 9 => [2, 2, 2, 2, 2],
        _ => [3, 2, 2, 2, 2],
    };
    Carry { credits: 500, up }
}

/// Play missions 1–10 in order, carrying credits and upgrades over (and spending them between
/// missions); stops at the first loss.
pub fn run_campaign(seed: u64, difficulty: Difficulty) -> Vec<Report> {
    let mut carry = Carry::default();
    let mut out = Vec::new();
    for id in 1..=10u8 {
        let r = run_mission(id, seed.wrapping_add(u64::from(id)), difficulty, &carry);
        carry = r.carry;
        spend_credits(&mut carry, if id == 5 { 450 } else { 0 });
        let won = r.won;
        out.push(r);
        if !won {
            break;
        }
    }
    out
}

/// Win-rate table over `seeds` for every mission and difficulty (each with [`typical_carry`]).
pub fn survey(seeds: u64) -> String {
    let mut s =
        String::from("mission  difficulty  wins  avg-time  avg-hull  avg-kills  boardings\n");
    for id in 1..=10u8 {
        for d in Difficulty::ALL {
            let runs: Vec<Report> = (1..=seeds)
                .map(|seed| run_mission(id, seed, d, &typical_carry(id)))
                .collect();
            let n = runs.len() as f32;
            let wins = runs.iter().filter(|r| r.won).count();
            let avg = |f: &dyn Fn(&Report) -> f32| runs.iter().map(f).sum::<f32>() / n;
            s.push_str(&format!(
                "{id:>7}  {:<10}  {wins}/{}  {:>7.0}s  {:>7.0}%  {:>9.1}  {}/{}\n",
                d.name().split(' ').next().unwrap_or(""),
                runs.len(),
                avg(&|r| r.time),
                avg(&|r| r.hull_pct),
                avg(&|r| r.kills as f32),
                runs.iter().map(|r| r.boarding_wins).sum::<u32>(),
                runs.iter().map(|r| r.boardings).sum::<u32>(),
            ));
            let fails: Vec<&str> = runs
                .iter()
                .filter(|r| !r.won)
                .map(|r| r.reason.as_str())
                .collect();
            if !fails.is_empty() {
                s.push_str(&format!("          failures: {}\n", fails.join(" | ")));
            }
        }
    }
    s
}

fn me_map(v: glam::Vec3) -> glam::Vec3 {
    v
}

/// Boarding win rates per theme × size × difficulty (first objective of the theme).
pub fn board_survey(seeds: u64) -> String {
    use crate::board::Theme;
    let mut s = String::from("theme             size  difficulty  wins  avg-time  avg-damage\n");
    for theme in [
        Theme::Corvette,
        Theme::PrisonBarge,
        Theme::Station,
        Theme::DreadnoughtBridge,
    ] {
        for size in [1u32, 3, 5] {
            for d in Difficulty::ALL {
                let runs: Vec<board_bot::BotRun> = (1..=seeds)
                    .map(|seed| {
                        let objective = Objective::for_theme(theme)
                            [(seed as usize) % Objective::for_theme(theme).len()];
                        let level = generate(seed, theme, objective, size);
                        board_bot::play(level, seed, d.scale(), 0.85, 420.0)
                    })
                    .collect();
                let n = runs.len() as f32;
                let wins = runs
                    .iter()
                    .filter(|r| r.outcome == Outcome::Won && r.objective_done)
                    .count();
                s.push_str(&format!(
                    "{:<16}  {size:>4}  {:<10}  {wins}/{}  {:>7.0}s  {:>9.0}\n",
                    theme.name(),
                    d.name().split(' ').next().unwrap_or(""),
                    runs.len(),
                    runs.iter().map(|r| r.time).sum::<f32>() / n,
                    runs.iter().map(|r| r.damage_taken).sum::<f32>() / n,
                ));
            }
        }
    }
    s
}

/// Timing of the headless sim (T31): µs per 60 Hz step in a busy fight and in a big boarding
/// level. The frame budget at 60 fps is 16 667 µs and rendering shares it.
pub fn bench() -> String {
    use std::time::Instant;
    let mut out = String::from("scenario                          steps   avg µs   max µs  note\n");
    // space: mission 10 (armada) with the bot fighting
    let mut w = new_game(7, SectorId::for_mission(10));
    let mut m = MissionState::new(10);
    m.begin(&mut w);
    let mut bot = SpaceBot::default();
    bot.set_crew(&mut w);
    let (mut total, mut max, mut n, mut peak) = (0u128, 0u128, 0u32, 0usize);
    for _ in 0..3600 {
        let dec = bot.think(&mut w, m.goal());
        let t = Instant::now();
        let ev = w.step(&dec.cmds);
        m.update(&mut w, &ev);
        let us = t.elapsed().as_micros();
        total += us;
        max = max.max(us);
        n += 1;
        peak = peak.max(w.len());
    }
    out.push_str(&format!(
        "space: mission 10 armada      {n:>7}  {:>7}  {max:>7}  {peak} entities at peak\n",
        total / u128::from(n.max(1))
    ));
    // boarding: size-5 dreadnought with a party of three
    let level = generate(
        3,
        crate::board::Theme::DreadnoughtBridge,
        Objective::CaptureBridge,
        5,
    );
    let mut bw = crate::board::BoardWorld::new(level, 3);
    bw.add_mates(2);
    let mut bots: Vec<board_bot::BoardBot> =
        (0..3).map(|_| board_bot::BoardBot::new(0.85)).collect();
    let (mut total, mut max, mut n) = (0u128, 0u128, 0u32);
    while bw.outcome == Outcome::InProgress && n < 3600 {
        let inputs: Vec<_> = bots
            .iter_mut()
            .enumerate()
            .map(|(k, b)| bw.with_player(k, |w| b.act(w)))
            .collect();
        let t = Instant::now();
        bw.step_team(1.0 / 60.0, &inputs);
        let us = t.elapsed().as_micros();
        total += us;
        max = max.max(us);
        n += 1;
    }
    out.push_str(&format!(
        "boarding: size 5, party of 3  {n:>7}  {:>7}  {max:>7}  {} monsters\n",
        total / u128::from(n.max(1)),
        bw.monsters.len()
    ));
    out
}

/// Play one mission at an explicit difficulty (used by the CLI's `mission difficulty` form).
pub fn run_one(mission: u8, seed: u64, difficulty: Difficulty) -> Report {
    run_mission(mission, seed, difficulty, &Carry::default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bot_wins_the_tutorial_and_the_example_script() {
        let r = run_one(1, 2, Difficulty::Karl);
        assert!(r.won, "tutorial: {r:?}");
        let src = include_str!("../missions/wolf-pack.k2m");
        let id = crate::sim::script::register(crate::sim::script::parse(src).expect("parses"))
            .expect("registered");
        let r = run_mission(id, 1, Difficulty::Karl, &Carry::default());
        assert!(r.won, "wolf-pack: {r:?}");
    }

    #[test]
    fn the_bot_boards_the_courier_in_mission_four() {
        let r = run_mission(4, 1, Difficulty::Karl, &typical_carry(4));
        assert!(r.won && r.boardings >= 1 && r.boarding_wins >= 1, "{r:?}");
    }

    #[test]
    fn credits_are_spent_round_robin_and_a_reserve_is_kept() {
        let mut c = Carry {
            credits: 1000,
            up: [1; 5],
        };
        spend_credits(&mut c, 0);
        assert_eq!(c.up, [2, 2, 1, 1, 1]);
        assert_eq!(c.credits, 200);
        let mut c = Carry {
            credits: 1000,
            up: [1; 5],
        };
        spend_credits(&mut c, 650);
        assert_eq!(c.up.iter().map(|&u| u32::from(u)).sum::<u32>(), 5);
        assert_eq!(c.credits, 1000);
    }

    #[test]
    fn a_hopeless_start_is_reported_not_hung() {
        // an empty sector with no player ship ends as a loss, never an endless loop
        let mut w = new_game(1, SectorId::MarsOrbit);
        if let Some(p) = w.player.take() {
            w.despawn(p);
        }
        let mut bot = SpaceBot::default();
        assert!(bot.think(&mut w, None).cmds.is_empty());
    }
}
