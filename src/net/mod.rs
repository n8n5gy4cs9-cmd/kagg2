//! Multiplayer (T32): a server-authoritative bridge. One server owns the `World` and the mission;
//! each client sits at one or more of the seven stations and sends only `Command`s; the server
//! checks the seat, steps the sim at 60 Hz and sends snapshots at 20 Hz. Stations nobody holds
//! stay on crew AI. Plain `std::net` TCP, no dependencies, no encryption or authentication
//! (LAN play). Headless: the graphical client is not wired yet (see `HANDOFF.md`).

pub mod client;
pub mod proto;
pub mod server;
pub mod wire;

use crate::sim::Command;
use crate::sim::crew::Station;

/// Default TCP port.
pub const DEFAULT_PORT: u16 = 4747;

/// The station that owns a command; only its holder may send it.
pub fn station_of(c: &Command) -> Station {
    use Command as C;
    match c {
        C::SetThrottle(_)
        | C::AdjustThrottle(_)
        | C::Turn { .. }
        | C::Strafe { .. }
        | C::Brake(_)
        | C::SetAssist(_) => Station::Helm,
        C::FireLance
        | C::FireTorpedo
        | C::FireMissile
        | C::SetLanceFreq(_)
        | C::SetShieldFreq(_)
        | C::SetTarget(_) => Station::Weapons,
        C::SetPower(..) | C::SetCoolant(..) | C::PowerPreset(_) => Station::Engineering,
        C::AssignRepair { .. } => Station::Damage,
        C::Scan(_) | C::Sweep => Station::Science,
        C::SetDest(_)
        | C::AddWaypoint(_)
        | C::Autopilot(_)
        | C::Warp(_)
        | C::Dock(_)
        | C::Mine(_) => Station::Map,
        C::Order(_) | C::DemandSurrender(_) => Station::Comms,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::client::Client;
    use crate::net::proto::{ClientMsg, ServerMsg};
    use crate::net::server::{Config, Server};
    use std::time::{Duration, Instant};

    /// Run server polls and client polls until `done` holds (or fail after 5 s).
    fn pump(
        server: &mut Server,
        clients: &mut [&mut Client],
        mut done: impl FnMut(&[&mut Client]) -> bool,
    ) {
        let t0 = Instant::now();
        loop {
            server.poll();
            server.tick();
            for c in clients.iter_mut() {
                c.poll().expect("client connection");
            }
            if done(clients) {
                return;
            }
            assert!(t0.elapsed() < Duration::from_secs(5), "timed out");
            std::thread::sleep(Duration::from_millis(1));
        }
    }

    fn setup() -> (Server, Client, Client) {
        let mut server = Server::bind(
            "127.0.0.1:0",
            Config {
                mission: 1,
                seed: 3,
                difficulty: 1,
            },
        )
        .expect("bind");
        let addr = server.local_addr().expect("addr");
        let mut a = Client::connect(addr, "Ingrid").expect("connect a");
        let mut b = Client::connect(addr, "Bjorn").expect("connect b");
        pump(&mut server, &mut [&mut a, &mut b], |c| {
            c.iter().all(|c| c.player.is_some()) && c[0].lobby.players.len() == 2
        });
        (server, a, b)
    }

    #[test]
    fn every_command_has_a_station() {
        assert_eq!(station_of(&Command::FireLance), Station::Weapons);
        assert_eq!(station_of(&Command::SetThrottle(0.5)), Station::Helm);
        assert_eq!(station_of(&Command::Sweep), Station::Science);
    }

    #[test]
    fn seats_are_exclusive_and_released_on_leave() {
        let (mut server, mut a, mut b) = setup();
        a.send(&ClientMsg::Take(Station::Helm)).expect("send");
        pump(&mut server, &mut [&mut a, &mut b], |c| {
            c[1].lobby.stations[0].is_some()
        });
        let a_id = a.player;
        assert_eq!(b.lobby.stations[Station::Helm as usize], a_id);
        // the second player cannot take a held seat
        b.send(&ClientMsg::Take(Station::Helm)).expect("send");
        pump(&mut server, &mut [&mut a, &mut b], |c| {
            c[1].log.iter().any(|l| l.contains("TAKEN"))
        });
        assert_eq!(b.lobby.stations[Station::Helm as usize], a_id);
        // leaving frees the seat
        a.send(&ClientMsg::Bye).expect("send");
        pump(&mut server, &mut [&mut b], |c| {
            c[0].lobby.players.len() == 1
        });
        assert_eq!(b.lobby.stations[Station::Helm as usize], None);
    }

    #[test]
    fn only_the_seat_holder_commands_the_ship() {
        let (mut server, mut a, mut b) = setup();
        a.send(&ClientMsg::Take(Station::Helm)).expect("send");
        b.send(&ClientMsg::Take(Station::Weapons)).expect("send");
        a.send(&ClientMsg::Ready(true)).expect("send");
        b.send(&ClientMsg::Ready(true)).expect("send");
        pump(&mut server, &mut [&mut a, &mut b], |c| {
            c.iter().all(|c| c.started.is_some())
        });
        // helm throttle from the helm player works; the weapons player cannot steer
        a.send(&ClientMsg::Cmd(Command::SetThrottle(0.8)))
            .expect("send");
        b.send(&ClientMsg::Cmd(Command::SetThrottle(0.0)))
            .expect("send");
        pump(&mut server, &mut [&mut a, &mut b], |c| {
            c[0].snapshot
                .as_ref()
                .and_then(|s| s.ship.as_ref())
                .is_some_and(|s| s.throttle > 0.7)
        });
        assert!(
            b.log.iter().any(|l| l.contains("HELM")),
            "refusal reaches the sender: {:?}",
            b.log
        );
        let snap = a.snapshot.as_ref().expect("snapshot");
        assert!(snap.ents.len() > 5 && snap.player.is_some());
        assert!(!snap.objective.is_empty());
    }

    #[test]
    fn mismatched_protocol_is_rejected() {
        let mut server = Server::bind(
            "127.0.0.1:0",
            Config {
                mission: 1,
                seed: 1,
                difficulty: 1,
            },
        )
        .expect("bind");
        let addr = server.local_addr().expect("addr");
        let mut c = Client::connect_with_protocol(addr, "Old", 999).expect("connect");
        pump(&mut server, &mut [&mut c], |c| c[0].rejected.is_some());
        assert!(
            c.rejected
                .as_deref()
                .is_some_and(|r| r.contains("PROTOCOL"))
        );
    }

    #[test]
    fn garbage_and_oversize_frames_drop_the_connection() {
        use std::io::Write;
        let mut server = Server::bind(
            "127.0.0.1:0",
            Config {
                mission: 1,
                seed: 1,
                difficulty: 1,
            },
        )
        .expect("bind");
        let addr = server.local_addr().expect("addr");
        let mut raw = std::net::TcpStream::connect(addr).expect("connect");
        raw.write_all(&u32::MAX.to_le_bytes()).expect("write");
        let t0 = Instant::now();
        while server.connections() > 0 || t0.elapsed() < Duration::from_millis(50) {
            server.poll();
            if t0.elapsed() > Duration::from_secs(2) {
                break;
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        assert_eq!(server.connections(), 0);
        let _ = ServerMsg::Notice(String::new());
    }
}
