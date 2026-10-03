//! Starbase economy (T16): commodity prices, trading, refuel/repair/rearm, shipyard upgrades.

use super::ops::{COMMODITIES, Faction, Ops};

pub const NAMES: [&str; COMMODITIES] = [
    "ORE",
    "ICE",
    "IRON",
    "STEEL",
    "GRAIN",
    "MEAD",
    "MEDICINE",
    "ELECTRONICS",
    "WEAPONS",
    "MACHINERY",
    "LUXURIES",
    "RUNESTONES",
];
const BASE: [i32; COMMODITIES] = [20, 12, 30, 55, 18, 40, 90, 120, 160, 100, 200, 260];

fn hash(label: &str, a: u64, b: u64) -> f32 {
    let mut h = 0xcbf2_9ce4_8422_2325u64;
    for byte in label.bytes() {
        h = (h ^ byte as u64).wrapping_mul(0x0000_0100_0000_01b3);
    }
    h ^= a.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ b.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    h ^= h >> 32;
    h = h.wrapping_mul(0x94D0_49BB_1331_11EB);
    h ^= h >> 29;
    (h >> 40) as f32 / (1u64 << 24) as f32
}

/// Buy price at `station` for commodity `c` at sim time `t` (each station has its own bias,
/// and prices drift every 90 seconds).
pub fn price(station: &str, c: usize, t: f32) -> i32 {
    let bias = (hash(station, c as u64, 0) - 0.5) * 0.6;
    let vol = (hash(station, c as u64, 1 + (t / 90.0) as u64) - 0.5) * 0.3;
    ((BASE[c] as f32 * (1.0 + bias) * (1.0 + vol)).round() as i32).max(1)
}

/// What the station pays for goods.
pub fn sell_price(station: &str, c: usize, t: f32) -> i32 {
    ((price(station, c, t) as f32 * 0.88).floor() as i32).max(1)
}

pub fn buy(o: &mut Ops, station: &str, c: usize, qty: u16, t: f32) -> Result<(), &'static str> {
    let cost = price(station, c, t) * qty as i32;
    if qty == 0 || c >= COMMODITIES {
        return Err("NOTHING TO BUY");
    }
    if o.cargo.used() + qty > o.cargo.capacity {
        return Err("HOLD FULL");
    }
    if o.cargo.credits < cost {
        return Err("NOT ENOUGH CREDITS");
    }
    o.cargo.credits -= cost;
    o.cargo.hold[c] += qty;
    Ok(())
}

pub fn sell(o: &mut Ops, station: &str, c: usize, qty: u16, t: f32) -> Result<(), &'static str> {
    if qty == 0 || c >= COMMODITIES || o.cargo.hold[c] < qty {
        return Err("NOTHING TO SELL");
    }
    o.cargo.hold[c] -= qty;
    o.cargo.credits += sell_price(station, c, t) * qty as i32;
    Ok(())
}

/// Service price multiplier: the Forge is home (half price), allied bases normal, others dear.
pub fn service_mult(station: &str, f: Faction) -> f32 {
    if station == "Forge" {
        0.5
    } else if f == Faction::Mars {
        1.0
    } else {
        1.5
    }
}

pub fn refuel_cost(o: &Ops, mult: f32) -> i32 {
    ((o.nav.max_fuel - o.nav.fuel).max(0.0) * 2.0 * mult).ceil() as i32
}

pub fn repair_cost(o: &Ops, mult: f32) -> i32 {
    let hull = (o.max_hull - o.hull).max(0.0);
    let sys: f32 = o.sys.health.iter().map(|h| 100.0 - h).sum();
    let crew = (o.sys.crew_max - o.sys.crew) as f32 * 10.0;
    ((hull + sys * 2.0 + crew) * mult).ceil() as i32
}

pub fn rearm_cost(o: &Ops, mult: f32) -> i32 {
    let t = (o.max_torpedoes() - o.torpedoes) as f32 * 40.0;
    let m = (o.max_missiles() - o.missiles) as f32 * 25.0;
    ((t + m) * mult).ceil() as i32
}

fn pay(o: &mut Ops, cost: i32) -> Result<(), &'static str> {
    if cost == 0 {
        return Err("NOTHING TO DO");
    }
    if o.cargo.credits < cost {
        return Err("NOT ENOUGH CREDITS");
    }
    o.cargo.credits -= cost;
    Ok(())
}

pub fn refuel(o: &mut Ops, mult: f32) -> Result<(), &'static str> {
    pay(o, refuel_cost(o, mult))?;
    o.nav.fuel = o.nav.max_fuel;
    Ok(())
}

pub fn repair(o: &mut Ops, mult: f32) -> Result<(), &'static str> {
    pay(o, repair_cost(o, mult))?;
    o.hull = o.max_hull;
    o.sys.repair_all();
    Ok(())
}

pub fn rearm(o: &mut Ops, mult: f32) -> Result<(), &'static str> {
    pay(o, rearm_cost(o, mult))?;
    o.torpedoes = o.max_torpedoes();
    o.missiles = o.max_missiles();
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Upgrade {
    Lance,
    Shield,
    Reactor,
    Hull,
    Bays,
}

impl Upgrade {
    pub const ALL: [Upgrade; 5] = [
        Upgrade::Lance,
        Upgrade::Shield,
        Upgrade::Reactor,
        Upgrade::Hull,
        Upgrade::Bays,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Upgrade::Lance => "RUNE LANCES",
            Upgrade::Shield => "SHIELD GENERATOR",
            Upgrade::Reactor => "REACTOR CORE",
            Upgrade::Hull => "HULL PLATING",
            Upgrade::Bays => "ORDNANCE BAYS",
        }
    }

    pub fn blurb(self) -> &'static str {
        match self {
            Upgrade::Lance => "+30% lance damage per level",
            Upgrade::Shield => "+30% shield capacity per level",
            Upgrade::Reactor => "+20% power budget per level",
            Upgrade::Hull => "+25% hull points per level",
            Upgrade::Bays => "+2 torpedoes and +2 missiles per level",
        }
    }

    pub fn level(self, o: &Ops) -> u8 {
        match self {
            Upgrade::Lance => o.up.lance,
            Upgrade::Shield => o.up.shield,
            Upgrade::Reactor => o.up.reactor,
            Upgrade::Hull => o.up.hull,
            Upgrade::Bays => o.up.bays,
        }
    }

    /// Cost of the next level (`None` at max).
    pub fn cost(self, o: &Ops) -> Option<i32> {
        match self.level(o) {
            1 => Some(400),
            2 => Some(900),
            _ => None,
        }
    }
}

pub fn upgrade(o: &mut Ops, u: Upgrade) -> Result<(), &'static str> {
    let cost = u.cost(o).ok_or("ALREADY MAX LEVEL")?;
    pay(o, cost)?;
    match u {
        Upgrade::Lance => o.up.lance += 1,
        Upgrade::Shield => o.up.shield += 1,
        Upgrade::Reactor => o.up.reactor += 1,
        Upgrade::Hull => o.up.hull += 1,
        Upgrade::Bays => o.up.bays += 1,
    }
    o.recompute();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::flight::LONGSHIP;

    fn ship() -> Ops {
        let mut o = Ops::for_class(&LONGSHIP);
        o.cargo.credits = 1000;
        o
    }

    #[test]
    fn prices_differ_by_station_and_drift_over_time() {
        let spread =
            (0..COMMODITIES).any(|c| price("Forge", c, 0.0) != price("Belt Outpost", c, 0.0));
        let drift = (0..COMMODITIES).any(|c| price("Forge", c, 0.0) != price("Forge", c, 200.0));
        assert!(spread && drift);
        assert!(sell_price("Forge", 3, 0.0) < price("Forge", 3, 0.0));
    }

    #[test]
    fn buying_and_selling_respect_credits_and_hold() {
        let mut o = ship();
        assert_eq!(buy(&mut o, "Forge", 2, 1000, 0.0), Err("HOLD FULL"));
        assert!(buy(&mut o, "Forge", 2, 5, 0.0).is_ok());
        assert_eq!(o.cargo.hold[2], 5);
        assert!(o.cargo.credits < 1000);
        o.cargo.credits = 0;
        assert_eq!(buy(&mut o, "Forge", 11, 1, 0.0), Err("NOT ENOUGH CREDITS"));
        assert!(sell(&mut o, "Forge", 2, 5, 0.0).is_ok());
        assert!(o.cargo.credits > 0 && o.cargo.hold[2] == 0);
        assert!(sell(&mut o, "Forge", 2, 1, 0.0).is_err());
    }

    #[test]
    fn services_cost_credits_and_restore_stats() {
        let mut o = ship();
        o.hull = 100.0;
        o.torpedoes = 2;
        o.nav.fuel = 40.0;
        o.sys.damage(crate::sim::systems::Sys::Engines, 40.0);
        assert!(repair_cost(&o, 1.0) > 0);
        assert!(repair(&mut o, 0.5).is_ok());
        assert_eq!(o.hull, o.max_hull);
        assert_eq!(o.sys.health[0], 100.0);
        assert!(rearm(&mut o, 1.0).is_ok());
        assert_eq!(o.torpedoes, o.max_torpedoes());
        assert!(refuel(&mut o, 1.0).is_ok());
        assert_eq!(o.nav.fuel, 100.0);
        assert_eq!(refuel(&mut o, 1.0), Err("NOTHING TO DO"));
    }

    #[test]
    fn upgrades_raise_stats_and_cap_at_three() {
        let mut o = ship();
        o.cargo.credits = 5000;
        let (h0, s0) = (o.max_hull, o.max_shield);
        upgrade(&mut o, Upgrade::Hull).expect("hull 2");
        upgrade(&mut o, Upgrade::Shield).expect("shield 2");
        assert!(o.max_hull > h0 && o.max_shield > s0);
        upgrade(&mut o, Upgrade::Bays).expect("bays 2");
        assert_eq!(o.max_torpedoes(), 10);
        upgrade(&mut o, Upgrade::Reactor).expect("reactor 2");
        upgrade(&mut o, Upgrade::Reactor).expect("reactor 3");
        assert_eq!(upgrade(&mut o, Upgrade::Reactor), Err("ALREADY MAX LEVEL"));
        assert!(o.sys.output() > 6.5);
    }
}
