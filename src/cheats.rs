//! Developer console and cheat commands (headless: parsing and line editing only).
//!
//! The console is opened with `` ` `` or `F12` (`gfx::console` draws it, `main.rs` applies the
//! parsed [`Cheat`]s to the running scene). Type `help` for the list.

/// One parsed console command.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Cheat {
    Help,
    /// Toggle (`None`) or set invulnerability (ship hull/shields, boarding health).
    God(Option<bool>),
    /// Refill hull, shields, crew, health and armour.
    Heal,
    /// Refill torpedoes/missiles (space) or all weapons, ammo and keys (boarding).
    Ammo,
    /// Add credits (may be negative).
    Credits(i32),
    /// Destroy every hostile ship (space) or monster (boarding).
    Kill,
    /// End the mission or boarding as a win.
    Win,
    /// End the mission or boarding as a loss.
    Lose,
    /// Jump straight into the space part of a mission (`level`, `mission`).
    Mission(u8),
    /// Jump straight into the boarding action of a mission; `None` = the current mission.
    Board(Option<u8>),
    /// Back to the main menu.
    Menu,
    Quit,
}

/// Lines printed by `help`.
pub const HELP: &[&str] = &[
    "god [on|off]     invulnerable ship / boarding party",
    "heal             full hull, shields, crew, health, armour",
    "ammo             refill torpedoes+missiles / all weapons, ammo, keys",
    "credits N        add N credits",
    "kill             destroy all hostiles (ships or monsters)",
    "win | lose       end the mission or boarding",
    "level N          jump to mission N (1-10), space part",
    "board [N]        jump to the boarding level of mission N",
    "                 (4 courier, 5 barge, 8 Frost Command, 9 Bulwark, 10 Aegis Prime)",
    "menu | quit      leave the game screen / exit",
];

fn on_off(word: Option<&str>) -> Result<Option<bool>, String> {
    match word {
        None | Some("toggle") => Ok(None),
        Some("on" | "1" | "true") => Ok(Some(true)),
        Some("off" | "0" | "false") => Ok(Some(false)),
        Some(w) => Err(format!("expected on/off, got '{w}'")),
    }
}

fn mission_number(word: &str) -> Result<u8, String> {
    match word.parse::<u8>() {
        Ok(n) if (1..=crate::app::MISSION_COUNT).contains(&n) => Ok(n),
        _ => Err(format!(
            "mission must be 1-{}, got '{word}'",
            crate::app::MISSION_COUNT
        )),
    }
}

/// Parse one console line. `Ok(None)` for an empty line.
pub fn parse(line: &str) -> Result<Option<Cheat>, String> {
    let mut words = line.split_whitespace().map(str::to_ascii_lowercase);
    let Some(cmd) = words.next() else {
        return Ok(None);
    };
    let arg = words.next();
    let arg = arg.as_deref();
    let cheat = match (cmd.as_str(), arg) {
        ("help" | "?", _) => Cheat::Help,
        ("god" | "iddqd", a) => Cheat::God(on_off(a)?),
        ("heal" | "health", _) => Cheat::Heal,
        ("ammo" | "arm" | "keys", _) => Cheat::Ammo,
        ("credits" | "money", Some(n)) => Cheat::Credits(
            n.parse()
                .map_err(|_| format!("credits needs a number, got '{n}'"))?,
        ),
        ("credits" | "money", None) => return Err("usage: credits N".into()),
        ("kill" | "killall", _) => Cheat::Kill,
        ("win", _) => Cheat::Win,
        ("lose", _) => Cheat::Lose,
        ("level" | "mission" | "goto", Some(n)) => Cheat::Mission(mission_number(n)?),
        ("level" | "mission" | "goto", None) => return Err("usage: level N (1-10)".into()),
        ("board" | "boarding", a) => Cheat::Board(a.map(mission_number).transpose()?),
        ("menu", _) => Cheat::Menu,
        ("quit" | "exit", _) => Cheat::Quit,
        (other, _) => return Err(format!("unknown command '{other}' (try help)")),
    };
    Ok(Some(cheat))
}

/// Console state: the edit line, the output log and a command history.
#[derive(Debug, Default)]
pub struct Console {
    pub open: bool,
    pub input: String,
    pub log: Vec<String>,
    history: Vec<String>,
    recall: usize,
}

const MAX_LOG: usize = 60;
const MAX_INPUT: usize = 60;

impl Console {
    pub fn toggle(&mut self) {
        self.open = !self.open;
    }

    /// Append a printable character to the edit line.
    pub fn push_char(&mut self, c: char) {
        if (' '..='~').contains(&c) && self.input.len() < MAX_INPUT {
            self.input.push(c);
        }
    }

    pub fn backspace(&mut self) {
        self.input.pop();
    }

    pub fn print(&mut self, line: impl Into<String>) {
        self.log.push(line.into());
        let extra = self.log.len().saturating_sub(MAX_LOG);
        self.log.drain(..extra);
    }

    /// Step through earlier commands (`up = true` goes further back).
    pub fn history(&mut self, up: bool) {
        if self.history.is_empty() {
            return;
        }
        self.recall = if up {
            (self.recall + 1).min(self.history.len())
        } else {
            self.recall.saturating_sub(1)
        };
        self.input = match self.recall {
            0 => String::new(),
            n => self.history[self.history.len() - n].clone(),
        };
    }

    /// Run the edit line: echo it, remember it and return the parsed command (errors are printed).
    pub fn submit(&mut self) -> Option<Cheat> {
        let line = std::mem::take(&mut self.input);
        self.recall = 0;
        if line.trim().is_empty() {
            return None;
        }
        self.print(format!("> {line}"));
        self.history.push(line.clone());
        match parse(&line) {
            Ok(Some(Cheat::Help)) => {
                for l in HELP {
                    self.print(*l);
                }
                None
            }
            Ok(c) => c,
            Err(e) => {
                self.print(e);
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_commands_and_aliases() {
        assert_eq!(parse("  "), Ok(None));
        assert_eq!(parse("GOD"), Ok(Some(Cheat::God(None))));
        assert_eq!(parse("god off"), Ok(Some(Cheat::God(Some(false)))));
        assert_eq!(parse("credits -50"), Ok(Some(Cheat::Credits(-50))));
        assert_eq!(parse("level 4"), Ok(Some(Cheat::Mission(4))));
        assert_eq!(parse("board"), Ok(Some(Cheat::Board(None))));
        assert_eq!(parse("board 9"), Ok(Some(Cheat::Board(Some(9)))));
        assert_eq!(parse("win"), Ok(Some(Cheat::Win)));
    }

    #[test]
    fn rejects_bad_input() {
        assert!(parse("level 0").is_err());
        assert!(parse("level 11").is_err());
        assert!(parse("level").is_err());
        assert!(parse("credits x").is_err());
        assert!(parse("god maybe").is_err());
        assert!(parse("xyzzy").is_err());
    }

    #[test]
    fn console_edits_submits_and_recalls() {
        let mut c = Console::default();
        for ch in "kill".chars() {
            c.push_char(ch);
        }
        c.push_char('\n');
        c.push_char('x');
        assert_eq!(c.input, "killx");
        c.backspace();
        assert_eq!(c.submit(), Some(Cheat::Kill));
        assert!(c.input.is_empty());
        c.history(true);
        assert_eq!(c.input, "kill");
        c.history(false);
        assert!(c.input.is_empty());
        for ch in "nope".chars() {
            c.push_char(ch);
        }
        assert_eq!(c.submit(), None);
        assert!(c.log.last().is_some_and(|l| l.contains("unknown")));
    }
}
