//! Headless multiplayer server: `cargo run --release --bin server -- [--port 4747] [--mission 2]
//! [--seed 1] [--difficulty 0-2] [--script file.k2m]`. Clients take stations, press ready, and
//! the mission starts when everybody is ready.

use kagg2::net::DEFAULT_PORT;
use kagg2::net::server::{Config, Server};
use kagg2::sim::script;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let get = |flag: &str| {
        args.iter()
            .position(|a| a == flag)
            .and_then(|i| args.get(i + 1))
            .cloned()
    };
    let port: u16 = get("--port")
        .and_then(|p| p.parse().ok())
        .unwrap_or(DEFAULT_PORT);
    let mut mission: u8 = get("--mission").and_then(|p| p.parse().ok()).unwrap_or(1);
    if let Some(path) = get("--script") {
        match script::load_file(std::path::Path::new(&path)) {
            Ok(id) => mission = id,
            Err(e) => {
                eprintln!("cannot load {path}: {e}");
                std::process::exit(2);
            }
        }
    }
    let cfg = Config {
        mission,
        seed: get("--seed").and_then(|p| p.parse().ok()).unwrap_or(1),
        difficulty: get("--difficulty")
            .and_then(|p| p.parse().ok())
            .unwrap_or(1),
    };
    match Server::bind(("0.0.0.0", port), cfg) {
        Ok(mut s) => {
            println!("Kagg2 server on port {port}, mission {mission} (LAN only: no encryption)");
            s.run();
            println!("mission over");
        }
        Err(e) => {
            eprintln!("cannot listen on port {port}: {e}");
            std::process::exit(1);
        }
    }
}
