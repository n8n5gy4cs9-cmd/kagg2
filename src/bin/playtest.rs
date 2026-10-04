//! Bot playtest report: `cargo run --release --bin playtest -- [seeds] [mission]`.

use kagg2::playtest;
use kagg2::save::Difficulty;

fn main() {
    let mut args = std::env::args()
        .skip(1)
        .filter(|a| a != "boarding" && a != "bench");
    let seeds: u64 = args.next().and_then(|a| a.parse().ok()).unwrap_or(3);
    if std::env::args().any(|a| a == "bench") {
        print!("{}", playtest::bench());
        return;
    }
    if args_is_boarding() {
        print!("{}", playtest::board_survey(seeds));
        return;
    }
    if let Some(mission) = args.next().and_then(|a| a.parse::<u8>().ok()) {
        for seed in 1..=seeds {
            let diff = std::env::var("KAGG2_DIFF")
                .ok()
                .and_then(|d| d.parse::<usize>().ok())
                .map_or(Difficulty::Karl, |i| Difficulty::ALL[i.min(2)]);
            let r = playtest::run_mission(mission, seed, diff, &playtest::typical_carry(mission));
            println!("{r:?}");
        }
        return;
    }
    print!("{}", playtest::survey(seeds));
    for d in Difficulty::ALL {
        let run = playtest::run_campaign(1, d);
        for r in &run {
            println!(
                "  m{:<2} {} {:>5.0}s hull {:>3.0}% ship {:?} credits {}",
                r.mission,
                if r.won { "won " } else { "LOST" },
                r.time,
                r.hull_pct,
                r.carry.up,
                r.carry.credits
            );
        }
        let end = match run.last() {
            Some(r) if !r.won => format!("in mission {}: {}", r.mission, r.reason),
            Some(_) => "in victory".to_owned(),
            None => "-".to_owned(),
        };
        println!(
            "campaign {:<8}: {} missions won, ended {end}",
            d.name().split(' ').next().unwrap_or(""),
            run.iter().filter(|r| r.won).count(),
        );
    }
}

fn args_is_boarding() -> bool {
    std::env::args().any(|a| a == "boarding")
}
