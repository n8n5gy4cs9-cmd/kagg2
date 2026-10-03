//! In-game help text (T29): per-station overlay (F8), boarding controls card, and the pages
//! of the main-menu Help screen. Plain data, no drawing.

use super::stations::Tab;

pub const FLIGHT: &[&str] = &[
    "W/S  THROTTLE UP/DOWN      SHIFT  FULL      X  ZERO",
    "A/D  YAW      UP/DOWN  PITCH      Q/E  ROLL",
    "LEFT/RIGHT  STRAFE      R/F  STRAFE UP/DOWN      SPACE  BRAKE",
    "ENTER  LANCES      Z  TORPEDO      V  MISSILE (LOCK FIRST)",
    "T  NEXT TARGET      G  CLEAR TARGET      N  SCAN TARGET",
    "P  AUTOPILOT      O  GO TO TARGET      L  DOCK / UNDOCK      K  MINE",
    "B  BOARD A CRIPPLED SHIP      C  CAMERA      M  FLIGHT ASSIST",
    "F1-F7  STATIONS      TAB  CREW AUTO      F8  STATION HELP",
    "-/=  VOLUME      F11  MUSIC      ESC  MENU",
];

pub const BOARDING: &[&str] = &[
    "W/A/S/D  MOVE      MOUSE  LOOK      SPACE  JUMP      C  CROUCH",
    "LEFT CLICK / ENTER  FIRE      1-6  CHOOSE WEAPON",
    "E  USE DOORS, TERMINALS, CELLS AND THE AIRLOCK",
    "COLLECT KEYS (BRONZE, SILVER, GOLD) TO OPEN LOCKED DOORS",
    "WATCH THE OBJECTIVE ON THE HUD. ALARMS CALL EVERY GUARD.",
    "ESC  ABORT BOARDING AND RETURN TO YOUR SHIP",
];

/// Overlay text for a station tab.
pub fn station(tab: Tab) -> &'static [&'static str] {
    match tab {
        Tab::Helm => &[
            "HELM (F1)  FLY THE SHIP",
            "THROTTLE SETS TARGET SPEED; THE SHIP HOLDS IT.",
            "SPACE BRAKES TO A STOP. M TOGGLES FLIGHT ASSIST.",
            "CLICK THE VIEW OR HOLD ENTER TO FIRE LANCES.",
            "P / O  AUTOPILOT AND GO-TO TARGET.",
            "TAB LETS THE HELM CREW FLY FOR YOU (AUTO).",
        ],
        Tab::Weapons => &[
            "WEAPONS (F2)  LANCES, TORPEDOES, MISSILES",
            "LANCES: HITSCAN, HEAT UP - FIRE IN BURSTS.",
            "TORPEDOES (Z): SLOW, HEAVY, BEST AGAINST OPEN HULLS.",
            "MISSILES (V): NEED A LOCK - TARGET FIRST (T).",
            "[ ] SET LANCE FREQUENCY; MATCHING THE ENEMY SHIELD",
            "FREQUENCY (FROM A SCAN) DOES MORE DAMAGE.",
        ],
        Tab::Engineering => &[
            "ENGINEERING (F3)  POWER AND HEAT",
            "THE REACTOR HAS A FIXED BUDGET. SLIDERS SHARE IT",
            "BETWEEN SYSTEMS; COOLANT SLIDERS FIGHT HEAT.",
            "1 / 2 / 3 APPLY POWER PRESETS (BALANCED, WEAPONS, SHIELDS).",
            "OVERHEATED SYSTEMS BURN OUT - WATCH THE HEAT BARS.",
        ],
        Tab::Damage => &[
            "DAMAGE CONTROL (F4)  REPAIR",
            "THREE REPAIR TEAMS. SEND EACH TO A DAMAGED SYSTEM.",
            "WITHOUT LIFE SUPPORT YOU LOSE CREW OVER TIME.",
            "HULL DAMAGE IS FIXED ONLY AT A STARBASE.",
        ],
        Tab::Science => &[
            "SCIENCE (F5)  SCANS",
            "TARGET A SHIP (T), PRESS N AND HOLD THE SCAN.",
            "A FINISHED SCAN SHOWS SHIELD FREQUENCY, WEAK POINTS",
            "AND HOW HARD THE SHIP IS TO BOARD.  U  LONG-RANGE SWEEP.",
        ],
        Tab::Comms => &[
            "COMMS (F6)  HAIL AND ORDER",
            "H HAILS THE TARGET. NUMBER KEYS CHOOSE A REPLY.",
            "ORDER WINGMEN: ATTACK, GUARD, HOLD, FOLLOW.",
            "DEMAND SURRENDER FROM A WEAK SHIP - IT MAY OBEY.",
        ],
        Tab::Map => &[
            "MAP (F7)  NAVIGATION",
            "CLICK A BODY TO SET THE DESTINATION; ADD WAYPOINTS.",
            "AUTOPILOT AVOIDS PLANETS AND MOONS.",
            "WARP COSTS FUEL; REFUEL AT A STARBASE (DOCK WITH L).",
        ],
    }
}

/// One page of the Help screen: title and lines.
pub struct Page {
    pub title: &'static str,
    pub lines: Vec<&'static str>,
}

pub fn pages() -> Vec<Page> {
    let mut v = vec![
        Page {
            title: "FLIGHT AND WEAPONS",
            lines: FLIGHT.to_vec(),
        },
        Page {
            title: "BOARDING",
            lines: BOARDING.to_vec(),
        },
    ];
    for t in Tab::ALL {
        let s = station(t);
        v.push(Page {
            title: s[0],
            lines: s[1..].to_vec(),
        });
    }
    v.push(Page {
        title: "TIPS",
        lines: vec![
            "SHIELDS HAVE FOUR FACINGS - TURN YOUR STRONG SIDE TO THE ENEMY.",
            "CRIPPLE, DO NOT DESTROY: A SHIP WITH HULL UNDER 60 % AND NO",
            "SHIELDS CAN BE BOARDED FOR LOOT AND MISSION OBJECTIVES.",
            "DOCK AT FRIENDLY STARBASES TO REPAIR, REARM AND BUY UPGRADES.",
            "YOUR CREW CAN RUN ANY STATION: TAB TOGGLES AUTO.",
            "SETTINGS HAS DIFFICULTY, MOUSE AND KEY BINDINGS.",
        ],
    });
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_station_has_help_and_pages_cover_everything() {
        for t in Tab::ALL {
            assert!(station(t).len() >= 4);
        }
        assert_eq!(pages().len(), 2 + Tab::ALL.len() + 1);
        assert!(pages().iter().all(|p| !p.lines.is_empty()));
    }
}
