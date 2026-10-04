//! Binary wire format (T32): length-prefixed frames, little-endian fields, no dependencies.
//! Everything read from the network is validated; nothing here panics on bad input.

use std::io::{self, Read, Write};

use glam::Vec3;

use crate::sim::ai::Order;
use crate::sim::crew::Station;
use crate::sim::ops::Dest;
use crate::sim::systems::Sys;
use crate::sim::{Command, EntityId, SectorId};

/// Largest accepted frame body (bytes); bigger frames drop the connection.
pub const MAX_FRAME: usize = 256 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WireError(pub &'static str);

impl std::fmt::Display for WireError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "wire: {}", self.0)
    }
}

impl std::error::Error for WireError {}

type Res<T> = Result<T, WireError>;

/// Append-only encoder.
#[derive(Default)]
pub struct Writer(pub Vec<u8>);

impl Writer {
    pub fn u8(&mut self, v: u8) -> &mut Self {
        self.0.push(v);
        self
    }
    pub fn bool(&mut self, v: bool) -> &mut Self {
        self.u8(u8::from(v))
    }
    pub fn u16(&mut self, v: u16) -> &mut Self {
        self.0.extend_from_slice(&v.to_le_bytes());
        self
    }
    pub fn u32(&mut self, v: u32) -> &mut Self {
        self.0.extend_from_slice(&v.to_le_bytes());
        self
    }
    pub fn u64(&mut self, v: u64) -> &mut Self {
        self.0.extend_from_slice(&v.to_le_bytes());
        self
    }
    pub fn f32(&mut self, v: f32) -> &mut Self {
        self.0.extend_from_slice(&v.to_le_bytes());
        self
    }
    pub fn vec3(&mut self, v: Vec3) -> &mut Self {
        self.f32(v.x).f32(v.y).f32(v.z)
    }
    /// A string of at most 65 535 bytes (longer ones are cut at a char boundary).
    pub fn str(&mut self, s: &str) -> &mut Self {
        let mut end = s.len().min(usize::from(u16::MAX));
        while !s.is_char_boundary(end) {
            end -= 1;
        }
        self.u16(end as u16);
        self.0.extend_from_slice(&s.as_bytes()[..end]);
        self
    }
    pub fn id(&mut self, id: EntityId) -> &mut Self {
        self.u32(id.index).u32(id.generation)
    }
    pub fn opt_id(&mut self, id: Option<EntityId>) -> &mut Self {
        match id {
            Some(i) => self.bool(true).id(i),
            None => self.bool(false),
        }
    }
}

/// Bounds-checked decoder over a byte slice.
pub struct Reader<'a> {
    buf: &'a [u8],
}

impl<'a> Reader<'a> {
    pub fn new(buf: &'a [u8]) -> Reader<'a> {
        Reader { buf }
    }

    fn take(&mut self, n: usize) -> Res<&'a [u8]> {
        if self.buf.len() < n {
            return Err(WireError("truncated message"));
        }
        let (a, b) = self.buf.split_at(n);
        self.buf = b;
        Ok(a)
    }

    pub fn u8(&mut self) -> Res<u8> {
        Ok(self.take(1)?[0])
    }
    pub fn bool(&mut self) -> Res<bool> {
        match self.u8()? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(WireError("bad bool")),
        }
    }
    pub fn u16(&mut self) -> Res<u16> {
        let b = self.take(2)?;
        Ok(u16::from_le_bytes([b[0], b[1]]))
    }
    pub fn u32(&mut self) -> Res<u32> {
        let b = self.take(4)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }
    pub fn u64(&mut self) -> Res<u64> {
        let b = self.take(8)?;
        let mut a = [0; 8];
        a.copy_from_slice(b);
        Ok(u64::from_le_bytes(a))
    }
    /// A finite float (NaN and infinities are rejected so they never reach the sim).
    pub fn f32(&mut self) -> Res<f32> {
        let b = self.take(4)?;
        let v = f32::from_le_bytes([b[0], b[1], b[2], b[3]]);
        if v.is_finite() {
            Ok(v)
        } else {
            Err(WireError("non-finite float"))
        }
    }
    pub fn vec3(&mut self) -> Res<Vec3> {
        Ok(Vec3::new(self.f32()?, self.f32()?, self.f32()?))
    }
    pub fn str(&mut self) -> Res<String> {
        let n = usize::from(self.u16()?);
        String::from_utf8(self.take(n)?.to_vec()).map_err(|_| WireError("bad utf-8"))
    }
    pub fn id(&mut self) -> Res<EntityId> {
        Ok(EntityId {
            index: self.u32()?,
            generation: self.u32()?,
        })
    }
    pub fn opt_id(&mut self) -> Res<Option<EntityId>> {
        Ok(if self.bool()? { Some(self.id()?) } else { None })
    }
    pub fn done(&self) -> Res<()> {
        if self.buf.is_empty() {
            Ok(())
        } else {
            Err(WireError("trailing bytes"))
        }
    }
}

pub fn station_code(s: Station) -> u8 {
    s as u8
}

pub fn station_from(c: u8) -> Res<Station> {
    Station::ALL
        .get(usize::from(c))
        .copied()
        .ok_or(WireError("bad station"))
}

fn sys_from(c: u8) -> Res<Sys> {
    Sys::ALL
        .get(usize::from(c))
        .copied()
        .ok_or(WireError("bad system"))
}

fn sector_from(c: u8) -> Res<SectorId> {
    SectorId::ALL
        .get(usize::from(c))
        .copied()
        .ok_or(WireError("bad sector"))
}

fn sector_code(s: SectorId) -> u8 {
    SectorId::ALL.iter().position(|&x| x == s).unwrap_or(0) as u8
}

fn order_code(o: Order) -> u8 {
    match o {
        Order::Attack => 0,
        Order::Guard => 1,
        Order::Follow => 2,
        Order::Retreat => 3,
    }
}

fn order_from(c: u8) -> Res<Order> {
    Ok(match c {
        0 => Order::Attack,
        1 => Order::Guard,
        2 => Order::Follow,
        3 => Order::Retreat,
        _ => return Err(WireError("bad order")),
    })
}

fn dest_put(w: &mut Writer, d: Dest) {
    match d {
        Dest::None => {
            w.u8(0);
        }
        Dest::Point(p) => {
            w.u8(1).vec3(p);
        }
        Dest::Body(id) => {
            w.u8(2).id(id);
        }
    }
}

fn dest_get(r: &mut Reader) -> Res<Dest> {
    Ok(match r.u8()? {
        0 => Dest::None,
        1 => Dest::Point(r.vec3()?),
        2 => Dest::Body(r.id()?),
        _ => return Err(WireError("bad destination")),
    })
}

/// Encode a sim command.
pub fn put_command(w: &mut Writer, c: Command) {
    use Command as C;
    match c {
        C::SetThrottle(v) => {
            w.u8(0).f32(v);
        }
        C::AdjustThrottle(v) => {
            w.u8(1).f32(v);
        }
        C::Turn { pitch, yaw, roll } => {
            w.u8(2).f32(pitch).f32(yaw).f32(roll);
        }
        C::Strafe { x, y } => {
            w.u8(3).f32(x).f32(y);
        }
        C::Brake(b) => {
            w.u8(4).bool(b);
        }
        C::SetAssist(b) => {
            w.u8(5).bool(b);
        }
        C::FireLance => {
            w.u8(6);
        }
        C::FireTorpedo => {
            w.u8(7);
        }
        C::FireMissile => {
            w.u8(8);
        }
        C::SetLanceFreq(f) => {
            w.u8(9).u8(f);
        }
        C::SetShieldFreq(f) => {
            w.u8(10).u8(f);
        }
        C::SetTarget(t) => {
            w.u8(11).opt_id(t);
        }
        C::SetPower(s, v) => {
            w.u8(12).u8(s.idx() as u8).f32(v);
        }
        C::SetCoolant(s, v) => {
            w.u8(13).u8(s.idx() as u8).f32(v);
        }
        C::PowerPreset(p) => {
            w.u8(14).u8(p);
        }
        C::AssignRepair { team, sys } => {
            w.u8(15).u8(team);
            match sys {
                Some(s) => w.bool(true).u8(s.idx() as u8),
                None => w.bool(false),
            };
        }
        C::Scan(t) => {
            w.u8(16).opt_id(t);
        }
        C::Sweep => {
            w.u8(17);
        }
        C::SetDest(d) => {
            w.u8(18);
            dest_put(w, d);
        }
        C::AddWaypoint(p) => {
            w.u8(19).vec3(p);
        }
        C::Autopilot(b) => {
            w.u8(20).bool(b);
        }
        C::Warp(s) => {
            w.u8(21);
            match s {
                Some(s) => w.bool(true).u8(sector_code(s)),
                None => w.bool(false),
            };
        }
        C::Order(o) => {
            w.u8(22).u8(order_code(o));
        }
        C::DemandSurrender(id) => {
            w.u8(23).id(id);
        }
        C::Dock(t) => {
            w.u8(24).opt_id(t);
        }
        C::Mine(t) => {
            w.u8(25).opt_id(t);
        }
    }
}

/// Decode a sim command (values are range-checked where the sim assumes a range).
pub fn get_command(r: &mut Reader) -> Res<Command> {
    use Command as C;
    Ok(match r.u8()? {
        0 => C::SetThrottle(r.f32()?.clamp(-0.3, 1.0)),
        1 => C::AdjustThrottle(r.f32()?.clamp(-1.0, 1.0)),
        2 => C::Turn {
            pitch: r.f32()?.clamp(-1.0, 1.0),
            yaw: r.f32()?.clamp(-1.0, 1.0),
            roll: r.f32()?.clamp(-1.0, 1.0),
        },
        3 => C::Strafe {
            x: r.f32()?.clamp(-1.0, 1.0),
            y: r.f32()?.clamp(-1.0, 1.0),
        },
        4 => C::Brake(r.bool()?),
        5 => C::SetAssist(r.bool()?),
        6 => C::FireLance,
        7 => C::FireTorpedo,
        8 => C::FireMissile,
        9 => C::SetLanceFreq(r.u8()?.min(9)),
        10 => C::SetShieldFreq(r.u8()?.min(9)),
        11 => C::SetTarget(r.opt_id()?),
        12 => C::SetPower(sys_from(r.u8()?)?, r.f32()?.clamp(0.0, 3.0)),
        13 => C::SetCoolant(sys_from(r.u8()?)?, r.f32()?.clamp(0.0, 1.0)),
        14 => C::PowerPreset(r.u8()?),
        15 => {
            let team = r.u8()?;
            if team > 2 {
                return Err(WireError("bad repair team"));
            }
            let sys = if r.bool()? {
                Some(sys_from(r.u8()?)?)
            } else {
                None
            };
            C::AssignRepair { team, sys }
        }
        16 => C::Scan(r.opt_id()?),
        17 => C::Sweep,
        18 => C::SetDest(dest_get(r)?),
        19 => C::AddWaypoint(r.vec3()?),
        20 => C::Autopilot(r.bool()?),
        21 => C::Warp(if r.bool()? {
            Some(sector_from(r.u8()?)?)
        } else {
            None
        }),
        22 => C::Order(order_from(r.u8()?)?),
        23 => C::DemandSurrender(r.id()?),
        24 => C::Dock(r.opt_id()?),
        25 => C::Mine(r.opt_id()?),
        _ => return Err(WireError("unknown command")),
    })
}

/// Write one frame: `u32` body length, then the body.
pub fn write_frame(out: &mut impl Write, body: &[u8]) -> io::Result<()> {
    let len = u32::try_from(body.len())
        .ok()
        .filter(|&l| l as usize <= MAX_FRAME)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "frame too large"))?;
    let mut buf = Vec::with_capacity(body.len() + 4);
    buf.extend_from_slice(&len.to_le_bytes());
    buf.extend_from_slice(body);
    out.write_all(&buf)
}

/// Blocking read of one frame (for tests and simple tools).
pub fn read_frame(inp: &mut impl Read) -> io::Result<Vec<u8>> {
    let mut len = [0u8; 4];
    inp.read_exact(&mut len)?;
    let n = u32::from_le_bytes(len) as usize;
    if n > MAX_FRAME {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "frame too large",
        ));
    }
    let mut body = vec![0; n];
    inp.read_exact(&mut body)?;
    Ok(body)
}

/// Incremental frame splitter for non-blocking sockets.
#[derive(Default)]
pub struct FrameBuf {
    buf: Vec<u8>,
}

impl FrameBuf {
    pub fn push(&mut self, data: &[u8]) {
        self.buf.extend_from_slice(data);
    }

    /// Next complete frame body, `Ok(None)` if more bytes are needed, `Err` for an oversize frame.
    pub fn next_frame(&mut self) -> Result<Option<Vec<u8>>, WireError> {
        if self.buf.len() < 4 {
            return Ok(None);
        }
        let n = u32::from_le_bytes([self.buf[0], self.buf[1], self.buf[2], self.buf[3]]) as usize;
        if n > MAX_FRAME {
            return Err(WireError("frame too large"));
        }
        if self.buf.len() < 4 + n {
            return Ok(None);
        }
        let body = self.buf[4..4 + n].to_vec();
        self.buf.drain(..4 + n);
        Ok(Some(body))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(i: u32) -> EntityId {
        EntityId {
            index: i,
            generation: 3,
        }
    }

    fn all_commands() -> Vec<Command> {
        vec![
            Command::SetThrottle(0.5),
            Command::AdjustThrottle(-0.1),
            Command::Turn {
                pitch: 0.1,
                yaw: -0.2,
                roll: 0.3,
            },
            Command::Strafe { x: 1.0, y: -1.0 },
            Command::Brake(true),
            Command::SetAssist(false),
            Command::FireLance,
            Command::FireTorpedo,
            Command::FireMissile,
            Command::SetLanceFreq(4),
            Command::SetShieldFreq(7),
            Command::SetTarget(Some(id(9))),
            Command::SetTarget(None),
            Command::SetPower(Sys::Shields, 1.5),
            Command::SetCoolant(Sys::Engines, 0.4),
            Command::PowerPreset(2),
            Command::AssignRepair {
                team: 1,
                sys: Some(Sys::Reactor),
            },
            Command::AssignRepair { team: 2, sys: None },
            Command::Scan(Some(id(1))),
            Command::Sweep,
            Command::SetDest(Dest::None),
            Command::SetDest(Dest::Point(Vec3::new(1.0, 2.0, 3.0))),
            Command::SetDest(Dest::Body(id(5))),
            Command::AddWaypoint(Vec3::new(-1.0, 0.0, 9.0)),
            Command::Autopilot(true),
            Command::Warp(Some(SectorId::Belt)),
            Command::Warp(None),
            Command::Order(Order::Follow),
            Command::DemandSurrender(id(2)),
            Command::Dock(Some(id(3))),
            Command::Mine(None),
        ]
    }

    #[test]
    fn every_command_round_trips() {
        for c in all_commands() {
            let mut w = Writer::default();
            put_command(&mut w, c);
            let mut r = Reader::new(&w.0);
            assert_eq!(get_command(&mut r), Ok(c), "{c:?}");
            assert!(r.done().is_ok());
        }
    }

    #[test]
    fn bad_input_is_an_error_not_a_panic() {
        assert!(get_command(&mut Reader::new(&[])).is_err());
        assert!(get_command(&mut Reader::new(&[200])).is_err());
        assert!(get_command(&mut Reader::new(&[0, 0, 0])).is_err());
        let nan = f32::NAN.to_le_bytes();
        assert!(get_command(&mut Reader::new(&[0, nan[0], nan[1], nan[2], nan[3]])).is_err());
        assert!(get_command(&mut Reader::new(&[12, 99, 0, 0, 0, 0])).is_err());
        // out-of-range demands are clamped
        let big = 9.0f32.to_le_bytes();
        let c = get_command(&mut Reader::new(&[0, big[0], big[1], big[2], big[3]]));
        assert_eq!(c, Ok(Command::SetThrottle(1.0)));
        assert!(Reader::new(&[2, 0]).str().is_err());
    }

    #[test]
    fn frames_split_across_reads() {
        let mut fb = FrameBuf::default();
        let mut raw = Vec::new();
        write_frame(&mut raw, b"hello").expect("write");
        write_frame(&mut raw, b"world!").expect("write");
        fb.push(&raw[..3]);
        assert_eq!(fb.next_frame(), Ok(None));
        fb.push(&raw[3..9]);
        assert_eq!(fb.next_frame(), Ok(Some(b"hello".to_vec())));
        assert_eq!(fb.next_frame(), Ok(None));
        fb.push(&raw[9..]);
        assert_eq!(fb.next_frame(), Ok(Some(b"world!".to_vec())));
        fb.push(&(MAX_FRAME as u32 + 1).to_le_bytes());
        assert!(fb.next_frame().is_err());
    }

    #[test]
    fn long_strings_are_cut_on_a_char_boundary() {
        let s = "é".repeat(40_000);
        let mut w = Writer::default();
        w.str(&s);
        let back = Reader::new(&w.0).str().expect("valid utf-8");
        assert!(back.len() <= usize::from(u16::MAX) && s.starts_with(&back));
    }
}
