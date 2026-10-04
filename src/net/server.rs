//! The authoritative server (T32): lobby, seat checks, the 60 Hz sim and 20 Hz snapshots.

use std::io::{self, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream, ToSocketAddrs};
use std::time::{Duration, Instant};

use super::proto::{
    ClientMsg, EntSnap, LobbyState, MAX_NAME, PROTOCOL, Seat, ServerMsg, ShipSnap, Snapshot,
    kind_code,
};
use super::station_of;
use super::wire::{FrameBuf, write_frame};
use crate::balance::DIFFICULTY_SCALE;
use crate::sim::crew::Station;
use crate::sim::mission::{Carry, MissionState};
use crate::sim::{Addressed, SectorId, World, new_game};

/// Most commands queued for one sim step (flood protection).
const MAX_PENDING: usize = 256;
/// Messages read from one client per poll.
const MAX_MSGS_PER_POLL: usize = 64;
/// A client whose unsent data grows past this is too slow and is dropped.
const MAX_OUTBOX: usize = 4 * 1024 * 1024;
/// Entities per snapshot (nearest first).
const MAX_SNAP_ENTS: usize = 160;
/// Sim steps between snapshots (60 Hz -> 20 Hz).
const SNAP_EVERY: u64 = 3;

#[derive(Debug, Clone, Copy)]
pub struct Config {
    /// Mission id (`1..=10`, or a loaded script).
    pub mission: u8,
    pub seed: u64,
    /// Difficulty `0..=2` (Thrall, Karl, Jarl).
    pub difficulty: u8,
}

struct Conn {
    id: u8,
    name: String,
    ready: bool,
    welcomed: bool,
    stream: TcpStream,
    frames: FrameBuf,
    out: Vec<u8>,
    dead: bool,
}

struct Game {
    world: World,
    mission: MissionState,
    over: bool,
}

pub struct Server {
    listener: TcpListener,
    conns: Vec<Conn>,
    next_id: u8,
    stations: [Option<u8>; 7],
    cfg: Config,
    game: Option<Game>,
    pending: Vec<Addressed>,
}

impl Server {
    pub fn bind(addr: impl ToSocketAddrs, cfg: Config) -> io::Result<Server> {
        let listener = TcpListener::bind(addr)?;
        listener.set_nonblocking(true)?;
        Ok(Server {
            listener,
            conns: Vec::new(),
            next_id: 1,
            stations: [None; 7],
            cfg,
            game: None,
            pending: Vec::new(),
        })
    }

    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.listener.local_addr()
    }

    pub fn connections(&self) -> usize {
        self.conns.len()
    }

    pub fn playing(&self) -> bool {
        self.game.as_ref().is_some_and(|g| !g.over)
    }

    pub fn finished(&self) -> bool {
        self.game.as_ref().is_some_and(|g| g.over)
    }

    fn lobby(&self) -> LobbyState {
        LobbyState {
            players: self
                .conns
                .iter()
                .filter(|c| c.welcomed)
                .map(|c| Seat {
                    player: c.id,
                    name: c.name.clone(),
                    ready: c.ready,
                })
                .collect(),
            stations: self.stations,
        }
    }

    fn send(c: &mut Conn, m: &ServerMsg) {
        if write_frame(&mut c.out, &m.encode()).is_err() || c.out.len() > MAX_OUTBOX {
            c.dead = true;
        }
    }

    fn broadcast(&mut self, m: &ServerMsg) {
        let body = m.encode();
        for c in self.conns.iter_mut().filter(|c| c.welcomed) {
            if write_frame(&mut c.out, &body).is_err() || c.out.len() > MAX_OUTBOX {
                c.dead = true;
            }
        }
    }

    fn notice(&mut self, text: impl Into<String>) {
        self.broadcast(&ServerMsg::Notice(text.into()));
    }

    /// Crew AI runs every automatable station nobody sits at.
    fn sync_crew(&mut self) {
        let held = self.stations;
        if let Some(g) = self.game.as_mut() {
            for s in Station::ALL {
                if s.automatable() {
                    g.world.crew.auto[s as usize] = held[s as usize].is_none();
                }
            }
        }
    }

    fn start(&mut self) {
        let mut world = new_game(self.cfg.seed, SectorId::for_mission(self.cfg.mission));
        world.difficulty = DIFFICULTY_SCALE[usize::from(self.cfg.difficulty.min(2))];
        Carry::default().apply(&mut world);
        let mut mission = MissionState::new(self.cfg.mission);
        mission.begin(&mut world);
        world.crew.skill = 0.8;
        self.game = Some(Game {
            world,
            mission,
            over: false,
        });
        self.sync_crew();
        let m = ServerMsg::Start {
            seed: self.cfg.seed,
            mission: self.cfg.mission,
            difficulty: self.cfg.difficulty,
        };
        self.broadcast(&m);
        self.notice("MISSION START");
    }

    fn handle(&mut self, idx: usize, msg: ClientMsg) {
        let id = self.conns[idx].id;
        let name = self.conns[idx].name.clone();
        let welcomed = self.conns[idx].welcomed;
        if !welcomed {
            let ClientMsg::Hello { name, protocol } = msg else {
                self.conns[idx].dead = true;
                return;
            };
            if protocol != PROTOCOL {
                let why = format!("PROTOCOL MISMATCH (SERVER {PROTOCOL}, YOU {protocol})");
                Self::send(&mut self.conns[idx], &ServerMsg::Reject(why));
                self.conns[idx].dead = true;
                return;
            }
            let clean: String = name.chars().filter(|c| !c.is_control()).collect();
            let c = &mut self.conns[idx];
            c.name = if clean.trim().is_empty() {
                format!("Crew {}", c.id)
            } else {
                clean.chars().take(MAX_NAME).collect()
            };
            c.welcomed = true;
            let (pid, joined) = (c.id, c.name.clone());
            Self::send(c, &ServerMsg::Welcome { player: pid });
            if let Some(g) = &self.game {
                let m = ServerMsg::Start {
                    seed: self.cfg.seed,
                    mission: self.cfg.mission,
                    difficulty: self.cfg.difficulty,
                };
                let over = g.over;
                Self::send(&mut self.conns[idx], &m);
                if over {
                    Self::send(
                        &mut self.conns[idx],
                        &ServerMsg::Notice("THE MISSION IS ALREADY OVER".into()),
                    );
                }
            }
            let l = ServerMsg::Lobby(self.lobby());
            self.broadcast(&l);
            self.notice(format!("{joined} JOINED"));
            return;
        }
        match msg {
            ClientMsg::Hello { .. } => {}
            ClientMsg::Take(s) => match self.stations[s as usize] {
                None => {
                    self.stations[s as usize] = Some(id);
                    self.sync_crew();
                    let l = ServerMsg::Lobby(self.lobby());
                    self.broadcast(&l);
                    self.notice(format!("{name} TOOK {}", s.name()));
                }
                Some(p) if p == id => {}
                Some(p) => {
                    let who = self
                        .conns
                        .iter()
                        .find(|c| c.id == p)
                        .map_or("ANOTHER PLAYER".to_owned(), |c| c.name.clone());
                    let m = ServerMsg::Notice(format!("{} IS TAKEN BY {who}", s.name()));
                    Self::send(&mut self.conns[idx], &m);
                }
            },
            ClientMsg::Release(s) => {
                if self.stations[s as usize] == Some(id) {
                    self.stations[s as usize] = None;
                    self.sync_crew();
                    let l = ServerMsg::Lobby(self.lobby());
                    self.broadcast(&l);
                }
            }
            ClientMsg::Ready(r) => {
                self.conns[idx].ready = r;
                let l = ServerMsg::Lobby(self.lobby());
                self.broadcast(&l);
                let everyone = self.conns.iter().filter(|c| c.welcomed).all(|c| c.ready);
                if self.game.is_none() && everyone {
                    self.start();
                }
            }
            ClientMsg::Cmd(cmd) => {
                let st = station_of(&cmd);
                let ship = self.game.as_ref().and_then(|g| g.world.player);
                match (ship, self.stations[st as usize] == Some(id)) {
                    (Some(ship), true) if self.pending.len() < MAX_PENDING => {
                        self.pending.push(Addressed { ship, cmd });
                    }
                    (Some(_), false) => {
                        let m = ServerMsg::Notice(format!("NOT YOUR STATION: {}", st.name()));
                        Self::send(&mut self.conns[idx], &m);
                    }
                    _ => {}
                }
            }
            ClientMsg::Bye => self.conns[idx].dead = true,
        }
    }

    /// Accept connections, read and handle messages, flush output, drop dead clients.
    pub fn poll(&mut self) {
        while let Ok((stream, _)) = self.listener.accept() {
            if stream.set_nonblocking(true).is_err() || stream.set_nodelay(true).is_err() {
                continue;
            }
            if self.conns.len() >= Station::ALL.len() {
                continue; // full: drop the socket
            }
            let id = self.next_id;
            self.next_id = self.next_id.wrapping_add(1).max(1);
            self.conns.push(Conn {
                id,
                name: String::new(),
                ready: false,
                welcomed: false,
                stream,
                frames: FrameBuf::default(),
                out: Vec::new(),
                dead: false,
            });
        }
        for i in 0..self.conns.len() {
            let mut tmp = [0u8; 8192];
            loop {
                match self.conns[i].stream.read(&mut tmp) {
                    Ok(0) => {
                        self.conns[i].dead = true;
                        break;
                    }
                    Ok(n) => self.conns[i].frames.push(&tmp[..n]),
                    Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
                    Err(_) => {
                        self.conns[i].dead = true;
                        break;
                    }
                }
            }
            for _ in 0..MAX_MSGS_PER_POLL {
                if self.conns[i].dead {
                    break;
                }
                match self.conns[i].frames.next_frame() {
                    Ok(Some(body)) => match ClientMsg::decode(&body) {
                        Ok(m) => self.handle(i, m),
                        Err(_) => self.conns[i].dead = true,
                    },
                    Ok(None) => break,
                    Err(_) => self.conns[i].dead = true,
                }
            }
        }
        self.flush_and_reap();
    }

    fn flush_and_reap(&mut self) {
        for c in &mut self.conns {
            while !c.out.is_empty() {
                match c.stream.write(&c.out) {
                    Ok(0) => {
                        c.dead = true;
                        break;
                    }
                    Ok(n) => {
                        c.out.drain(..n);
                    }
                    Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
                    Err(_) => {
                        c.dead = true;
                        break;
                    }
                }
            }
        }
        // a dead client still gets what was queued for it (a rejection) before it is dropped
        let mut left: Vec<String> = Vec::new();
        let mut freed = false;
        self.conns.retain(|c| {
            let drop = c.dead && (c.out.is_empty() || !c.welcomed);
            if drop && c.welcomed {
                left.push(c.name.clone());
            }
            if drop {
                for s in self.stations.iter_mut() {
                    if *s == Some(c.id) {
                        *s = None;
                        freed = true;
                    }
                }
            }
            !drop
        });
        if freed {
            self.sync_crew();
        }
        if !left.is_empty() || freed {
            let l = ServerMsg::Lobby(self.lobby());
            self.broadcast(&l);
            for n in left {
                self.notice(format!("{n} LEFT"));
            }
        }
    }

    fn snapshot(g: &Game) -> Snapshot {
        let w = &g.world;
        let me = w.player.and_then(|p| w.get(p));
        let origin = me.map_or(glam::Vec3::ZERO, |e| e.pos);
        let mut ents: Vec<(f32, EntSnap)> = w
            .iter()
            .map(|(id, e)| {
                let hull_pct = e.ops.as_deref().map_or(100, |o| {
                    (100.0 * o.hull / o.max_hull.max(1.0)).clamp(0.0, 100.0) as u8
                });
                (
                    e.pos.distance_squared(origin),
                    EntSnap {
                        id,
                        kind: kind_code(e.kind),
                        faction: e.faction as u8,
                        pos: e.pos,
                        vel: e.vel,
                        rot: e.rot.to_array(),
                        radius: e.radius,
                        hull_pct,
                        label: e.label.to_owned(),
                    },
                )
            })
            .collect();
        ents.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.id.cmp(&b.1.id)));
        ents.truncate(MAX_SNAP_ENTS);
        let ship = me.and_then(|e| {
            let o = e.ops.as_deref()?;
            Some(ShipSnap {
                hull: o.hull,
                max_hull: o.max_hull,
                shields: o.shields,
                torpedoes: o.torpedoes,
                missiles: o.missiles,
                throttle: e.ship.map_or(0.0, |s| s.throttle),
                target: o.target,
                docked: o.docked.is_some(),
                credits: o.cargo.credits,
            })
        });
        let (cur, total) = g.mission.progress();
        Snapshot {
            tick: w.tick,
            time: w.time,
            player: w.player,
            ship,
            objective: g.mission.objective().unwrap_or("").to_owned(),
            stage: (cur.min(255) as u8, total.min(255) as u8),
            ents: ents.into_iter().map(|e| e.1).collect(),
        }
    }

    /// Advance the sim one 60 Hz step (when a mission is running) and publish snapshots.
    pub fn tick(&mut self) {
        let cmds = std::mem::take(&mut self.pending);
        let Some(g) = self.game.as_mut().filter(|g| !g.over) else {
            return;
        };
        let events = g.world.step(&cmds);
        g.mission.update(&mut g.world, &events);
        let dead = g
            .world
            .player
            .and_then(|p| g.world.get(p))
            .and_then(|e| e.ops.as_deref())
            .is_none_or(|o| o.dead);
        if dead {
            g.mission.fail("YOUR SHIP WAS DESTROYED");
        }
        let notes = g.mission.take_notes();
        let result = g.mission.result();
        let reason = g.mission.reason();
        let snap = g
            .world
            .tick
            .is_multiple_of(SNAP_EVERY)
            .then(|| Self::snapshot(g));
        let texts: Vec<String> = events
            .iter()
            .filter_map(|e| match e {
                crate::sim::Event::Message(t) => Some(t.clone()),
                _ => None,
            })
            .collect();
        if result.is_some() {
            g.over = true;
        }
        for n in notes {
            if n.who.is_empty() {
                self.notice(n.text);
            } else {
                self.notice(format!("{}: {}", n.who, n.text));
            }
        }
        for t in texts {
            self.notice(t);
        }
        if let Some(s) = snap {
            self.broadcast(&ServerMsg::Snapshot(s));
        }
        if let Some(won) = result {
            self.broadcast(&ServerMsg::End {
                won,
                reason: reason.to_owned(),
            });
        }
        self.flush_and_reap();
    }

    /// Run until the mission ends (the server binary's main loop): polls the network every
    /// millisecond and steps the sim on a fixed 60 Hz wall-clock schedule.
    pub fn run(&mut self) {
        let step = Duration::from_secs_f64(1.0 / 60.0);
        let mut next = Instant::now();
        while !self.finished() {
            self.poll();
            let now = Instant::now();
            let mut guard = 0;
            while now >= next && guard < 5 {
                self.tick();
                next += step;
                guard += 1;
            }
            if guard == 5 {
                next = now; // fell behind: drop the backlog instead of spiralling
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        // let the final messages out
        for _ in 0..50 {
            self.poll();
            std::thread::sleep(Duration::from_millis(2));
        }
    }
}
