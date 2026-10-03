//! Rebindable flight keys (T28). The defaults match the key list in the help screens.

use macroquad::prelude::KeyCode;

/// Flight actions the player can rebind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bind {
    ThrottleUp,
    ThrottleDown,
    FullThrottle,
    ZeroThrottle,
    YawLeft,
    YawRight,
    PitchUp,
    PitchDown,
    RollLeft,
    RollRight,
    StrafeLeft,
    StrafeRight,
    StrafeUp,
    StrafeDown,
    Brake,
    Fire,
    Torpedo,
    Missile,
    NextTarget,
    ClearTarget,
    Board,
    Dock,
    Autopilot,
    Scan,
    Camera,
    Assist,
}

impl Bind {
    pub const ALL: [Bind; 26] = [
        Bind::ThrottleUp,
        Bind::ThrottleDown,
        Bind::FullThrottle,
        Bind::ZeroThrottle,
        Bind::YawLeft,
        Bind::YawRight,
        Bind::PitchUp,
        Bind::PitchDown,
        Bind::RollLeft,
        Bind::RollRight,
        Bind::StrafeLeft,
        Bind::StrafeRight,
        Bind::StrafeUp,
        Bind::StrafeDown,
        Bind::Brake,
        Bind::Fire,
        Bind::Torpedo,
        Bind::Missile,
        Bind::NextTarget,
        Bind::ClearTarget,
        Bind::Board,
        Bind::Dock,
        Bind::Autopilot,
        Bind::Scan,
        Bind::Camera,
        Bind::Assist,
    ];

    /// Label shown in the settings screen (also the save-file token).
    pub fn name(self) -> &'static str {
        match self {
            Bind::ThrottleUp => "Throttle up",
            Bind::ThrottleDown => "Throttle down",
            Bind::FullThrottle => "Full throttle",
            Bind::ZeroThrottle => "Zero throttle",
            Bind::YawLeft => "Yaw left",
            Bind::YawRight => "Yaw right",
            Bind::PitchUp => "Pitch up",
            Bind::PitchDown => "Pitch down",
            Bind::RollLeft => "Roll left",
            Bind::RollRight => "Roll right",
            Bind::StrafeLeft => "Strafe left",
            Bind::StrafeRight => "Strafe right",
            Bind::StrafeUp => "Strafe up",
            Bind::StrafeDown => "Strafe down",
            Bind::Brake => "Brake",
            Bind::Fire => "Fire lances",
            Bind::Torpedo => "Fire torpedo",
            Bind::Missile => "Fire missile",
            Bind::NextTarget => "Next target",
            Bind::ClearTarget => "Clear target",
            Bind::Board => "Board ship",
            Bind::Dock => "Dock / undock",
            Bind::Autopilot => "Autopilot",
            Bind::Scan => "Scan target",
            Bind::Camera => "Camera",
            Bind::Assist => "Flight assist",
        }
    }

    fn default_key(self) -> KeyCode {
        match self {
            Bind::ThrottleUp => KeyCode::W,
            Bind::ThrottleDown => KeyCode::S,
            Bind::FullThrottle => KeyCode::LeftShift,
            Bind::ZeroThrottle => KeyCode::X,
            Bind::YawLeft => KeyCode::A,
            Bind::YawRight => KeyCode::D,
            Bind::PitchUp => KeyCode::Up,
            Bind::PitchDown => KeyCode::Down,
            Bind::RollLeft => KeyCode::Q,
            Bind::RollRight => KeyCode::E,
            Bind::StrafeLeft => KeyCode::Left,
            Bind::StrafeRight => KeyCode::Right,
            Bind::StrafeUp => KeyCode::R,
            Bind::StrafeDown => KeyCode::F,
            Bind::Brake => KeyCode::Space,
            Bind::Fire => KeyCode::Enter,
            Bind::Torpedo => KeyCode::Z,
            Bind::Missile => KeyCode::V,
            Bind::NextTarget => KeyCode::T,
            Bind::ClearTarget => KeyCode::G,
            Bind::Board => KeyCode::B,
            Bind::Dock => KeyCode::L,
            Bind::Autopilot => KeyCode::P,
            Bind::Scan => KeyCode::N,
            Bind::Camera => KeyCode::C,
            Bind::Assist => KeyCode::M,
        }
    }

    fn index(self) -> usize {
        Bind::ALL.iter().position(|b| *b == self).unwrap_or(0)
    }
}

/// Keys that may be bound, with their save-file names.
const KEYS: &[(KeyCode, &str)] = &[
    (KeyCode::A, "A"),
    (KeyCode::B, "B"),
    (KeyCode::C, "C"),
    (KeyCode::D, "D"),
    (KeyCode::E, "E"),
    (KeyCode::F, "F"),
    (KeyCode::G, "G"),
    (KeyCode::H, "H"),
    (KeyCode::I, "I"),
    (KeyCode::J, "J"),
    (KeyCode::K, "K"),
    (KeyCode::L, "L"),
    (KeyCode::M, "M"),
    (KeyCode::N, "N"),
    (KeyCode::O, "O"),
    (KeyCode::P, "P"),
    (KeyCode::Q, "Q"),
    (KeyCode::R, "R"),
    (KeyCode::S, "S"),
    (KeyCode::T, "T"),
    (KeyCode::U, "U"),
    (KeyCode::V, "V"),
    (KeyCode::W, "W"),
    (KeyCode::X, "X"),
    (KeyCode::Y, "Y"),
    (KeyCode::Z, "Z"),
    (KeyCode::Key0, "0"),
    (KeyCode::Key1, "1"),
    (KeyCode::Key2, "2"),
    (KeyCode::Key3, "3"),
    (KeyCode::Key4, "4"),
    (KeyCode::Key5, "5"),
    (KeyCode::Key6, "6"),
    (KeyCode::Key7, "7"),
    (KeyCode::Key8, "8"),
    (KeyCode::Key9, "9"),
    (KeyCode::Up, "Up"),
    (KeyCode::Down, "Down"),
    (KeyCode::Left, "Left"),
    (KeyCode::Right, "Right"),
    (KeyCode::Space, "Space"),
    (KeyCode::Enter, "Enter"),
    (KeyCode::LeftShift, "LShift"),
    (KeyCode::RightShift, "RShift"),
    (KeyCode::LeftControl, "LCtrl"),
    (KeyCode::RightControl, "RCtrl"),
    (KeyCode::LeftAlt, "LAlt"),
    (KeyCode::Tab, "Tab"),
    (KeyCode::LeftBracket, "["),
    (KeyCode::RightBracket, "]"),
    (KeyCode::Comma, ","),
    (KeyCode::Period, "."),
    (KeyCode::Semicolon, ";"),
    (KeyCode::Apostrophe, "'"),
    (KeyCode::Slash, "/"),
    (KeyCode::Backspace, "Backspace"),
    (KeyCode::Insert, "Insert"),
    (KeyCode::Delete, "Delete"),
    (KeyCode::Home, "Home"),
    (KeyCode::End, "End"),
    (KeyCode::PageUp, "PageUp"),
    (KeyCode::PageDown, "PageDown"),
];

/// Display / save name of a key; `?` for keys that cannot be bound.
pub fn key_name(k: KeyCode) -> &'static str {
    KEYS.iter().find(|(c, _)| *c == k).map_or("?", |(_, n)| *n)
}

fn key_from_name(n: &str) -> Option<KeyCode> {
    KEYS.iter().find(|(_, name)| *name == n).map(|(c, _)| *c)
}

/// First bindable key pressed this frame (used while rebinding).
pub fn pressed_bindable() -> Option<KeyCode> {
    KEYS.iter()
        .map(|(c, _)| *c)
        .find(|c| macroquad::input::is_key_pressed(*c))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bindings {
    codes: [KeyCode; 26],
}

impl Default for Bindings {
    fn default() -> Bindings {
        Bindings {
            codes: Bind::ALL.map(Bind::default_key),
        }
    }
}

impl Bindings {
    pub fn get(&self, b: Bind) -> KeyCode {
        self.codes[b.index()]
    }

    pub fn set(&mut self, b: Bind, k: KeyCode) {
        self.codes[b.index()] = k;
    }

    /// Bindings that currently share a key with `b` (for the conflict marker).
    pub fn conflicts(&self, b: Bind) -> bool {
        Bind::ALL
            .iter()
            .any(|o| *o != b && self.get(*o) == self.get(b))
    }

    /// `name key` lines for the settings file.
    pub fn to_lines(&self) -> Vec<String> {
        Bind::ALL
            .iter()
            .map(|b| {
                format!(
                    "key {} {}",
                    b.name().replace(' ', "_"),
                    key_name(self.get(*b))
                )
            })
            .collect()
    }

    /// Apply one `key <bind> <key>` line; unknown names are ignored.
    pub fn apply_line(&mut self, bind: &str, key: &str) {
        let bind = bind.replace('_', " ");
        if let (Some(b), Some(k)) = (
            Bind::ALL.iter().find(|b| b.name() == bind),
            key_from_name(key),
        ) {
            self.set(*b, k);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_have_no_conflicts_and_round_trip() {
        let b = Bindings::default();
        assert!(Bind::ALL.iter().all(|x| !b.conflicts(*x)));
        let mut c = Bindings::default();
        c.set(Bind::Fire, KeyCode::Key5);
        let mut d = Bindings::default();
        for line in c.to_lines() {
            let mut p = line.split_whitespace();
            p.next();
            if let (Some(n), Some(k)) = (p.next(), p.next()) {
                d.apply_line(n, k);
            }
        }
        assert_eq!(d, c);
        assert_eq!(d.get(Bind::Fire), KeyCode::Key5);
    }

    #[test]
    fn every_default_key_is_bindable() {
        for b in Bind::ALL {
            assert_ne!(key_name(Bindings::default().get(b)), "?", "{b:?}");
        }
    }
}
