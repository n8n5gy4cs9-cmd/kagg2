//! Mission scripting (T34): a small line-based text format (`.k2m`) that compiles to the same
//! `Mission` data the campaign uses, plus a registry so mods can add missions at run time.
//!
//! ```text
//! # comments start with '#'; strings are "double quoted"
//! mission "Wolf Pack"            # optional title
//! sector belt                    # mars phobos belt jupiter luna earth
//! stage "FLY TO THE OUTPOST" reach "Belt Outpost" 900
//!   say 1 "Sigrun" "Keep your eyes open."
//!   limit 300                    # seconds before the stage fails
//! stage "DESTROY THE WOLVES" clear
//!   spawn 2 player 0 0 -3000 attack interceptor,cutter label "Wolf"
//!   protect "Hauler" "THE HAULER WAS LOST"
//! ```
//!
//! Goals: `throttle F`, `target`, `reach BODY RANGE`, `dock BODY`, `scan N`, `cargo COMMODITY QTY`,
//! `upgrade`, `wait SECS`, `clear`, `clear_but PREFIX`, `gone PREFIX`, `disabled PREFIX`,
//! `board OBJECTIVE PREFIX` (`sabotage capture rescue data boss`), `escort BODY RANGE`.
//! `spawn AT ANCHOR X Y Z ROLE CLASSES [label "L"]`: anchor `player` or `body "NAME"`, role
//! `attack patrol drone ally hauler`, classes a comma list of `interceptor cutter frigate
//! destroyer dreadnought`. `say AT "WHO" "TEXT"`.
//!
//! Compiled scripts are leaked (`&'static`) once at load time, which keeps the mission engine's
//! data types unchanged; scripts are small and loaded at start-up, so the leak is bounded.

use std::sync::Mutex;

use super::mission::{Act, Anchor, Beat, Goal, Mission, Role, Spawn, Stage};
use super::sector::{SectorId, new_game};
use crate::board::Objective;

/// First mission id handed out to scripts (the campaign owns `1..=10`).
pub const FIRST_SCRIPT_ID: u8 = 11;

/// A parse or validation problem, with the 1-based line number when it has one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptError {
    pub line: usize,
    pub msg: String,
}

impl std::fmt::Display for ScriptError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.line == 0 {
            write!(f, "{}", self.msg)
        } else {
            write!(f, "line {}: {}", self.line, self.msg)
        }
    }
}

impl std::error::Error for ScriptError {}

fn err<T>(line: usize, msg: impl Into<String>) -> Result<T, ScriptError> {
    Err(ScriptError {
        line,
        msg: msg.into(),
    })
}

/// A compiled script: where it is flown and the mission it plays.
#[derive(Debug, Clone, Copy)]
pub struct Script {
    pub title: &'static str,
    pub sector: SectorId,
    pub mission: &'static Mission,
}

static REGISTRY: Mutex<Vec<(u8, Script)>> = Mutex::new(Vec::new());

/// Split a line into tokens; `"quoted strings"` keep their spaces. `#` starts a comment.
fn tokens(line: usize, text: &str) -> Result<Vec<String>, ScriptError> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let (mut quoted, mut had) = (false, false);
    for c in text.chars() {
        match c {
            '"' => {
                quoted = !quoted;
                had = true;
            }
            '#' if !quoted => break,
            c if c.is_whitespace() && !quoted => {
                if had || !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                    had = false;
                }
            }
            c => cur.push(c),
        }
    }
    if quoted {
        return err(line, "unterminated string");
    }
    if had || !cur.is_empty() {
        out.push(cur);
    }
    Ok(out)
}

fn leak(s: &str) -> &'static str {
    Box::leak(s.to_owned().into_boxed_str())
}

fn num<T: std::str::FromStr>(
    line: usize,
    tok: Option<&String>,
    what: &str,
) -> Result<T, ScriptError> {
    match tok.map(|t| t.parse::<T>()) {
        Some(Ok(v)) => Ok(v),
        _ => err(line, format!("expected a number for {what}")),
    }
}

fn word<'a>(line: usize, tok: Option<&'a String>, what: &str) -> Result<&'a str, ScriptError> {
    match tok {
        Some(t) => Ok(t.as_str()),
        None => err(line, format!("missing {what}")),
    }
}

const CLASSES: [&str; 5] = [
    "interceptor",
    "cutter",
    "frigate",
    "destroyer",
    "dreadnought",
];

fn parse_goal(line: usize, t: &[String]) -> Result<Goal, ScriptError> {
    let kind = word(line, t.first(), "goal")?;
    Ok(match kind {
        "throttle" => Goal::Throttle(num(line, t.get(1), "throttle")?),
        "target" => Goal::Target,
        "reach" => Goal::Reach {
            body: leak(word(line, t.get(1), "body name")?),
            range: num(line, t.get(2), "range")?,
        },
        "dock" => Goal::DockAt(leak(word(line, t.get(1), "station name")?)),
        "scan" => Goal::Scan(num(line, t.get(1), "scan count")?),
        "cargo" => Goal::Cargo {
            commodity: num(line, t.get(1), "commodity index")?,
            qty: num(line, t.get(2), "quantity")?,
        },
        "upgrade" => Goal::Upgrade,
        "wait" => Goal::Wait(num(line, t.get(1), "seconds")?),
        "clear" => Goal::Clear,
        "clear_but" => Goal::ClearBut(leak(word(line, t.get(1), "label prefix")?)),
        "gone" => Goal::Gone(leak(word(line, t.get(1), "label prefix")?)),
        "disabled" => Goal::Disabled(leak(word(line, t.get(1), "label prefix")?)),
        "board" => Goal::Board {
            obj: match word(line, t.get(1), "objective")? {
                "sabotage" => Objective::Sabotage,
                "capture" => Objective::CaptureBridge,
                "rescue" => Objective::Rescue,
                "data" => Objective::StealData,
                "boss" => Objective::KillBoss,
                o => return err(line, format!("unknown boarding objective '{o}'")),
            },
            ship: leak(word(line, t.get(2), "ship label prefix")?),
        },
        "escort" => Goal::Escort {
            dest: leak(word(line, t.get(1), "destination body")?),
            range: num(line, t.get(2), "range")?,
        },
        other => return err(line, format!("unknown goal '{other}'")),
    })
}

fn parse_spawn(line: usize, t: &[String]) -> Result<Beat, ScriptError> {
    // spawn AT ANCHOR [NAME] X Y Z ROLE CLASSES [label L]
    let at: f32 = num(line, t.get(1), "spawn time")?;
    let (anchor, mut i) = match word(line, t.get(2), "anchor")? {
        "player" => (Anchor::Player, 3),
        "body" => (Anchor::Body(leak(word(line, t.get(3), "body name")?)), 4),
        a => return err(line, format!("unknown anchor '{a}' (player or body)")),
    };
    let mut off = [0.0; 3];
    for o in &mut off {
        *o = num(line, t.get(i), "offset")?;
        i += 1;
    }
    let role = match word(line, t.get(i), "role")? {
        "attack" => Role::Attack,
        "patrol" => Role::Patrol,
        "drone" => Role::Drone,
        "ally" => Role::Ally,
        "hauler" => Role::Hauler,
        r => return err(line, format!("unknown role '{r}'")),
    };
    let classes: Vec<&'static str> = word(line, t.get(i + 1), "ship classes")?
        .split(',')
        .map(|c| {
            if CLASSES.contains(&c) || matches!(role, Role::Ally | Role::Hauler) {
                Ok(leak(c))
            } else {
                err(line, format!("unknown ship class '{c}'"))
            }
        })
        .collect::<Result<_, _>>()?;
    let label = match t.get(i + 2).map(String::as_str) {
        Some("label") => Some(leak(word(line, t.get(i + 3), "label")?)),
        Some(x) => return err(line, format!("unexpected '{x}'")),
        None => None,
    };
    Ok(Beat {
        at,
        act: Act::Spawn(Spawn {
            anchor,
            off,
            role,
            classes: Box::leak(classes.into_boxed_slice()),
            label,
        }),
    })
}

struct Building {
    text: &'static str,
    goal: Goal,
    beats: Vec<Beat>,
    protect: Option<(&'static str, &'static str)>,
    limit: f32,
}

impl Building {
    fn finish(self) -> Stage {
        let mut beats = self.beats;
        beats.sort_by(|a, b| a.at.total_cmp(&b.at));
        Stage {
            text: self.text,
            goal: self.goal,
            beats: Box::leak(beats.into_boxed_slice()),
            protect: self.protect,
            limit: self.limit,
        }
    }
}

/// Compile script text into a [`Script`] (not registered yet; see [`register`]).
pub fn parse(src: &str) -> Result<Script, ScriptError> {
    let mut title = "Script mission";
    let mut sector = None;
    let mut stages: Vec<Stage> = Vec::new();
    let mut cur: Option<Building> = None;
    for (n, raw) in src.lines().enumerate() {
        let line = n + 1;
        let t = tokens(line, raw)?;
        let Some(head) = t.first().map(String::as_str) else {
            continue;
        };
        match head {
            "mission" => title = leak(word(line, t.get(1), "title")?),
            "sector" => {
                let name = word(line, t.get(1), "sector name")?;
                sector = Some(SectorId::parse(name).ok_or(ScriptError {
                    line,
                    msg: format!("unknown sector '{name}'"),
                })?);
            }
            "stage" => {
                if let Some(b) = cur.take() {
                    stages.push(b.finish());
                }
                let text = word(line, t.get(1), "objective text")?;
                if text.len() >= 80 {
                    return err(line, "objective text must be under 80 characters");
                }
                cur = Some(Building {
                    text: leak(text),
                    goal: parse_goal(line, &t[2..])?,
                    beats: Vec::new(),
                    protect: None,
                    limit: 0.0,
                });
            }
            "say" | "spawn" | "protect" | "limit" => {
                let Some(b) = cur.as_mut() else {
                    return err(line, format!("'{head}' needs a stage above it"));
                };
                match head {
                    "say" => {
                        let text = word(line, t.get(3), "text")?;
                        if text.len() >= 100 {
                            return err(line, "caption must be under 100 characters");
                        }
                        b.beats.push(Beat {
                            at: num(line, t.get(1), "time")?,
                            act: Act::Say(leak(word(line, t.get(2), "speaker")?), leak(text)),
                        });
                    }
                    "spawn" => b.beats.push(parse_spawn(line, &t)?),
                    "protect" => {
                        b.protect = Some((
                            leak(word(line, t.get(1), "label prefix")?),
                            leak(word(line, t.get(2), "failure message")?),
                        ));
                    }
                    _ => b.limit = num(line, t.get(1), "seconds")?,
                }
            }
            other => return err(line, format!("unknown command '{other}'")),
        }
    }
    if let Some(b) = cur.take() {
        stages.push(b.finish());
    }
    if stages.is_empty() {
        return err(0, "a mission needs at least one stage");
    }
    let Some(sector) = sector else {
        return err(0, "missing 'sector NAME'");
    };
    let script = Script {
        title,
        sector,
        mission: Box::leak(Box::new(Mission {
            stages: Box::leak(stages.into_boxed_slice()),
        })),
    };
    validate(&script)?;
    Ok(script)
}

/// Check that every body a script names exists in its sector.
pub fn validate(s: &Script) -> Result<(), ScriptError> {
    let w = new_game(1, s.sector);
    let has = |l: &str| w.iter().any(|(_, e)| e.label == l);
    let need = |l: &str| {
        if has(l) {
            Ok(())
        } else {
            err(0, format!("'{l}' is not a body of {}", s.sector.name()))
        }
    };
    for st in s.mission.stages {
        match st.goal {
            Goal::Reach { body, .. } => need(body)?,
            Goal::DockAt(n) | Goal::Escort { dest: n, .. } => need(n)?,
            _ => {}
        }
        for b in st.beats {
            if let Act::Spawn(sp) = b.act
                && let Anchor::Body(n) = sp.anchor
            {
                need(n)?;
            }
        }
    }
    Ok(())
}

/// Register a compiled script and return its mission id (`11..`), or `None` when full.
pub fn register(s: Script) -> Option<u8> {
    let mut reg = REGISTRY.lock().ok()?;
    let id = FIRST_SCRIPT_ID.checked_add(u8::try_from(reg.len()).ok()?)?;
    reg.push((id, s));
    Some(id)
}

/// The registered script with mission id `id`.
pub fn get(id: u8) -> Option<Script> {
    REGISTRY
        .lock()
        .ok()?
        .iter()
        .find(|(i, _)| *i == id)
        .map(|(_, s)| *s)
}

/// Parse and register a script file.
pub fn load_file(path: &std::path::Path) -> Result<u8, ScriptError> {
    let text = std::fs::read_to_string(path).map_err(|e| ScriptError {
        line: 0,
        msg: format!("{}: {e}", path.display()),
    })?;
    let s = parse(&text)?;
    register(s).ok_or(ScriptError {
        line: 0,
        msg: "too many scripts".into(),
    })
}

/// Load every `*.k2m` file in `dir` (sorted by name); returns one result per file.
pub fn load_dir(dir: &std::path::Path) -> Vec<(std::path::PathBuf, Result<u8, ScriptError>)> {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut files: Vec<_> = rd
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "k2m"))
        .collect();
    files.sort();
    files
        .into_iter()
        .map(|p| {
            let r = load_file(&p);
            (p, r)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
# a tiny mission
mission "Wolf Pack"
sector belt
stage "FLY TO THE OUTPOST" reach "Belt Outpost" 900
  say 1 "Sigrun" "Keep your eyes open, # not a comment."
  limit 300
stage "DESTROY THE WOLVES" clear
  spawn 2 player 0 0 -3000 attack interceptor,cutter label "Wolf"
  protect "Hauler" "THE HAULER WAS LOST"
stage "BOARD THE LEADER" board data Wolf
"#;

    #[test]
    fn parses_stages_beats_and_options() {
        let s = parse(SAMPLE).expect("parses");
        assert_eq!(s.title, "Wolf Pack");
        assert_eq!(s.sector, SectorId::Belt);
        let st = s.mission.stages;
        assert_eq!(st.len(), 3);
        assert!(
            matches!(st[0].goal, Goal::Reach { body: "Belt Outpost", range } if range == 900.0)
        );
        assert_eq!(st[0].limit, 300.0);
        assert!(matches!(st[0].beats[0].act, Act::Say("Sigrun", t) if t.contains("# not")));
        assert!(
            matches!(st[1].beats[0].act, Act::Spawn(sp) if sp.classes == ["interceptor", "cutter"] && sp.label == Some("Wolf"))
        );
        assert_eq!(st[1].protect, Some(("Hauler", "THE HAULER WAS LOST")));
        assert!(matches!(
            st[2].goal,
            Goal::Board {
                obj: Objective::StealData,
                ship: "Wolf"
            }
        ));
    }

    #[test]
    fn errors_name_the_line() {
        let e = parse("sector mars\nstage \"X\" fly\n").expect_err("bad goal");
        assert_eq!(e.line, 2);
        assert!(e.msg.contains("unknown goal"));
        assert!(
            parse("stage \"X\" clear\n")
                .expect_err("no sector")
                .msg
                .contains("sector")
        );
        assert!(
            parse("sector mars\n")
                .expect_err("no stage")
                .msg
                .contains("stage")
        );
        assert!(parse("sector mars\nsay 1 \"a\" \"b\"\n").is_err());
        assert!(parse("sector mars\nstage \"X\" reach Nowhere 5\n").is_err());
        assert!(
            parse("sector mars\nstage \"X\" clear\nspawn 1 player 0 0 0 attack tank\n").is_err()
        );
        assert!(parse("sector mars\nstage \"unterminated clear\n").is_err());
    }

    #[test]
    fn registered_scripts_are_playable_missions() {
        let s = parse(SAMPLE).expect("parses");
        let id = register(s).expect("id");
        assert!(id >= FIRST_SCRIPT_ID);
        assert!(get(id).is_some());
        assert_eq!(SectorId::for_mission(id), SectorId::Belt);
        assert_eq!(
            super::super::mission::def(id).map(|m| m.stages.len()),
            Some(3)
        );
    }
}
