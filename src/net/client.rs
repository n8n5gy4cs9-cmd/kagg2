//! Headless network client (T32): connects, tracks the lobby and the latest snapshot.

use std::io::{self, Read, Write};
use std::net::{SocketAddr, TcpStream};

use super::proto::{ClientMsg, LobbyState, PROTOCOL, ServerMsg, Snapshot};
use super::wire::{FrameBuf, write_frame};

pub struct Client {
    stream: TcpStream,
    frames: FrameBuf,
    out: Vec<u8>,
    closed: bool,
    /// Our player id once the server has welcomed us.
    pub player: Option<u8>,
    pub lobby: LobbyState,
    pub snapshot: Option<Snapshot>,
    /// `(seed, mission, difficulty)` once the game has started.
    pub started: Option<(u64, u8, u8)>,
    /// `(won, reason)` once the mission is over.
    pub ended: Option<(bool, String)>,
    pub rejected: Option<String>,
    /// Notices from the server, oldest first.
    pub log: Vec<String>,
}

impl Client {
    pub fn connect(addr: SocketAddr, name: &str) -> io::Result<Client> {
        Client::connect_with_protocol(addr, name, PROTOCOL)
    }

    /// Connect claiming a specific protocol version (for tests of the handshake).
    pub fn connect_with_protocol(
        addr: SocketAddr,
        name: &str,
        protocol: u16,
    ) -> io::Result<Client> {
        let stream = TcpStream::connect(addr)?;
        stream.set_nodelay(true)?;
        stream.set_nonblocking(true)?;
        let mut c = Client {
            stream,
            frames: FrameBuf::default(),
            out: Vec::new(),
            closed: false,
            player: None,
            lobby: LobbyState::default(),
            snapshot: None,
            started: None,
            ended: None,
            rejected: None,
            log: Vec::new(),
        };
        c.send(&ClientMsg::Hello {
            name: name.to_owned(),
            protocol,
        })?;
        Ok(c)
    }

    /// Queue a message and try to flush.
    pub fn send(&mut self, m: &ClientMsg) -> io::Result<()> {
        write_frame(&mut self.out, &m.encode())?;
        self.flush()
    }

    fn flush(&mut self) -> io::Result<()> {
        while !self.out.is_empty() {
            match self.stream.write(&self.out) {
                Ok(0) => return Err(io::ErrorKind::WriteZero.into()),
                Ok(n) => {
                    self.out.drain(..n);
                }
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }

    /// Read what has arrived and update the client state. Errors when the server is gone.
    pub fn poll(&mut self) -> io::Result<Vec<ServerMsg>> {
        let mut tmp = [0u8; 16 * 1024];
        while !self.closed {
            if self.flush().is_err() {
                self.closed = true;
                break;
            }
            match self.stream.read(&mut tmp) {
                Ok(0) => self.closed = true,
                Ok(n) => self.frames.push(&tmp[..n]),
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
                Err(_) => self.closed = true,
            }
        }
        let mut got = Vec::new();
        while let Some(body) = self
            .frames
            .next_frame()
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?
        {
            let m = ServerMsg::decode(&body)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
            match &m {
                ServerMsg::Welcome { player } => self.player = Some(*player),
                ServerMsg::Reject(why) => self.rejected = Some(why.clone()),
                ServerMsg::Lobby(l) => self.lobby = l.clone(),
                ServerMsg::Start {
                    seed,
                    mission,
                    difficulty,
                } => self.started = Some((*seed, *mission, *difficulty)),
                ServerMsg::Snapshot(s) => self.snapshot = Some(s.clone()),
                ServerMsg::Notice(t) => {
                    self.log.push(t.clone());
                    if self.log.len() > 50 {
                        self.log.remove(0);
                    }
                }
                ServerMsg::End { won, reason } => self.ended = Some((*won, reason.clone())),
            }
            got.push(m);
        }
        // a server that said goodbye still gets its last messages delivered first
        if self.closed && got.is_empty() {
            return Err(io::ErrorKind::UnexpectedEof.into());
        }
        Ok(got)
    }
}
