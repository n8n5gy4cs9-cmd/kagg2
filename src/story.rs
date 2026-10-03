//! Static story text shown on menu and briefing screens (full missions arrive in T25).

pub const TITLE: &str = "KAGGA2";
pub const SUBTITLE: &str = "Mars Vikings against the Earth Minions";

/// Menu blurb, one line per entry.
pub const BLURB: [&str; 3] = [
    "2347. The Concord of Earth has sent its Minions to burn Marsgard.",
    "You are Captain of the longship Huginn. Fly the whale-road, break their ships,",
    "then board the wreck, axe in hand, and take the Concord's fleet codes.",
];

/// Briefing text for a mission (1-based); `None` for unknown missions.
pub fn briefing(mission: u8) -> Option<[&'static str; 3]> {
    match mission {
        1 => Some([
            "FIRST RAVEN - Mars Orbit",
            "Jarl Ingrid: \"The Huginn is yours. Fly her, test her lances on the drones,",
            "and dock at the Forge. Concord scouts are near; do not be surprised.\"",
        ]),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_mission_has_briefing() {
        assert!(briefing(1).is_some());
        assert!(briefing(0).is_none());
    }
}
