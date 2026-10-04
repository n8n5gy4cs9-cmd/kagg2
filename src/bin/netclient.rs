//! Text-mode test client: `netclient HOST:PORT NAME STATION[,STATION...] [SECS]`.
//! Takes the stations, readies up, and prints the flagship's status once a second. A helm client
//! also holds a steady 60 % throttle. Stations: helm weapons engineering damage science comms map.

use std::net::ToSocketAddrs;
use std::time::{Duration, Instant};

use kagg2::net::client::Client;
use kagg2::net::proto::ClientMsg;
use kagg2::sim::Command;
use kagg2::sim::crew::Station;

fn station(name: &str) -> Option<Station> {
    Station::ALL
        .into_iter()
        .find(|s| s.name().eq_ignore_ascii_case(name))
}

fn main() -> std::io::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (Some(addr), Some(name)) = (args.first(), args.get(1)) else {
        eprintln!("usage: netclient HOST:PORT NAME STATION[,STATION...] [SECS]");
        std::process::exit(2);
    };
    let Some(addr) = addr.to_socket_addrs()?.next() else {
        eprintln!("cannot resolve {addr}");
        std::process::exit(2);
    };
    let seats: Vec<Station> = args
        .get(2)
        .map(|s| s.split(',').filter_map(station).collect())
        .unwrap_or_default();
    let secs: u64 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(30);
    let mut c = Client::connect(addr, name)?;
    for s in &seats {
        c.send(&ClientMsg::Take(*s))?;
    }
    c.send(&ClientMsg::Ready(true))?;
    let (t0, mut last) = (Instant::now(), Instant::now());
    while t0.elapsed() < Duration::from_secs(secs) && c.ended.is_none() {
        c.poll()?;
        if let Some(why) = &c.rejected {
            eprintln!("rejected: {why}");
            break;
        }
        if seats.contains(&Station::Helm)
            && c.started.is_some()
            && last.elapsed() > Duration::from_millis(500)
        {
            c.send(&ClientMsg::Cmd(Command::SetThrottle(0.6)))?;
        }
        if last.elapsed() > Duration::from_secs(1) {
            last = Instant::now();
            if let Some(s) = &c.snapshot {
                let hull = s.ship.as_ref().map_or(0.0, |x| x.hull);
                println!(
                    "t={:.0}s stage {}/{} \"{}\" hull {hull:.0} entities {}",
                    s.time,
                    s.stage.0,
                    s.stage.1,
                    s.objective,
                    s.ents.len()
                );
            } else {
                println!("lobby: {} players", c.lobby.players.len());
            }
            for l in c.log.drain(..) {
                println!("  {l}");
            }
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    if let Some((won, why)) = &c.ended {
        println!("mission {} {why}", if *won { "won" } else { "lost" });
    }
    Ok(())
}
