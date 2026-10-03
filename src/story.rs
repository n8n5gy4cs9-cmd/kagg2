//! Story text: menu blurb, mission briefings (≤ 6 lines), saga verses, endings (see `STORY.md`).

pub const TITLE: &str = "KAGG2";
pub const SUBTITLE: &str = "Mars Vikings against the Earth Minions";

/// Menu blurb, one line per entry.
pub const BLURB: [&str; 3] = [
    "2347. The Concord of Earth has sent its Minions to burn Marsgard.",
    "You are Captain of the longship Huginn. Fly the whale-road, break their ships,",
    "then board the wreck, axe in hand, and take the Concord's fleet codes.",
];

/// Title, place and briefing lines of one mission.
pub struct Briefing {
    pub title: &'static str,
    pub place: &'static str,
    pub lines: &'static [&'static str],
}

static BRIEFINGS: [Briefing; 10] = [
    Briefing {
        title: "FIRST RAVEN",
        place: "Mars Orbit",
        lines: &[
            "Jarl Ingrid: \"The Huginn is yours, Captain. Fly her, test her lances",
            "on the drones, and dock at the Forge.\"",
            "This mission is also your tutorial: the objective line at the top",
            "of the screen tells you what to do next. F8 shows help for the station.",
            "Concord scouts are near. Do not be surprised.",
        ],
    },
    Briefing {
        title: "PHOBOS WATCH",
        place: "Phobos",
        lines: &[
            "Jarl Ingrid: \"Raiders hit the Phobos relay every night.\"",
            "Fly there and hold it against two waves.",
            "Bjorn wants your hands on Engineering (F3): shields eat power,",
            "lances eat power, and the reactor has only so much.",
        ],
    },
    Briefing {
        title: "IRON TITHE",
        place: "Asteroid Belt",
        lines: &[
            "Jarl Ingrid: \"The Free Miners pay their Tithe in ore, and the",
            "Concord wants it. Load six ore, scan the stones, then walk two",
            "haulers home to Belt Outpost.\"",
            "Mine with K on a targeted rock; scan with N. Lose a hauler and",
            "the Tithe is lost.",
        ],
    },
    Briefing {
        title: "BOARDING PARTY",
        place: "Asteroid Belt",
        lines: &[
            "Jarl Ingrid: \"A Concord courier carries the Codebook.\"",
            "Break her shields and hull below 60 %, then close in and press B.",
            "Inside, take the data core and get out through the airlock.",
            "Sink her and the Codebook burns with her.",
        ],
    },
    Briefing {
        title: "THE SILENT MOON",
        place: "Deimos",
        lines: &[
            "Jarl Ingrid: \"Our people ride a Concord prison barge.\"",
            "Wait in Deimos' shadow until the convoy comes.",
            "Kill the escorts with missiles, cripple the barge, board it",
            "and open every cell. Do not sink the barge.",
        ],
    },
    Briefing {
        title: "JOVIAN TOLL",
        place: "Ganymede",
        lines: &[
            "Jarl Ingrid: \"Jotun Yards sells hulls. The Concord taxes the road.\"",
            "Dock at Ganymede Port, spend your hacksilver on an upgrade,",
            "then break the toll squad at the gate.",
            "(Warp gates are not built yet; the toll road is a plain fight.)",
        ],
    },
    Briefing {
        title: "WOLVES OF THE BELT",
        place: "Asteroid Belt",
        lines: &[
            "Jarl Ingrid: \"Three skiffs ride with you now.\"",
            "Use Comms (F6) to order the wolves: attack, guard, hold.",
            "A patrol first, then a destroyer. Focus fire wins fleet battles.",
        ],
    },
    Briefing {
        title: "GATE OF FROST",
        place: "Concord forward base, Luna",
        lines: &[
            "Jarl Ingrid: \"Three generators hold the Gate's shield.\"",
            "Destroy them before the Armada arrives (the clock is running).",
            "Then cripple Frost Command, board her and sabotage the reactor.",
        ],
    },
    Briefing {
        title: "LUNA SIEGE",
        place: "Luna",
        lines: &[
            "Jarl Ingrid: \"Break the fortress ring, then take the Bulwark.\"",
            "Admiral-Prefect Halden commands her; his guards are tougher",
            "than anything you have boarded. Bring armour and the rail gun.",
        ],
    },
    Briefing {
        title: "RAGNAROK OVER EARTH",
        place: "Earth Orbit",
        lines: &[
            "Jarl Ingrid: \"Everything we are rides on this day.\"",
            "Break the Armada's screen, cripple Aegis Prime and board her.",
            "Prefect Voss waits on the bridge. End him.",
        ],
    },
];

/// Briefing for a mission (1-based); `None` for unknown missions.
pub fn briefing(mission: u8) -> Option<&'static Briefing> {
    BRIEFINGS.get(usize::from(mission).checked_sub(1)?)
}

/// Saga verses: `[win, loss]` for each mission.
static VERSES: [[&str; 2]; 10] = [
    [
        "Iron wakes on the red hill; ravens ride the reactor's breath.",
        "The first raven fell early. The Forge still keeps its fire.",
    ],
    [
        "Phobos holds. Raiders broke on the stone like surf on the skerry.",
        "The relay burned. Even small moons remember.",
    ],
    [
        "Ore for the Tithe, blood for the road. The haulers sing tonight.",
        "The haulers drifted, the Tithe was lost. Hunger walks the Belt.",
    ],
    [
        "The Codebook lies in rust-red hands. Words are sharper than axes.",
        "The courier burned. The Codebook is ash in cold vacuum.",
    ],
    [
        "Chains fell from Martian wrists. The silent moon kept their secret.",
        "The barge went dark with all its songs inside.",
    ],
    [
        "Jotun hammers rang, the toll was paid in lance-fire.",
        "Ganymede closed its ports and the road went red.",
    ],
    [
        "The wolves ran the Belt and left it empty of white armour.",
        "The pack was scattered. The Belt howls alone.",
    ],
    [
        "The Gate of Frost has no shield, and Frost Command no reactor.",
        "The Armada came early and the Gate stayed shut.",
    ],
    [
        "Bulwark broke on Luna's light. Halden's last order went unheard.",
        "Luna's ring held. The whale-road is closed for now.",
    ],
    [
        "Aegis Prime fell silent. Voss kept his promise, and so did we.",
        "The last longship fell. The saga ends mid-line.",
    ],
];

/// Saga verse shown on the debrief.
pub fn verse(mission: u8, won: bool) -> &'static str {
    VERSES
        .get(usize::from(mission).saturating_sub(1))
        .map_or("", |v| v[usize::from(!won)])
}

/// How the war ends (chosen after Voss falls).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ending {
    /// Destroy the fleet codes: Earth and Mars equal, an uneasy peace.
    BreakArmada,
    /// Use the codes: Mars rules.
    TakeThrone,
}

impl Ending {
    pub fn code(self) -> u8 {
        match self {
            Ending::BreakArmada => 1,
            Ending::TakeThrone => 2,
        }
    }

    pub fn from_code(c: u8) -> Option<Ending> {
        match c {
            1 => Some(Ending::BreakArmada),
            2 => Some(Ending::TakeThrone),
            _ => None,
        }
    }

    pub fn epilogue(self) -> &'static [&'static str] {
        match self {
            Ending::BreakArmada => &[
                "You burn the fleet codes in the Aegis Prime reactor.",
                "The Armada drifts, silent and leaderless. Earth sends envoys,",
                "Mars sends the Jarls, and for the first time the two sit as equals.",
                "The peace is uneasy and the skalds are suspicious of it,",
                "but nobody is shooting, and the saga is yours to finish.",
            ],
            Ending::TakeThrone => &[
                "You take the fleet codes to the throne room of Geneva.",
                "The Armada answers to Marsgard now, every ship, every Minion.",
                "The Jarls call it victory. Earth calls it the Rust Crown.",
                "Skald Eirik writes the verse; Sigrun asks who will ever dare",
                "to take the codes from you. Nobody answers.",
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_mission_has_briefing_and_both_verses() {
        for m in 1..=10 {
            let b = briefing(m).expect("briefing");
            assert!(b.lines.len() <= 6 && !b.lines.is_empty());
            assert!(!verse(m, true).is_empty() && !verse(m, false).is_empty());
            assert_ne!(verse(m, true), verse(m, false));
        }
        assert!(briefing(0).is_none() && briefing(11).is_none());
    }

    #[test]
    fn endings_round_trip() {
        for e in [Ending::BreakArmada, Ending::TakeThrone] {
            assert_eq!(Ending::from_code(e.code()), Some(e));
            assert!(!e.epilogue().is_empty());
        }
        assert_eq!(Ending::from_code(0), None);
    }
}
