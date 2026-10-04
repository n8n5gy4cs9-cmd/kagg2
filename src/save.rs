//! Settings and campaign save files (T28): small versioned `key value` text files in
//! `$KAGG2_HOME`, else `$HOME/.kagg2`, else `./.kagg2`.

use std::path::PathBuf;

use crate::gfx::keys::Bindings;
use crate::sim::mission::Carry;

/// Enemy toughness: Thrall (easy), Karl (normal), Jarl (hard).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Difficulty {
    Thrall,
    Karl,
    Jarl,
}

impl Difficulty {
    pub const ALL: [Difficulty; 3] = [Difficulty::Thrall, Difficulty::Karl, Difficulty::Jarl];

    pub fn name(self) -> &'static str {
        match self {
            Difficulty::Thrall => "Thrall (easy)",
            Difficulty::Karl => "Karl (normal)",
            Difficulty::Jarl => "Jarl (hard)",
        }
    }

    /// Scale applied to `World::difficulty` and `BoardWorld::difficulty`.
    pub fn scale(self) -> f32 {
        crate::balance::DIFFICULTY_SCALE[self as usize]
    }

    fn code(self) -> u8 {
        self as u8
    }

    fn from_code(c: u8) -> Difficulty {
        Difficulty::ALL
            .get(usize::from(c))
            .copied()
            .unwrap_or(Difficulty::Karl)
    }

    pub fn step(self, delta: i32) -> Difficulty {
        Difficulty::from_code((i32::from(self.code()) + delta).clamp(0, 2) as u8)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Settings {
    pub volume: f32,
    pub music: bool,
    pub mouse_sens: f32,
    pub invert_y: bool,
    pub difficulty: Difficulty,
    pub keys: Bindings,
}

impl Default for Settings {
    fn default() -> Settings {
        Settings {
            volume: 0.8,
            music: true,
            mouse_sens: 1.0,
            invert_y: false,
            difficulty: Difficulty::Karl,
            keys: Bindings::default(),
        }
    }
}

impl Settings {
    pub fn to_text(&self) -> String {
        let mut s = format!(
            "kagg2-settings 1\nvolume {:.2}\nmusic {}\nmouse {:.2}\ninvert {}\ndifficulty {}\n",
            self.volume,
            u8::from(self.music),
            self.mouse_sens,
            u8::from(self.invert_y),
            self.difficulty.code()
        );
        for l in self.keys.to_lines() {
            s.push_str(&l);
            s.push('\n');
        }
        s
    }

    /// Parse a settings file; unknown or bad lines keep their defaults, a wrong header gives
    /// all defaults.
    pub fn from_text(text: &str) -> Settings {
        let mut s = Settings::default();
        let mut lines = text.lines();
        if lines.next().map(str::trim) != Some("kagg2-settings 1") {
            return s;
        }
        for line in lines {
            let mut p = line.split_whitespace();
            let (Some(k), Some(v)) = (p.next(), p.next()) else {
                continue;
            };
            match k {
                "volume" => s.volume = v.parse().map_or(s.volume, |x: f32| x.clamp(0.0, 1.0)),
                "music" => s.music = v == "1",
                "mouse" => {
                    s.mouse_sens = v.parse().map_or(s.mouse_sens, |x: f32| x.clamp(0.2, 3.0))
                }
                "invert" => s.invert_y = v == "1",
                "difficulty" => {
                    s.difficulty = v.parse().map_or(s.difficulty, Difficulty::from_code)
                }
                "key" => {
                    if let Some(key) = p.next() {
                        s.keys.apply_line(v, key);
                    }
                }
                _ => {}
            }
        }
        s
    }
}

/// Campaign progress: the next mission to play and what the ship carries into it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SaveGame {
    pub mission: u8,
    pub carry: Carry,
    /// `Ending::code` of the finished campaign, or 0.
    pub ending: u8,
}

impl SaveGame {
    pub fn new_game() -> SaveGame {
        SaveGame {
            mission: 1,
            carry: Carry::default(),
            ending: 0,
        }
    }

    pub fn to_text(&self) -> String {
        let u = self.carry.up;
        format!(
            "kagg2-save 1\nmission {}\ncredits {}\nup {} {} {} {} {}\nending {}\n",
            self.mission, self.carry.credits, u[0], u[1], u[2], u[3], u[4], self.ending
        )
    }

    /// Parse a save file; `None` if the header is wrong or the mission is missing.
    pub fn from_text(text: &str) -> Option<SaveGame> {
        let mut lines = text.lines();
        if lines.next()?.trim() != "kagg2-save 1" {
            return None;
        }
        let mut s = SaveGame::new_game();
        let mut have_mission = false;
        for line in lines {
            let mut p = line.split_whitespace();
            let (Some(k), Some(v)) = (p.next(), p.next()) else {
                continue;
            };
            match k {
                "mission" => {
                    s.mission = v.parse::<u8>().ok()?.clamp(1, crate::app::MISSION_COUNT);
                    have_mission = true;
                }
                "credits" => s.carry.credits = v.parse().ok()?,
                "ending" => s.ending = v.parse().unwrap_or(0),
                "up" => {
                    let mut up = [1u8; 5];
                    for (slot, tok) in up.iter_mut().zip(std::iter::once(v).chain(p)) {
                        *slot = tok.parse::<u8>().ok()?.clamp(1, 3);
                    }
                    s.carry.up = up;
                }
                _ => {}
            }
        }
        have_mission.then_some(s)
    }
}

/// Folder scanned for mod mission scripts (`*.k2m`, see `sim::script`).
pub fn missions_dir() -> PathBuf {
    dir().join("missions")
}

fn dir() -> PathBuf {
    if let Some(d) = std::env::var_os("KAGG2_HOME") {
        return PathBuf::from(d);
    }
    std::env::var_os("HOME")
        .map_or_else(|| PathBuf::from("."), PathBuf::from)
        .join(".kagg2")
}

fn write(name: &str, text: &str) -> std::io::Result<()> {
    let d = dir();
    std::fs::create_dir_all(&d)?;
    std::fs::write(d.join(name), text)
}

fn read(name: &str) -> Option<String> {
    std::fs::read_to_string(dir().join(name)).ok()
}

pub fn load_settings() -> Settings {
    read("settings.txt").map_or_else(Settings::default, |t| Settings::from_text(&t))
}

pub fn store_settings(s: &Settings) -> std::io::Result<()> {
    write("settings.txt", &s.to_text())
}

pub fn load_game() -> Option<SaveGame> {
    read("save.txt").and_then(|t| SaveGame::from_text(&t))
}

pub fn store_game(s: &SaveGame) -> std::io::Result<()> {
    write("save.txt", &s.to_text())
}

#[cfg(test)]
mod tests {
    use super::*;
    use macroquad::prelude::KeyCode;

    #[test]
    fn settings_round_trip_and_clamp() {
        let mut s = Settings {
            volume: 0.3,
            music: false,
            mouse_sens: 1.5,
            invert_y: true,
            difficulty: Difficulty::Jarl,
            ..Default::default()
        };
        s.keys.set(crate::gfx::keys::Bind::Fire, KeyCode::Key5);
        assert_eq!(Settings::from_text(&s.to_text()), s);
        let bad =
            Settings::from_text("kagg2-settings 1\nvolume 9\nmouse 0\ndifficulty 7\nfoo bar\n");
        assert_eq!(bad.volume, 1.0);
        assert_eq!(bad.mouse_sens, 0.2);
        assert_eq!(bad.difficulty, Difficulty::Karl);
        assert_eq!(Settings::from_text("garbage"), Settings::default());
    }

    #[test]
    fn save_round_trip_and_rejects_bad_files() {
        let g = SaveGame {
            mission: 6,
            carry: Carry {
                credits: 4200,
                up: [2, 1, 3, 1, 2],
            },
            ending: 1,
        };
        assert_eq!(SaveGame::from_text(&g.to_text()), Some(g));
        assert_eq!(SaveGame::from_text("kagg2-save 2\nmission 3\n"), None);
        assert_eq!(SaveGame::from_text("kagg2-save 1\ncredits 5\n"), None);
        assert_eq!(SaveGame::from_text("kagg2-save 1\nmission x\n"), None);
        let clamped = SaveGame::from_text("kagg2-save 1\nmission 99\nup 9 0 2 2 2\n");
        assert_eq!(
            clamped.map(|s| (s.mission, s.carry.up[0], s.carry.up[1])),
            Some((10, 3, 1))
        );
    }

    #[test]
    fn difficulty_scales_and_steps_within_range() {
        assert!(Difficulty::Thrall.scale() < Difficulty::Karl.scale());
        assert!(Difficulty::Karl.scale() < Difficulty::Jarl.scale());
        assert_eq!(Difficulty::Jarl.step(1), Difficulty::Jarl);
        assert_eq!(Difficulty::Thrall.step(-1), Difficulty::Thrall);
        assert_eq!(Difficulty::Karl.step(1), Difficulty::Jarl);
    }
}
