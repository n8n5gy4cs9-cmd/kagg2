//! Per-ship operations state: hull, shields, weapons, systems, navigation, scanning and cargo.

use glam::Vec3;

use super::flight::ShipClass;
use super::sector::SectorId;
use super::systems::Systems;
use super::world::EntityId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Faction {
    Neutral,
    /// Mars Vikings (the player and allies).
    Mars,
    /// Earth Minions.
    Minion,
}

impl Faction {
    pub fn hostile_to(self, other: Faction) -> bool {
        matches!(
            (self, other),
            (Faction::Mars, Faction::Minion) | (Faction::Minion, Faction::Mars)
        )
    }

    pub fn name(self) -> &'static str {
        match self {
            Faction::Neutral => "NEUTRAL",
            Faction::Mars => "MARS",
            Faction::Minion => "MINION",
        }
    }
}

/// Shield facings.
pub const FORE: usize = 0;
pub const AFT: usize = 1;
pub const PORT: usize = 2;
pub const STARBOARD: usize = 3;
pub const FACING_NAMES: [&str; 4] = ["FORE", "AFT", "PORT", "STBD"];

/// Base stats of a ship class (before upgrades).
#[derive(Debug, Clone, Copy)]
pub struct ClassStats {
    pub hull: f32,
    pub shield: f32,
    pub torpedoes: u8,
    pub missiles: u8,
    pub crew: u32,
    /// Boarding difficulty 1–5 (shown by science).
    pub boarding: u8,
}

pub fn class_stats(name: &str) -> ClassStats {
    let (hull, shield, torpedoes, missiles, crew, boarding) = match name {
        "Longship" => (260.0, 120.0, 8, 12, 24, 3),
        "Skiff" => (110.0, 60.0, 2, 4, 4, 1),
        "Interceptor" => (60.0, 35.0, 0, 2, 2, 1),
        "Cutter" => (120.0, 70.0, 2, 3, 8, 2),
        "Frigate" => (260.0, 130.0, 4, 4, 20, 3),
        "Destroyer" => (420.0, 200.0, 6, 6, 40, 4),
        "Dreadnought" => (1100.0, 480.0, 12, 12, 120, 5),
        _ => (200.0, 80.0, 2, 2, 10, 2),
    };
    ClassStats {
        hull,
        shield,
        torpedoes,
        missiles,
        crew,
        boarding,
    }
}

pub const COMMODITIES: usize = 12;

#[derive(Debug, Clone)]
pub struct Cargo {
    pub credits: i32,
    pub hold: [u16; COMMODITIES],
    pub capacity: u16,
}

impl Cargo {
    pub fn used(&self) -> u16 {
        self.hold.iter().sum()
    }
}

/// Shipyard upgrade levels, `1..=3` (`1` = stock).
#[derive(Debug, Clone, Copy)]
pub struct Upgrades {
    pub lance: u8,
    pub shield: u8,
    pub reactor: u8,
    pub hull: u8,
    pub bays: u8,
}

impl Default for Upgrades {
    fn default() -> Self {
        Upgrades {
            lance: 1,
            shield: 1,
            reactor: 1,
            hull: 1,
            bays: 1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Dest {
    None,
    Point(Vec3),
    Body(EntityId),
}

#[derive(Debug, Clone)]
pub struct Nav {
    pub dest: Dest,
    /// Further waypoints after `dest` (course plot).
    pub route: Vec<Vec3>,
    pub autopilot: bool,
    pub fuel: f32,
    pub max_fuel: f32,
    /// Warp target and charge `0..=1`.
    pub warp: Option<(SectorId, f32)>,
}

#[derive(Debug, Clone, Default)]
pub struct Scan {
    pub target: Option<EntityId>,
    pub progress: f32,
    pub done: Vec<EntityId>,
    /// Remaining seconds of long-range sweep boost and its cooldown.
    pub sweep: f32,
    pub sweep_cd: f32,
}

#[derive(Debug, Clone)]
pub struct Ops {
    pub hull: f32,
    pub max_hull: f32,
    pub shields: [f32; 4],
    pub max_shield: f32,
    pub shield_freq: u8,
    /// Seconds since the shields were last hit (regen starts after a delay).
    pub shield_idle: f32,
    pub lance_freq: u8,
    pub lance_cd: f32,
    pub torpedoes: u8,
    pub missiles: u8,
    pub torp_cd: f32,
    pub miss_cd: f32,
    /// Aim error (radians) for lance shots; the player's guns are perfect.
    pub aim_spread: f32,
    pub target: Option<EntityId>,
    pub sys: Systems,
    pub nav: Nav,
    pub scan: Scan,
    pub cargo: Cargo,
    pub up: Upgrades,
    pub class_name: &'static str,
    pub boarding: u8,
    /// Docked at this station, at this world-space offset from it.
    pub docked: Option<(EntityId, Vec3)>,
    /// Mining bot target and progress.
    pub mining: Option<(EntityId, f32)>,
    /// Surrendered or crippled: drifting, harmless, boardable.
    pub disabled: bool,
    pub dead: bool,
}

impl Ops {
    pub fn for_class(class: &ShipClass) -> Ops {
        let st = class_stats(class.name);
        let mut o = Ops {
            hull: st.hull,
            max_hull: st.hull,
            shields: [st.shield; 4],
            max_shield: st.shield,
            shield_freq: 0,
            shield_idle: 10.0,
            lance_freq: 0,
            lance_cd: 0.0,
            torpedoes: st.torpedoes,
            missiles: st.missiles,
            torp_cd: 0.0,
            miss_cd: 0.0,
            aim_spread: 0.0,
            target: None,
            sys: Systems::new(st.crew),
            nav: Nav {
                dest: Dest::None,
                route: Vec::new(),
                autopilot: false,
                fuel: 100.0,
                max_fuel: 100.0,
                warp: None,
            },
            scan: Scan::default(),
            cargo: Cargo {
                credits: 0,
                hold: [0; COMMODITIES],
                capacity: (st.crew as u16).clamp(8, 60),
            },
            up: Upgrades::default(),
            class_name: class.name,
            boarding: st.boarding,
            docked: None,
            mining: None,
            disabled: false,
            dead: false,
        };
        o.recompute();
        o.shields = [o.max_shield; 4];
        o.hull = o.max_hull;
        o
    }

    /// Re-derive maxima from class stats and upgrade levels (keeps current values clamped).
    pub fn recompute(&mut self) {
        let st = class_stats(self.class_name);
        let lv = |l: u8| l.clamp(1, 3) as f32 - 1.0;
        self.max_hull = st.hull * (1.0 + 0.25 * lv(self.up.hull));
        self.max_shield = st.shield * (1.0 + 0.3 * lv(self.up.shield));
        self.sys.reactor_level = self.up.reactor.clamp(1, 3);
        self.hull = self.hull.min(self.max_hull);
        for s in &mut self.shields {
            *s = s.min(self.max_shield);
        }
    }

    pub fn max_torpedoes(&self) -> u8 {
        class_stats(self.class_name).torpedoes + 2 * (self.up.bays.clamp(1, 3) - 1)
    }

    pub fn max_missiles(&self) -> u8 {
        class_stats(self.class_name).missiles + 2 * (self.up.bays.clamp(1, 3) - 1)
    }

    /// Lance damage multiplier from the shipyard.
    pub fn lance_mult(&self) -> f32 {
        1.0 + 0.3 * (self.up.lance.clamp(1, 3) as f32 - 1.0)
    }

    pub fn shield_total(&self) -> f32 {
        self.shields.iter().sum()
    }

    /// Hull below 60 % with the shields nearly gone: the ship can be clamped on to and boarded.
    pub fn crippled(&self) -> bool {
        self.hull < self.max_hull * 0.6 && self.shield_total() < self.max_shield * 4.0 * 0.15
    }
}
