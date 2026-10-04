//! Messages exchanged between server and clients (T32) and their encoding.

use glam::Vec3;

use super::wire::{
    Reader, WireError, Writer, get_command, put_command, station_code, station_from,
};
use crate::sim::crew::Station;
use crate::sim::world::Kind;
use crate::sim::{Command, EntityId};

/// Bumped whenever the wire format changes; a mismatch is rejected at `Hello`.
pub const PROTOCOL: u16 = 1;
/// Longest player name kept (bytes).
pub const MAX_NAME: usize = 24;

type Res<T> = Result<T, WireError>;

#[derive(Debug, Clone, PartialEq)]
pub enum ClientMsg {
    Hello {
        name: String,
        protocol: u16,
    },
    /// Sit down at a station (fails if somebody else has it).
    Take(Station),
    /// Leave a station (it goes back to crew AI).
    Release(Station),
    Ready(bool),
    /// A bridge command; the server checks the sender holds the right station.
    Cmd(Command),
    Bye,
}

/// One row of the lobby.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Seat {
    pub player: u8,
    pub name: String,
    pub ready: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LobbyState {
    pub players: Vec<Seat>,
    /// Who holds each station (`Station::ALL` order); `None` = crew AI.
    pub stations: [Option<u8>; 7],
}

/// One entity as clients see it.
#[derive(Debug, Clone, PartialEq)]
pub struct EntSnap {
    pub id: EntityId,
    pub kind: u8,
    pub faction: u8,
    pub pos: Vec3,
    pub vel: Vec3,
    pub rot: [f32; 4],
    pub radius: f32,
    /// Hull `0..=100` (100 for scenery).
    pub hull_pct: u8,
    pub label: String,
}

/// The flagship's vitals, for the station panels.
#[derive(Debug, Clone, PartialEq)]
pub struct ShipSnap {
    pub hull: f32,
    pub max_hull: f32,
    pub shields: [f32; 4],
    pub torpedoes: u8,
    pub missiles: u8,
    pub throttle: f32,
    pub target: Option<EntityId>,
    pub docked: bool,
    pub credits: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Snapshot {
    pub tick: u64,
    pub time: f32,
    pub player: Option<EntityId>,
    pub ship: Option<ShipSnap>,
    pub objective: String,
    /// `(current, total)` mission stage.
    pub stage: (u8, u8),
    pub ents: Vec<EntSnap>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ServerMsg {
    Welcome {
        player: u8,
    },
    Reject(String),
    Lobby(LobbyState),
    Start {
        seed: u64,
        mission: u8,
        difficulty: u8,
    },
    Snapshot(Snapshot),
    /// A line for the message log (command refused, player left, mission captions, ...).
    Notice(String),
    End {
        won: bool,
        reason: String,
    },
}

pub fn kind_code(k: Kind) -> u8 {
    k as u8
}

impl ClientMsg {
    pub fn encode(&self) -> Vec<u8> {
        let mut w = Writer::default();
        match self {
            ClientMsg::Hello { name, protocol } => {
                w.u8(0).u16(*protocol).str(name);
            }
            ClientMsg::Take(s) => {
                w.u8(1).u8(station_code(*s));
            }
            ClientMsg::Release(s) => {
                w.u8(2).u8(station_code(*s));
            }
            ClientMsg::Ready(r) => {
                w.u8(3).bool(*r);
            }
            ClientMsg::Cmd(c) => {
                w.u8(4);
                put_command(&mut w, *c);
            }
            ClientMsg::Bye => {
                w.u8(5);
            }
        }
        w.0
    }

    pub fn decode(body: &[u8]) -> Res<ClientMsg> {
        let mut r = Reader::new(body);
        let m = match r.u8()? {
            0 => {
                let protocol = r.u16()?;
                let mut name = r.str()?;
                while name.len() > MAX_NAME {
                    name.pop();
                }
                ClientMsg::Hello { name, protocol }
            }
            1 => ClientMsg::Take(station_from(r.u8()?)?),
            2 => ClientMsg::Release(station_from(r.u8()?)?),
            3 => ClientMsg::Ready(r.bool()?),
            4 => ClientMsg::Cmd(get_command(&mut r)?),
            5 => ClientMsg::Bye,
            _ => return Err(WireError("unknown client message")),
        };
        r.done()?;
        Ok(m)
    }
}

fn put_lobby(w: &mut Writer, l: &LobbyState) {
    w.u8(l.players.len().min(255) as u8);
    for p in l.players.iter().take(255) {
        w.u8(p.player).str(&p.name).bool(p.ready);
    }
    for s in l.stations {
        match s {
            Some(p) => w.bool(true).u8(p),
            None => w.bool(false),
        };
    }
}

fn get_lobby(r: &mut Reader) -> Res<LobbyState> {
    let n = r.u8()?;
    let mut players = Vec::new();
    for _ in 0..n {
        players.push(Seat {
            player: r.u8()?,
            name: r.str()?,
            ready: r.bool()?,
        });
    }
    let mut stations = [None; 7];
    for s in &mut stations {
        *s = if r.bool()? { Some(r.u8()?) } else { None };
    }
    Ok(LobbyState { players, stations })
}

fn put_snapshot(w: &mut Writer, s: &Snapshot) {
    w.u64(s.tick).f32(s.time).opt_id(s.player);
    match &s.ship {
        Some(p) => {
            w.bool(true).f32(p.hull).f32(p.max_hull);
            for v in p.shields {
                w.f32(v);
            }
            w.u8(p.torpedoes).u8(p.missiles).f32(p.throttle);
            w.opt_id(p.target).bool(p.docked).u32(p.credits as u32);
        }
        None => {
            w.bool(false);
        }
    }
    w.str(&s.objective).u8(s.stage.0).u8(s.stage.1);
    w.u16(s.ents.len().min(usize::from(u16::MAX)) as u16);
    for e in s.ents.iter().take(usize::from(u16::MAX)) {
        w.id(e.id).u8(e.kind).u8(e.faction).vec3(e.pos).vec3(e.vel);
        for q in e.rot {
            w.f32(q);
        }
        w.f32(e.radius).u8(e.hull_pct).str(&e.label);
    }
}

fn get_snapshot(r: &mut Reader) -> Res<Snapshot> {
    let tick = r.u64()?;
    let time = r.f32()?;
    let player = r.opt_id()?;
    let ship = if r.bool()? {
        let hull = r.f32()?;
        let max_hull = r.f32()?;
        let mut shields = [0.0; 4];
        for s in &mut shields {
            *s = r.f32()?;
        }
        Some(ShipSnap {
            hull,
            max_hull,
            shields,
            torpedoes: r.u8()?,
            missiles: r.u8()?,
            throttle: r.f32()?,
            target: r.opt_id()?,
            docked: r.bool()?,
            credits: r.u32()? as i32,
        })
    } else {
        None
    };
    let objective = r.str()?;
    let stage = (r.u8()?, r.u8()?);
    let n = r.u16()?;
    let mut ents = Vec::new();
    for _ in 0..n {
        let id = r.id()?;
        let kind = r.u8()?;
        let faction = r.u8()?;
        let pos = r.vec3()?;
        let vel = r.vec3()?;
        let rot = [r.f32()?, r.f32()?, r.f32()?, r.f32()?];
        ents.push(EntSnap {
            id,
            kind,
            faction,
            pos,
            vel,
            rot,
            radius: r.f32()?,
            hull_pct: r.u8()?,
            label: r.str()?,
        });
    }
    Ok(Snapshot {
        tick,
        time,
        player,
        ship,
        objective,
        stage,
        ents,
    })
}

impl ServerMsg {
    pub fn encode(&self) -> Vec<u8> {
        let mut w = Writer::default();
        match self {
            ServerMsg::Welcome { player } => {
                w.u8(0).u8(*player);
            }
            ServerMsg::Reject(why) => {
                w.u8(1).str(why);
            }
            ServerMsg::Lobby(l) => {
                w.u8(2);
                put_lobby(&mut w, l);
            }
            ServerMsg::Start {
                seed,
                mission,
                difficulty,
            } => {
                w.u8(3).u64(*seed).u8(*mission).u8(*difficulty);
            }
            ServerMsg::Snapshot(s) => {
                w.u8(4);
                put_snapshot(&mut w, s);
            }
            ServerMsg::Notice(t) => {
                w.u8(5).str(t);
            }
            ServerMsg::End { won, reason } => {
                w.u8(6).bool(*won).str(reason);
            }
        }
        w.0
    }

    pub fn decode(body: &[u8]) -> Res<ServerMsg> {
        let mut r = Reader::new(body);
        let m = match r.u8()? {
            0 => ServerMsg::Welcome { player: r.u8()? },
            1 => ServerMsg::Reject(r.str()?),
            2 => ServerMsg::Lobby(get_lobby(&mut r)?),
            3 => ServerMsg::Start {
                seed: r.u64()?,
                mission: r.u8()?,
                difficulty: r.u8()?,
            },
            4 => ServerMsg::Snapshot(get_snapshot(&mut r)?),
            5 => ServerMsg::Notice(r.str()?),
            6 => ServerMsg::End {
                won: r.bool()?,
                reason: r.str()?,
            },
            _ => return Err(WireError("unknown server message")),
        };
        r.done()?;
        Ok(m)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_messages_round_trip() {
        for m in [
            ClientMsg::Hello {
                name: "Bjorn".into(),
                protocol: PROTOCOL,
            },
            ClientMsg::Take(Station::Weapons),
            ClientMsg::Release(Station::Helm),
            ClientMsg::Ready(true),
            ClientMsg::Cmd(Command::FireLance),
            ClientMsg::Bye,
        ] {
            assert_eq!(ClientMsg::decode(&m.encode()), Ok(m));
        }
        assert!(ClientMsg::decode(&[1, 99]).is_err());
        assert!(ClientMsg::decode(&[3, 1, 1]).is_err());
        let long = ClientMsg::Hello {
            name: "x".repeat(100),
            protocol: 1,
        };
        assert!(matches!(
            ClientMsg::decode(&long.encode()),
            Ok(ClientMsg::Hello { name, .. }) if name.len() == MAX_NAME
        ));
    }

    #[test]
    fn server_messages_round_trip() {
        let snap = Snapshot {
            tick: 99,
            time: 1.5,
            player: Some(EntityId {
                index: 2,
                generation: 1,
            }),
            ship: Some(ShipSnap {
                hull: 200.0,
                max_hull: 260.0,
                shields: [1.0, 2.0, 3.0, 4.0],
                torpedoes: 4,
                missiles: 6,
                throttle: 0.5,
                target: None,
                docked: false,
                credits: -5,
            }),
            objective: "FLY".into(),
            stage: (2, 5),
            ents: vec![EntSnap {
                id: EntityId {
                    index: 7,
                    generation: 0,
                },
                kind: kind_code(Kind::Ship),
                faction: 2,
                pos: Vec3::new(1.0, 2.0, 3.0),
                vel: Vec3::ZERO,
                rot: [0.0, 0.0, 0.0, 1.0],
                radius: 20.0,
                hull_pct: 80,
                label: "Cutter".into(),
            }],
        };
        for m in [
            ServerMsg::Welcome { player: 3 },
            ServerMsg::Reject("full".into()),
            ServerMsg::Lobby(LobbyState {
                players: vec![Seat {
                    player: 1,
                    name: "Astrid".into(),
                    ready: true,
                }],
                stations: [Some(1), None, None, None, None, None, Some(1)],
            }),
            ServerMsg::Start {
                seed: 5,
                mission: 2,
                difficulty: 1,
            },
            ServerMsg::Snapshot(snap),
            ServerMsg::Notice("hi".into()),
            ServerMsg::End {
                won: true,
                reason: String::new(),
            },
        ] {
            assert_eq!(ServerMsg::decode(&m.encode()), Ok(m));
        }
    }
}
