//! Campaign mission engine (T25): a mission is a list of stages; each stage has a goal, timed
//! beats (dialogue and spawns), an optional "must not be lost" ship and an optional time limit.
//! Headless: the scene feeds it the world and the step events, then reads the notes it emits.

use glam::Vec3;

use super::ai::{self, Ai, Mode};
use super::event::Event;
use super::flight::{HAULER, SKIFF};
use super::ops::{Dest, Faction, Upgrades};
use super::sector::ship;
use super::world::{EntityId, Kind, World};
use crate::board::Objective;

/// Where a spawn offset is measured from.
#[derive(Debug, Clone, Copy)]
pub enum Anchor {
    /// The player's ship; the offset is in its own frame (`-z` is ahead).
    Player,
    /// A named body of the sector (world-frame offset).
    Body(&'static str),
}

/// How spawned ships behave.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Attack,
    Patrol,
    /// Hostile but motionless and unarmed (target drones, shield generators).
    Drone,
    /// Marsgard wingman skiffs.
    Ally,
    /// Marsgard haulers that fly to the stage's `Escort` destination.
    Hauler,
}

#[derive(Debug, Clone, Copy)]
pub struct Spawn {
    pub anchor: Anchor,
    pub off: [f32; 3],
    pub role: Role,
    /// Lowercase Minion class names; for `Ally`/`Hauler` only the count matters.
    pub classes: &'static [&'static str],
    /// Overrides the class name as the ship label (`Courier`, `Prison Barge`, ...).
    pub label: Option<&'static str>,
}

#[derive(Debug, Clone, Copy)]
pub enum Act {
    Say(&'static str, &'static str),
    Spawn(Spawn),
}

/// Something that happens `at` seconds after the stage starts.
#[derive(Debug, Clone, Copy)]
pub struct Beat {
    pub at: f32,
    pub act: Act,
}

#[derive(Debug, Clone, Copy)]
pub enum Goal {
    Throttle(f32),
    /// The player has a weapons target selected.
    Target,
    Reach {
        body: &'static str,
        range: f32,
    },
    DockAt(&'static str),
    Scan(u32),
    Cargo {
        commodity: usize,
        qty: u16,
    },
    /// Buy any shipyard upgrade.
    Upgrade,
    Wait(f32),
    /// No active hostile ship is left.
    Clear,
    /// No active hostile ship is left apart from those whose label starts with the prefix.
    ClearBut(&'static str),
    /// No ship with this label prefix is left.
    Gone(&'static str),
    /// The labelled ship is disabled or crippled (boardable).
    Disabled(&'static str),
    /// Complete a boarding action against the labelled ship with this objective.
    Board {
        obj: Objective,
        ship: &'static str,
    },
    /// Every `Hauler…` ship is within `range` of `dest`.
    Escort {
        dest: &'static str,
        range: f32,
    },
}

#[derive(Debug, Clone, Copy)]
pub struct Stage {
    /// Objective line shown on the HUD.
    pub text: &'static str,
    pub goal: Goal,
    pub beats: &'static [Beat],
    /// Failing condition: a ship whose label starts with the prefix is destroyed.
    pub protect: Option<(&'static str, &'static str)>,
    /// Seconds before the stage fails (`0` = no limit).
    pub limit: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct Mission {
    pub stages: &'static [Stage],
}

const NONE: &[Beat] = &[];

const fn say(at: f32, who: &'static str, text: &'static str) -> Beat {
    Beat {
        at,
        act: Act::Say(who, text),
    }
}

const fn spawn(
    at: f32,
    anchor: Anchor,
    off: [f32; 3],
    role: Role,
    classes: &'static [&'static str],
    label: Option<&'static str>,
) -> Beat {
    Beat {
        at,
        act: Act::Spawn(Spawn {
            anchor,
            off,
            role,
            classes,
            label,
        }),
    }
}

const fn stage(text: &'static str, goal: Goal, beats: &'static [Beat]) -> Stage {
    Stage {
        text,
        goal,
        beats,
        protect: None,
        limit: 0.0,
    }
}

const fn guarded(mut s: Stage, prefix: &'static str, msg: &'static str) -> Stage {
    s.protect = Some((prefix, msg));
    s
}

const fn timed(mut s: Stage, limit: f32) -> Stage {
    s.limit = limit;
    s
}

const INGRID: &str = "Jarl Ingrid";
const SIGRUN: &str = "Sigrun";
const BJORN: &str = "Bjorn";
const ASTRID: &str = "Astrid";
const SKALD: &str = "Skald Eirik";
use Anchor::{Body, Player};

static M1: [Stage; 6] = [
    stage(
        "RAISE THE THROTTLE - HOLD W (SHIFT = FULL)",
        Goal::Throttle(0.4),
        &[say(
            0.5,
            INGRID,
            "The Huginn is yours, Captain. Hold W to raise the throttle.",
        )],
    ),
    stage(
        "FLY TO THE FORGE - A/D YAW, ARROWS PITCH, SPACE BRAKES",
        Goal::Reach {
            body: "Forge",
            range: 700.0,
        },
        &[say(
            0.0,
            SIGRUN,
            "The Forge lies ahead. A and D turn, arrows pitch, Q and E roll.",
        )],
    ),
    stage(
        "PRESS T TO TARGET A DRONE",
        Goal::Target,
        &[
            spawn(
                0.0,
                Player,
                [0.0, 0.0, -1100.0],
                Role::Drone,
                &["interceptor", "interceptor", "interceptor"],
                Some("Target Drone"),
            ),
            say(
                0.5,
                BJORN,
                "Target drones off the bow. T cycles targets; brackets show range.",
            ),
        ],
    ),
    stage(
        "HOLD ENTER TO FIRE THE LANCES - DESTROY THE DRONES",
        Goal::Clear,
        &[say(
            0.0,
            BJORN,
            "Lances run hot. Fire in bursts and watch the heat bar.",
        )],
    ),
    stage(
        "DOCK AT THE FORGE - TARGET IT (T), PRESS L",
        Goal::DockAt("Forge"),
        &[say(
            0.0,
            INGRID,
            "Well flown. Dock at the Forge to rearm: get close, target it, press L.",
        )],
    ),
    stage(
        "SCOUTS INBOUND - PRESS L TO UNDOCK AND DESTROY THEM",
        Goal::Clear,
        &[
            spawn(
                1.0,
                Body("Forge"),
                [0.0, 100.0, -1600.0],
                Role::Attack,
                &["interceptor", "interceptor"],
                None,
            ),
            say(
                1.0,
                SKALD,
                "Scouts jumping in! The Concord never waits for the saga.",
            ),
        ],
    ),
];

static M2: [Stage; 3] = [
    stage(
        "FLY TO PHOBOS RELAY",
        Goal::Reach {
            body: "Phobos Relay",
            range: 1200.0,
        },
        &[say(
            0.0,
            INGRID,
            "Raiders are hitting the Phobos relay. Go, and hold it.",
        )],
    ),
    stage(
        "WAVE 1 - DESTROY THE RAIDERS",
        Goal::Clear,
        &[
            spawn(
                0.0,
                Body("Phobos Relay"),
                [-3000.0, 0.0, -3000.0],
                Role::Attack,
                &["interceptor", "interceptor", "cutter"],
                None,
            ),
            say(
                0.5,
                ASTRID,
                "The long scan shows raiders, low and fast. The omens are bad.",
            ),
            say(
                10.0,
                BJORN,
                "F3, then 1/2/3 for power presets. Shields first, Captain!",
            ),
        ],
    ),
    stage(
        "WAVE 2 - HOLD THE RELAY",
        Goal::Clear,
        &[
            spawn(
                0.0,
                Body("Phobos Relay"),
                [3500.0, 500.0, -2500.0],
                Role::Attack,
                &["cutter", "interceptor", "interceptor", "frigate"],
                None,
            ),
            say(
                0.5,
                SKALD,
                "A second wing! They bring a frigate. Good, I have verses.",
            ),
        ],
    ),
];

static M3: [Stage; 4] = [
    stage(
        "MINE 6 ORE - TARGET A BIG ROCK (T), PRESS K",
        Goal::Cargo {
            commodity: 0,
            qty: 6,
        },
        &[
            say(
                0.0,
                INGRID,
                "The Belt miners pay their Tithe in ore. Load the hold first.",
            ),
            say(
                12.0,
                SIGRUN,
                "The mining bot works when you are close and slow. Target a rock, press K.",
            ),
        ],
    ),
    stage(
        "SCAN 2 CONTACTS - T TARGET, N SCAN",
        Goal::Scan(2),
        &[say(
            0.0,
            ASTRID,
            "Scan the rocks and anything that moves. Ambushers hide among the stones.",
        )],
    ),
    guarded(
        stage(
            "ESCORT THE HAULERS TO BELT OUTPOST",
            Goal::Escort {
                dest: "Belt Outpost",
                range: 800.0,
            },
            &[
                spawn(
                    0.0,
                    Body("Belt Outpost"),
                    [2600.0, 0.0, 2600.0],
                    Role::Hauler,
                    &["hauler"],
                    Some("Hauler Ymir"),
                ),
                spawn(
                    0.0,
                    Body("Belt Outpost"),
                    [2800.0, 150.0, 2400.0],
                    Role::Hauler,
                    &["hauler"],
                    Some("Hauler Skuld"),
                ),
                say(0.5, INGRID, "Two haulers, one road. Keep them breathing."),
                spawn(
                    15.0,
                    Body("Belt Outpost"),
                    [-2500.0, 200.0, -1500.0],
                    Role::Attack,
                    &["cutter", "cutter", "interceptor"],
                    None,
                ),
                say(15.0, SKALD, "Ambush! Cutters on the haulers' flank!"),
            ],
        ),
        "Hauler",
        "A HAULER WAS DESTROYED - THE TITHE IS LOST",
    ),
    stage("DESTROY THE REMAINING AMBUSHERS", Goal::Clear, NONE),
];

static M4: [Stage; 2] = [
    guarded(
        stage(
            "CRIPPLE THE COURIER - SHIELDS DOWN, HULL UNDER 60%. DO NOT DESTROY IT",
            Goal::Disabled("Courier"),
            &[
                spawn(
                    0.0,
                    Player,
                    [-1500.0, 100.0, -3500.0],
                    Role::Attack,
                    &["cutter"],
                    Some("Courier"),
                ),
                spawn(
                    0.0,
                    Player,
                    [-1300.0, -100.0, -3800.0],
                    Role::Attack,
                    &["interceptor", "interceptor"],
                    None,
                ),
                say(
                    0.0,
                    INGRID,
                    "A Concord courier carries the Codebook. Break her, do not sink her.",
                ),
                say(
                    8.0,
                    BJORN,
                    "Thunder torpedoes on Z hit hard once her shields are down.",
                ),
            ],
        ),
        "Courier",
        "THE COURIER WAS DESTROYED - THE CODEBOOK IS LOST",
    ),
    guarded(
        stage(
            "TARGET THE COURIER (T), CLOSE IN AND PRESS B TO BOARD",
            Goal::Board {
                obj: Objective::StealData,
                ship: "Courier",
            },
            &[say(
                0.0,
                SIGRUN,
                "She is drifting. Close to the clamp range and press B, axe ready.",
            )],
        ),
        "Courier",
        "THE COURIER WAS DESTROYED - THE CODEBOOK IS LOST",
    ),
];

static M5: [Stage; 5] = [
    stage(
        "FLY TO DEIMOS AND GO DARK",
        Goal::Reach {
            body: "Deimos",
            range: 1500.0,
        },
        &[say(
            0.0,
            INGRID,
            "A Concord convoy carries our people in chains. Wait in Deimos' shadow.",
        )],
    ),
    stage(
        "HOLD POSITION - WAIT FOR THE CONVOY",
        Goal::Wait(26.0),
        &[
            say(
                2.0,
                ASTRID,
                "The ravens say they come soon. Keep still, keep cold.",
            ),
            say(
                14.0,
                SIGRUN,
                "Contacts on the sweep. The convoy approaches.",
            ),
            spawn(
                18.0,
                Body("Deimos"),
                [2800.0, 0.0, -2800.0],
                Role::Patrol,
                &["cutter"],
                Some("Prison Barge"),
            ),
            spawn(
                18.0,
                Body("Deimos"),
                [3100.0, 150.0, -2600.0],
                Role::Attack,
                &["interceptor", "interceptor", "cutter"],
                None,
            ),
        ],
    ),
    guarded(
        stage(
            "DESTROY THE ESCORTS - DO NOT SINK THE PRISON BARGE",
            Goal::ClearBut("Prison Barge"),
            &[say(
                0.0,
                SKALD,
                "Gungnir missiles on V, lock first. Keep clear of the barge!",
            )],
        ),
        "Prison Barge",
        "THE PRISON BARGE WAS DESTROYED - THE PRISONERS ARE LOST",
    ),
    guarded(
        stage(
            "CRIPPLE THE PRISON BARGE",
            Goal::Disabled("Prison Barge"),
            NONE,
        ),
        "Prison Barge",
        "THE PRISON BARGE WAS DESTROYED - THE PRISONERS ARE LOST",
    ),
    guarded(
        stage(
            "TARGET THE BARGE, PRESS B AND FREE THE PRISONERS",
            Goal::Board {
                obj: Objective::Rescue,
                ship: "Prison Barge",
            },
            &[say(
                0.0,
                SIGRUN,
                "Cells are aft. Take the keys, open them all.",
            )],
        ),
        "Prison Barge",
        "THE PRISON BARGE WAS DESTROYED - THE PRISONERS ARE LOST",
    ),
];

static M6: [Stage; 4] = [
    stage(
        "APPROACH GANYMEDE PORT",
        Goal::Reach {
            body: "Ganymede Port",
            range: 1000.0,
        },
        &[say(
            0.0,
            INGRID,
            "Jotun Yards sells hulls on Ganymede. The Concord taxes the road to it.",
        )],
    ),
    stage(
        "DOCK AT GANYMEDE PORT - TARGET IT (T), PRESS L",
        Goal::DockAt("Ganymede Port"),
        NONE,
    ),
    stage(
        "BUY A SHIPYARD UPGRADE (SHIPYARD TAB)",
        Goal::Upgrade,
        &[say(
            0.0,
            BJORN,
            "Lances, shields, reactor, hull, bays. I would take the reactor.",
        )],
    ),
    stage(
        "TOLL SQUAD - PRESS L TO UNDOCK AND BREAK THE BLOCKADE",
        Goal::Clear,
        &[
            spawn(
                1.0,
                Body("Ganymede Port"),
                [-3000.0, 0.0, -2500.0],
                Role::Attack,
                &["frigate", "cutter", "cutter"],
                None,
            ),
            spawn(
                1.0,
                Body("Ganymede Port"),
                [3000.0, 500.0, -1500.0],
                Role::Attack,
                &["interceptor", "interceptor", "cutter"],
                None,
            ),
            say(
                1.0,
                "Concord Toll",
                "Vessel Huginn: the toll is 2000 credits, or you will be boarded.",
            ),
            say(5.0, SKALD, "I vote we pay in lance fire."),
        ],
    ),
];

static M7: [Stage; 3] = [
    stage(
        "FORM UP - YOUR WINGMEN ARE JOINING",
        Goal::Wait(8.0),
        &[
            spawn(
                0.0,
                Player,
                [-350.0, 50.0, 100.0],
                Role::Ally,
                &["skiff"],
                Some("Skiff Geir"),
            ),
            spawn(
                0.0,
                Player,
                [350.0, -40.0, 100.0],
                Role::Ally,
                &["skiff"],
                Some("Skiff Hild"),
            ),
            say(
                0.5,
                INGRID,
                "Three skiffs ride with you. F6 Comms: order them to attack, guard or hold.",
            ),
        ],
    ),
    stage(
        "WOLF PACK - DESTROY THE CONCORD PATROL",
        Goal::Clear,
        &[
            spawn(
                0.0,
                Player,
                [0.0, 0.0, -4500.0],
                Role::Attack,
                &["frigate", "cutter", "cutter", "interceptor"],
                None,
            ),
            say(
                0.5,
                SIGRUN,
                "Patrol dead ahead. Give the wolves their orders.",
            ),
        ],
    ),
    stage(
        "SECOND WAVE - DESTROY THE DESTROYER",
        Goal::Clear,
        &[
            spawn(
                0.0,
                Player,
                [3500.0, 0.0, -5000.0],
                Role::Attack,
                &["destroyer", "interceptor", "interceptor"],
                None,
            ),
            say(0.5, SKALD, "A destroyer! Skiffs, focus fire!"),
        ],
    ),
];

static M8: [Stage; 3] = [
    guarded(
        timed(
            stage(
                "DESTROY THE 3 SHIELD GENERATORS BEFORE THE ARMADA ARRIVES",
                Goal::Gone("Shield Generator"),
                &[
                    spawn(
                        0.0,
                        Body("Tycho Fort"),
                        [2500.0, 0.0, -2500.0],
                        Role::Drone,
                        &["frigate", "frigate", "frigate"],
                        Some("Shield Generator"),
                    ),
                    spawn(
                        0.0,
                        Body("Tycho Fort"),
                        [-2500.0, 0.0, -1500.0],
                        Role::Attack,
                        &["cutter", "cutter", "interceptor"],
                        None,
                    ),
                    say(
                        0.0,
                        INGRID,
                        "The Gate of Frost: three generators hold its shield. Break them fast.",
                    ),
                ],
            ),
            420.0,
        ),
        "Huginn",
        "THE HUGINN WAS LOST",
    ),
    guarded(
        stage(
            "CRIPPLE THE COMMAND SHIP FROST COMMAND",
            Goal::Disabled("Frost Command"),
            &[
                spawn(
                    0.0,
                    Body("Tycho Fort"),
                    [0.0, 500.0, -3000.0],
                    Role::Patrol,
                    &["destroyer"],
                    Some("Frost Command"),
                ),
                say(0.5, ASTRID, "Her reactor sings a song of ending."),
            ],
        ),
        "Frost Command",
        "FROST COMMAND WAS DESTROYED - NOTHING LEFT TO SABOTAGE",
    ),
    guarded(
        stage(
            "BOARD FROST COMMAND (B) AND SABOTAGE THE REACTOR",
            Goal::Board {
                obj: Objective::Sabotage,
                ship: "Frost Command",
            },
            &[say(
                0.0,
                BJORN,
                "Put the reactor to sleep. Gently. Then run.",
            )],
        ),
        "Frost Command",
        "FROST COMMAND WAS DESTROYED - NOTHING LEFT TO SABOTAGE",
    ),
];

static M9: [Stage; 3] = [
    stage(
        "BREAK THE FORTRESS RING",
        Goal::Clear,
        &[
            spawn(
                0.0,
                Player,
                [0.0, 0.0, -4000.0],
                Role::Attack,
                &["frigate", "frigate", "cutter"],
                None,
            ),
            spawn(
                0.0,
                Player,
                [-3500.0, 300.0, -2500.0],
                Role::Attack,
                &["destroyer", "cutter", "interceptor"],
                None,
            ),
            say(
                0.0,
                INGRID,
                "Luna's ring stands between us and Earth. Break it.",
            ),
        ],
    ),
    guarded(
        stage(
            "CRIPPLE THE DREADNOUGHT BULWARK",
            Goal::Disabled("Bulwark"),
            &[
                spawn(
                    0.0,
                    Player,
                    [1500.0, 200.0, -5000.0],
                    Role::Patrol,
                    &["dreadnought"],
                    Some("Bulwark"),
                ),
                spawn(
                    0.0,
                    Player,
                    [1200.0, -200.0, -4500.0],
                    Role::Attack,
                    &["destroyer", "frigate"],
                    None,
                ),
                say(
                    0.5,
                    "Admiral Halden",
                    "A rumour with a rust problem. The Bulwark will end you, Captain.",
                ),
            ],
        ),
        "Bulwark",
        "THE BULWARK WAS DESTROYED - THE ADMIRAL ESCAPED THE BOARDING",
    ),
    guarded(
        stage(
            "BOARD THE BULWARK (B) AND SABOTAGE HER REACTOR",
            Goal::Board {
                obj: Objective::Sabotage,
                ship: "Bulwark",
            },
            &[say(
                0.0,
                BJORN,
                "A dreadnought reactor. I almost feel sorry for it.",
            )],
        ),
        "Bulwark",
        "THE BULWARK WAS DESTROYED - THE ADMIRAL ESCAPED THE BOARDING",
    ),
];

static M10: [Stage; 3] = [
    stage(
        "BREAK THE ARMADA'S SCREEN",
        Goal::Clear,
        &[
            spawn(
                0.0,
                Player,
                [0.0, 0.0, -4000.0],
                Role::Attack,
                &["destroyer", "cutter", "cutter"],
                None,
            ),
            // the second squadron arrives later so the opening is not an alpha strike (T30)
            spawn(
                45.0,
                Player,
                [3000.0, 300.0, -3000.0],
                Role::Attack,
                &["frigate", "frigate", "interceptor", "interceptor"],
                None,
            ),
            say(
                0.0,
                "Prefect Voss",
                "Captain Rurik. Lay down your axe. I am saving everyone, you included.",
            ),
        ],
    ),
    guarded(
        stage(
            "CRIPPLE THE FLAGSHIP AEGIS PRIME",
            Goal::Disabled("Aegis Prime"),
            &[
                spawn(
                    0.0,
                    Player,
                    [-1500.0, 200.0, -6000.0],
                    Role::Patrol,
                    &["dreadnought"],
                    Some("Aegis Prime"),
                ),
                spawn(
                    0.0,
                    Player,
                    [-1200.0, -100.0, -5500.0],
                    Role::Attack,
                    &["destroyer", "destroyer"],
                    None,
                ),
                say(
                    0.5,
                    INGRID,
                    "That is Aegis Prime. Everything we are rides on her.",
                ),
            ],
        ),
        "Aegis Prime",
        "AEGIS PRIME WAS DESTROYED - THE FLEET CODES BURNED WITH HER",
    ),
    guarded(
        stage(
            "BOARD AEGIS PRIME (B) AND KILL PREFECT VOSS",
            Goal::Board {
                obj: Objective::KillBoss,
                ship: "Aegis Prime",
            },
            &[say(
                0.0,
                ASTRID,
                "The ravens circle the bridge. Go, and finish the saga.",
            )],
        ),
        "Aegis Prime",
        "AEGIS PRIME WAS DESTROYED - THE FLEET CODES BURNED WITH HER",
    ),
];

static MISSIONS: [Mission; 10] = [
    Mission { stages: &M1 },
    Mission { stages: &M2 },
    Mission { stages: &M3 },
    Mission { stages: &M4 },
    Mission { stages: &M5 },
    Mission { stages: &M6 },
    Mission { stages: &M7 },
    Mission { stages: &M8 },
    Mission { stages: &M9 },
    Mission { stages: &M10 },
];

/// Definition of mission `id` (1-based): the campaign `1..=10`, then registered scripts.
pub fn def(id: u8) -> Option<&'static Mission> {
    if id > MISSIONS.len() as u8 {
        return super::script::get(id).map(|s| s.mission);
    }
    MISSIONS.get(usize::from(id).checked_sub(1)?)
}

/// A line of dialogue (or a system line when `who` is empty) for the HUD.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Note {
    pub who: &'static str,
    pub text: &'static str,
}

/// What the player's ship carries from one mission into the next.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Carry {
    pub credits: i32,
    pub up: [u8; 5],
}

impl Default for Carry {
    fn default() -> Carry {
        Carry {
            credits: 500,
            up: [1; 5],
        }
    }
}

impl Carry {
    pub fn from_world(w: &World) -> Option<Carry> {
        let o = w.player.and_then(|p| w.get(p))?.ops.as_deref()?;
        Some(Carry {
            credits: o.cargo.credits,
            up: [o.up.lance, o.up.shield, o.up.reactor, o.up.hull, o.up.bays],
        })
    }

    /// Put the carried credits and upgrades on the player's ship.
    pub fn apply(&self, w: &mut World) {
        let Some(o) = w
            .player
            .and_then(|p| w.get_mut(p))
            .and_then(|e| e.ops.as_deref_mut())
        else {
            return;
        };
        o.cargo.credits = self.credits;
        o.up = Upgrades {
            lance: self.up[0].clamp(1, 3),
            shield: self.up[1].clamp(1, 3),
            reactor: self.up[2].clamp(1, 3),
            hull: self.up[3].clamp(1, 3),
            bays: self.up[4].clamp(1, 3),
        };
        o.recompute();
        o.hull = o.max_hull;
        o.shields = [o.max_shield; 4];
        o.torpedoes = o.max_torpedoes();
        o.missiles = o.max_missiles();
    }
}

fn upgrade_sum(w: &World) -> u32 {
    w.player
        .and_then(|p| w.get(p))
        .and_then(|e| e.ops.as_deref())
        .map_or(0, |o| {
            [o.up.lance, o.up.shield, o.up.reactor, o.up.hull, o.up.bays]
                .iter()
                .map(|&l| u32::from(l))
                .sum()
        })
}

/// Active hostile warships, ignoring labels that start with `except` (when not empty).
fn hostiles(w: &World, except: &str) -> usize {
    w.iter()
        .filter(|(_, e)| {
            e.kind == Kind::Ship
                && e.faction == Faction::Minion
                && (except.is_empty() || !e.label.starts_with(except))
                && e.ops.as_deref().is_some_and(|o| !o.dead && !o.disabled)
        })
        .count()
}

fn find_body(w: &World, label: &str) -> Option<(EntityId, Vec3, f32)> {
    w.iter()
        .find(|(_, e)| e.label == label)
        .map(|(i, e)| (i, e.pos, e.radius))
}

/// Progress of one campaign mission.
#[derive(Debug, Clone)]
pub struct MissionState {
    pub mission: u8,
    stage: usize,
    t0: f32,
    next_beat: usize,
    notes: Vec<Note>,
    result: Option<bool>,
    reason: &'static str,
    boarded: bool,
    up_base: u32,
}

impl MissionState {
    pub fn new(mission: u8) -> MissionState {
        MissionState {
            mission,
            stage: 0,
            t0: 0.0,
            next_beat: 0,
            notes: Vec::new(),
            result: None,
            reason: "",
            boarded: false,
            up_base: 0,
        }
    }

    fn stage_def(&self) -> Option<&'static Stage> {
        def(self.mission).and_then(|m| m.stages.get(self.stage))
    }

    /// Prepare the world: remove the sector's stock hostile ships; the mission spawns its own.
    pub fn begin(&mut self, w: &mut World) {
        let stock: Vec<EntityId> = w
            .iter()
            .filter(|(_, e)| e.kind == Kind::Ship && e.faction == Faction::Minion)
            .map(|(i, _)| i)
            .collect();
        for id in stock {
            w.despawn(id);
        }
        self.up_base = upgrade_sum(w);
        self.t0 = w.time;
    }

    /// Objective line for the HUD, or `None` once the mission is decided.
    pub fn objective(&self) -> Option<&'static str> {
        self.result
            .is_none()
            .then(|| self.stage_def().map(|s| s.text))?
    }

    /// Goal of the current stage (what the playtest bot works toward).
    pub fn goal(&self) -> Option<Goal> {
        self.result
            .is_none()
            .then(|| self.stage_def().map(|s| s.goal))?
    }

    /// `(current, total)` stage numbers, 1-based.
    pub fn progress(&self) -> (usize, usize) {
        let total = def(self.mission).map_or(0, |m| m.stages.len());
        ((self.stage + 1).min(total), total)
    }

    /// `Some(true)` won, `Some(false)` failed, `None` still running.
    pub fn result(&self) -> Option<bool> {
        self.result
    }

    /// Why the mission was lost (empty unless it was).
    pub fn reason(&self) -> &'static str {
        self.reason
    }

    /// Seconds left on the current stage's time limit.
    pub fn time_left(&self, w: &World) -> Option<f32> {
        let s = self.stage_def().filter(|s| s.limit > 0.0)?;
        Some((s.limit - (w.time - self.t0)).max(0.0))
    }

    pub fn take_notes(&mut self) -> Vec<Note> {
        std::mem::take(&mut self.notes)
    }

    /// The boarding objective the mission demands against the ship labelled `label`, if any.
    pub fn board_wanted(&self, label: &str) -> Option<Objective> {
        match self.stage_def()?.goal {
            Goal::Board { obj, ship } if label.starts_with(ship) => Some(obj),
            _ => None,
        }
    }

    /// Report a finished boarding action.
    pub fn on_boarding(&mut self, label: &str, objective: Objective, won: bool) {
        if won && self.board_wanted(label) == Some(objective) {
            self.boarded = true;
        }
    }

    /// Mark the mission lost (the player died, ...).
    pub fn fail(&mut self, reason: &'static str) {
        if self.result.is_none() {
            self.result = Some(false);
            self.reason = reason;
        }
    }

    fn fire(&mut self, w: &mut World, goal: Goal, act: Act) {
        match act {
            Act::Say(who, text) => self.notes.push(Note { who, text }),
            Act::Spawn(s) => do_spawn(w, &s, goal),
        }
    }

    fn goal_met(&self, w: &World, goal: Goal, elapsed: f32, beats_done: bool) -> bool {
        let me = w.player.and_then(|p| w.get(p));
        let ops = me.and_then(|e| e.ops.as_deref());
        match goal {
            Goal::Throttle(f) => me.and_then(|e| e.ship).is_some_and(|s| s.throttle >= f),
            Goal::Target => ops.is_some_and(|o| o.target.is_some()),
            Goal::Reach { body, range } => {
                let (Some(me), Some((_, p, r))) = (me, find_body(w, body)) else {
                    return false;
                };
                me.pos.distance(p) - r <= range
            }
            Goal::DockAt(name) => ops
                .and_then(|o| o.docked)
                .and_then(|d| w.get(d.0))
                .is_some_and(|s| s.label == name),
            Goal::Scan(n) => ops.is_some_and(|o| o.scan.done.len() as u32 >= n),
            Goal::Cargo { commodity, qty } => {
                ops.is_some_and(|o| o.cargo.hold.get(commodity).copied().unwrap_or(0) >= qty)
            }
            Goal::Upgrade => upgrade_sum(w) > self.up_base,
            Goal::Wait(s) => elapsed >= s,
            Goal::Clear => beats_done && hostiles(w, "") == 0,
            Goal::ClearBut(p) => beats_done && hostiles(w, p) == 0,
            Goal::Gone(p) => beats_done && !w.iter().any(|(_, e)| e.label.starts_with(p)),
            Goal::Disabled(p) => {
                beats_done
                    && w.iter().any(|(_, e)| {
                        e.label.starts_with(p)
                            && e.ops.as_deref().is_some_and(|o| o.disabled || o.crippled())
                    })
            }
            Goal::Board { .. } => self.boarded,
            Goal::Escort { dest, range } => {
                let Some((_, dp, dr)) = find_body(w, dest) else {
                    return false;
                };
                let haulers: Vec<_> = w
                    .iter()
                    .filter(|(_, e)| e.kind == Kind::Ship && e.label.starts_with("Hauler"))
                    .collect();
                beats_done
                    && !haulers.is_empty()
                    && haulers
                        .iter()
                        .all(|(_, e)| e.pos.distance(dp) - dr <= range)
            }
        }
    }

    /// Advance the mission after a sim step: events (losses), timed beats and goals.
    pub fn update(&mut self, w: &mut World, events: &[Event]) {
        if self.result.is_some() {
            return;
        }
        let Some(stage) = self.stage_def() else {
            self.result = Some(true);
            return;
        };
        if let Some((prefix, msg)) = stage.protect {
            for e in events {
                if let Event::Destroyed { label, .. } = e
                    && label.starts_with(prefix)
                {
                    self.fail(msg);
                    return;
                }
            }
        }
        let elapsed = w.time - self.t0;
        while let Some(b) = stage.beats.get(self.next_beat)
            && elapsed >= b.at
        {
            self.next_beat += 1;
            self.fire(w, stage.goal, b.act);
        }
        if stage.limit > 0.0 && elapsed > stage.limit {
            self.fail("TIME RAN OUT");
            return;
        }
        if !w.tick.is_multiple_of(6) {
            return;
        }
        let beats_done = self.next_beat >= stage.beats.len();
        if self.goal_met(w, stage.goal, elapsed, beats_done) {
            self.stage += 1;
            self.next_beat = 0;
            self.t0 = w.time;
            self.boarded = false;
            if self.stage_def().is_none() {
                self.result = Some(true);
            } else {
                self.notes.push(Note {
                    who: "",
                    text: "OBJECTIVE COMPLETE",
                });
            }
        }
    }
}

fn do_spawn(w: &mut World, s: &Spawn, goal: Goal) {
    let origin = match s.anchor {
        Anchor::Player => w.player.and_then(|p| w.get(p)).map(|e| (e.pos, e.rot)),
        Anchor::Body(name) => find_body(w, name).map(|(_, p, _)| (p, glam::Quat::IDENTITY)),
    };
    let Some((pos, rot)) = origin else { return };
    let base = pos + rot * Vec3::from(s.off);
    for (i, class_name) in s.classes.iter().enumerate() {
        let a = i as f32 * 2.1;
        let at = base + Vec3::new(a.cos() * 220.0, (i as f32 - 1.0) * 40.0, a.sin() * 220.0);
        let id = match s.role {
            Role::Attack | Role::Patrol | Role::Drone => {
                let mode = if s.role == Role::Attack {
                    Mode::Attack
                } else {
                    Mode::Patrol
                };
                ai::spawn_minion(w, ai::class_of(class_name), at, mode)
            }
            Role::Ally => {
                let id = ship(w, "Skiff", SKIFF, at, 0.0);
                if let Some(e) = w.get_mut(id) {
                    e.faction = Faction::Mars;
                    e.ai = Some(Ai::wingman(0.9));
                    if let Some(o) = e.ops.as_deref_mut() {
                        o.sys.preset(2);
                    }
                }
                id
            }
            Role::Hauler => {
                let id = ship(w, "Hauler", HAULER, at, 0.0);
                let dest = match goal {
                    Goal::Escort { dest, .. } => find_body(w, dest).map(|b| b.0),
                    _ => None,
                };
                if let Some(e) = w.get_mut(id) {
                    e.faction = Faction::Mars;
                    if let (Some(o), Some(d)) = (e.ops.as_deref_mut(), dest) {
                        o.nav.dest = Dest::Body(d);
                        o.nav.autopilot = true;
                    }
                }
                id
            }
        };
        if let Some(e) = w.get_mut(id) {
            if let Some(l) = s.label {
                e.label = l;
            }
            if s.role == Role::Drone {
                e.ai = None;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::sector::{SectorId, new_game};

    fn started(mission: u8) -> (World, MissionState) {
        let mut w = new_game(7, SectorId::for_mission(mission));
        let mut m = MissionState::new(mission);
        m.begin(&mut w);
        (w, m)
    }

    fn run(w: &mut World, m: &mut MissionState, steps: u32) {
        for _ in 0..steps {
            let ev = w.step(&[]);
            m.update(w, &ev);
        }
    }

    #[test]
    fn every_mission_is_defined_and_references_real_bodies() {
        for id in 1..=10u8 {
            let mission = def(id).expect("mission defined");
            assert!(!mission.stages.is_empty());
            let w = new_game(1, SectorId::for_mission(id));
            let has = |l: &str| w.iter().any(|(_, e)| e.label == l);
            for s in mission.stages {
                assert!(s.text.len() < 80, "mission {id}: objective too long");
                match s.goal {
                    Goal::Reach { body, .. } => assert!(has(body), "{id}: {body}"),
                    Goal::DockAt(n) | Goal::Escort { dest: n, .. } => {
                        assert!(has(n), "{id}: {n}")
                    }
                    _ => {}
                }
                for b in s.beats {
                    match b.act {
                        Act::Spawn(sp) => {
                            if let Anchor::Body(n) = sp.anchor {
                                assert!(has(n), "{id}: {n}");
                            }
                        }
                        Act::Say(_, t) => assert!(t.len() < 100, "{id}: caption too long"),
                    }
                }
            }
        }
        assert!(def(0).is_none() && def(11).is_none());
    }

    #[test]
    fn begin_removes_stock_hostiles() {
        let (w, _) = started(2);
        assert_eq!(hostiles(&w, ""), 0);
    }

    #[test]
    fn tutorial_advances_on_throttle_and_target() {
        let (mut w, mut m) = started(1);
        run(&mut w, &mut m, 12);
        assert_eq!(m.progress().0, 1);
        let me = w.player.expect("player");
        if let Some(s) = w.get_mut(me).and_then(|e| e.ship.as_mut()) {
            s.throttle = 0.6;
        }
        run(&mut w, &mut m, 12);
        assert_eq!(m.progress().0, 2);
        assert!(
            m.take_notes()
                .iter()
                .any(|n| n.text == "OBJECTIVE COMPLETE")
        );
    }

    #[test]
    fn clear_waits_for_spawns_then_completes_when_they_die() {
        let (mut w, mut m) = started(2);
        m.stage = 1;
        m.t0 = w.time;
        run(&mut w, &mut m, 12);
        assert!(hostiles(&w, "") > 0, "wave spawned");
        assert_eq!(m.progress().0, 2);
        let ids: Vec<_> = w
            .iter()
            .filter(|(_, e)| e.faction == Faction::Minion && e.kind == Kind::Ship)
            .map(|(i, _)| i)
            .collect();
        for id in ids {
            w.despawn(id);
        }
        run(&mut w, &mut m, 12);
        assert_eq!(m.progress().0, 2, "late dialogue beat still pending");
        w.time += 11.0;
        run(&mut w, &mut m, 12);
        assert_eq!(m.progress().0, 3);
    }

    #[test]
    fn losing_the_protected_ship_fails_the_mission() {
        let (mut w, mut m) = started(4);
        run(&mut w, &mut m, 12);
        let ev = [Event::Destroyed {
            id: EntityId {
                index: 0,
                generation: 0,
            },
            pos: Vec3::ZERO,
            radius: 1.0,
            label: "Courier",
            by: None,
        }];
        m.update(&mut w, &ev);
        assert_eq!(m.result(), Some(false));
        assert!(m.reason().contains("CODEBOOK"));
    }

    #[test]
    fn boarding_stage_needs_the_right_objective() {
        let (mut w, mut m) = started(4);
        m.stage = 1;
        assert_eq!(m.board_wanted("Courier"), Some(Objective::StealData));
        assert_eq!(m.board_wanted("Cutter"), None);
        m.on_boarding("Courier", Objective::Sabotage, true);
        run(&mut w, &mut m, 12);
        assert_eq!(m.result(), None);
        m.on_boarding("Courier", Objective::StealData, true);
        run(&mut w, &mut m, 12);
        assert_eq!(m.result(), Some(true));
    }

    #[test]
    fn time_limit_fails_the_stage() {
        let (mut w, mut m) = started(8);
        assert!(m.time_left(&w).is_some_and(|t| t > 400.0));
        w.time += 500.0;
        run(&mut w, &mut m, 2);
        assert_eq!(m.result(), Some(false));
    }

    #[test]
    fn haulers_fly_to_the_outpost() {
        let (mut w, mut m) = started(3);
        m.stage = 2;
        m.t0 = w.time;
        run(&mut w, &mut m, 12);
        let haulers = w
            .iter()
            .filter(|(_, e)| e.label.starts_with("Hauler"))
            .count();
        assert_eq!(haulers, 2);
        assert!(
            w.iter()
                .filter(|(_, e)| e.label.starts_with("Hauler"))
                .all(|(_, e)| e.ops.as_deref().is_some_and(|o| o.nav.autopilot))
        );
    }

    #[test]
    fn carry_round_trips_through_the_world() {
        let mut w = new_game(1, SectorId::MarsOrbit);
        let c = Carry {
            credits: 1234,
            up: [2, 1, 3, 1, 2],
        };
        c.apply(&mut w);
        assert_eq!(Carry::from_world(&w), Some(c));
    }
}
