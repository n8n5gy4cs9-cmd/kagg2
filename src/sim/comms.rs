//! Comms (T15): hail dialogs, taunt grammar, station chatter.
//!
//! Pure functions: they return text and options, the UI turns choices into `Command`s.
// ref: REFERENCE_tobe_DELETED infinite-taunt.c (idea only: small grammar)

use super::ai::Order;
use super::ops::Faction;
use super::world::{EntityId, Kind, World};

/// Procedural insult from a small grammar, deterministic in `seed`.
pub fn taunt(seed: u64) -> String {
    const OPEN: [&str; 6] = [
        "Hear me, Viking",
        "Little raider",
        "Mars-spawn",
        "Mead-drinker",
        "Rust-bucket captain",
        "Scrap-hauler",
    ];
    const MID: [&str; 6] = [
        "your longship is a floating coffin",
        "the Prefect will mount your helm on a wall",
        "Earth's lances will carve your runes off",
        "no saga will remember you",
        "your gods cannot find you out here",
        "we will melt your hull for ballast",
    ];
    const END: [&str; 5] = [
        "Surrender now.",
        "Run while you can.",
        "This ends here.",
        "Pray to your hammer-god.",
        "Die loudly.",
    ];
    let pick = |k: u64, n: usize| ((seed >> (k * 11)) % n as u64) as usize;
    format!(
        "{}, {}. {}",
        OPEN[pick(0, 6)],
        MID[pick(1, 6)],
        END[pick(2, 5)]
    )
}

/// What the player can say after a hail.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reply {
    RequestDock,
    AskNews,
    DemandSurrender,
    Taunt,
    Order(Order),
    Goodbye,
}

#[derive(Debug, Clone)]
pub struct Hail {
    pub from: &'static str,
    pub greeting: String,
    pub options: Vec<(&'static str, Reply)>,
}

fn news(sector: &str, seed: u64) -> &'static str {
    let _ = sector;
    const NEWS: [&str; 5] = [
        "Minion patrols have grown bolder near the Belt.",
        "The Prefect is gathering a fleet at Earth.",
        "Forge says the mead is cheap, the runestones dear.",
        "Pirates? No, those were Minion scouts. Be careful.",
        "A frequency trick: scan them, then tune your lances to their shields.",
    ];
    NEWS[(seed % 5) as usize]
}

/// Opening of a hail from the player's ship to `to`.
pub fn hail(w: &World, from: EntityId, to: EntityId) -> Option<Hail> {
    let t = w.get(to)?;
    let me = w.get(from)?;
    let name = t.label;
    let dist = me.pos.distance(t.pos);
    if dist > 25_000.0 {
        return Some(Hail {
            from: name,
            greeting: "...(static) Out of comms range.".into(),
            options: vec![("CLOSE CHANNEL", Reply::Goodbye)],
        });
    }
    let hostile = t.faction.hostile_to(me.faction);
    let mut options: Vec<(&'static str, Reply)> = Vec::new();
    let greeting = match (t.kind, t.faction) {
        (Kind::Station, Faction::Minion) => {
            options.push(("TAUNT", Reply::Taunt));
            "Concord Fort: You are in restricted Earth space. Withdraw or be destroyed.".to_owned()
        }
        (Kind::Station, _) => {
            options.push(("REQUEST DOCKING", Reply::RequestDock));
            options.push(("ASK FOR NEWS", Reply::AskNews));
            format!("{name}: Welcome, Huginn. Docking bays are open. What do you need?")
        }
        (Kind::Ship, _) if hostile => {
            options.push(("DEMAND SURRENDER", Reply::DemandSurrender));
            options.push(("TAUNT", Reply::Taunt));
            format!("{name}: Channel open. Make it quick, Viking.")
        }
        (Kind::Ship, Faction::Mars) if t.ai.is_some() => {
            options.push(("WINGMEN: ATTACK MY TARGET", Reply::Order(Order::Attack)));
            options.push(("WINGMEN: GUARD ME", Reply::Order(Order::Guard)));
            options.push(("WINGMEN: FORM UP", Reply::Order(Order::Follow)));
            options.push(("WINGMEN: FALL BACK", Reply::Order(Order::Retreat)));
            format!("{name}: Ready, captain. Orders?")
        }
        (Kind::Ship, _) => format!("{name}: Just hauling cargo. No trouble, please."),
        _ => format!("{name}: (no one answers - it is only a rock)"),
    };
    options.push(("CLOSE CHANNEL", Reply::Goodbye));
    Some(Hail {
        from: name,
        greeting,
        options,
    })
}

/// Text answer to a reply that has no direct sim effect (news, taunts).
pub fn answer(w: &World, to: EntityId, reply: Reply, seed: u64) -> Option<String> {
    let t = w.get(to)?;
    match reply {
        Reply::AskNews => Some(format!("{}: {}", t.label, news(w.sector.name(), seed))),
        Reply::Taunt => Some(format!("{}: {}", t.label, taunt(seed))),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::{SectorId, new_game};

    #[test]
    fn taunts_vary_and_are_deterministic() {
        assert_eq!(taunt(5), taunt(5));
        let set: std::collections::BTreeSet<String> = (0..50u64)
            .map(|s| taunt(s.wrapping_mul(0x9E37_79B9_7F4A_7C15)))
            .collect();
        assert!(set.len() > 10);
    }

    #[test]
    fn hails_depend_on_who_answers() {
        let w = new_game(1, SectorId::MarsOrbit);
        let me = w.player.expect("player");
        let forge = w
            .iter()
            .find(|(_, e)| e.label == "Forge")
            .map(|(i, _)| i)
            .expect("forge");
        let h = hail(&w, me, forge).expect("hail");
        assert!(h.options.iter().any(|o| o.1 == Reply::RequestDock));
        let w2 = new_game(1, SectorId::EarthOrbit);
        let me2 = w2.player.expect("player");
        let fort = w2
            .iter()
            .find(|(_, e)| e.label.contains("Fort"))
            .map(|(i, _)| i)
            .expect("fort");
        let h2 = hail(&w2, me2, fort).expect("hail");
        assert!(!h2.options.iter().any(|o| o.1 == Reply::RequestDock));
    }
}
