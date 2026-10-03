//! Settings screen (T28): difficulty, volume, music, mouse, invert-Y and key rebinding.
//! Keyboard only: Up/Down select, Left/Right change, Enter rebinds, Esc returns.

use macroquad::prelude::*;

use super::keys::{Bind, key_name, pressed_bindable};
use crate::save::Settings;

const RUST: Color = Color::new(0.85, 0.36, 0.20, 1.0);
const BONE: Color = Color::new(0.85, 0.80, 0.70, 1.0);
const DIM: Color = Color::new(0.45, 0.45, 0.48, 1.0);
const BAD: Color = Color::new(1.0, 0.4, 0.3, 1.0);

const FIXED_ROWS: usize = 5;
const ROWS: usize = FIXED_ROWS + Bind::ALL.len() + 2;
const RESET: usize = FIXED_ROWS + Bind::ALL.len();
const BACK: usize = RESET + 1;

#[derive(Default)]
pub struct SettingsUi {
    row: usize,
    rebinding: bool,
}

fn label(s: &Settings, row: usize) -> (String, String) {
    match row {
        0 => ("Difficulty".into(), s.difficulty.name().into()),
        1 => ("Master volume".into(), format!("{:.0}%", s.volume * 100.0)),
        2 => ("Music".into(), if s.music { "on" } else { "off" }.into()),
        3 => ("Mouse sensitivity".into(), format!("{:.1}x", s.mouse_sens)),
        4 => (
            "Invert Y".into(),
            if s.invert_y { "on" } else { "off" }.into(),
        ),
        RESET => ("Reset key bindings".into(), "Enter".into()),
        BACK => ("Back".into(), "Enter".into()),
        r => {
            let b = Bind::ALL[r - FIXED_ROWS];
            (format!("Key: {}", b.name()), key_name(s.keys.get(b)).into())
        }
    }
}

impl SettingsUi {
    /// Handle input; returns `true` when the player leaves the screen (caller saves).
    pub fn update(&mut self, s: &mut Settings) -> bool {
        if self.rebinding {
            if is_key_pressed(KeyCode::Escape) {
                self.rebinding = false;
            } else if let Some(k) = pressed_bindable()
                && let Some(b) = Bind::ALL.get(self.row.wrapping_sub(FIXED_ROWS))
            {
                s.keys.set(*b, k);
                self.rebinding = false;
            }
            return false;
        }
        if is_key_pressed(KeyCode::Escape) {
            return true;
        }
        if is_key_pressed(KeyCode::Down) {
            self.row = (self.row + 1) % ROWS;
        }
        if is_key_pressed(KeyCode::Up) {
            self.row = (self.row + ROWS - 1) % ROWS;
        }
        let dir =
            i32::from(is_key_pressed(KeyCode::Right)) - i32::from(is_key_pressed(KeyCode::Left));
        let enter = is_key_pressed(KeyCode::Enter);
        match self.row {
            0 => s.difficulty = s.difficulty.step(dir),
            1 => s.volume = (s.volume + dir as f32 * 0.1).clamp(0.0, 1.0),
            2 if dir != 0 || enter => s.music = !s.music,
            3 => s.mouse_sens = (s.mouse_sens + dir as f32 * 0.1).clamp(0.2, 3.0),
            4 if dir != 0 || enter => s.invert_y = !s.invert_y,
            RESET if enter => s.keys = Default::default(),
            BACK if enter => return true,
            r if (FIXED_ROWS..RESET).contains(&r) && enter => self.rebinding = true,
            _ => {}
        }
        false
    }

    pub fn draw(&self, s: &Settings) {
        let (sw, sh) = (screen_width(), screen_height());
        let title = "SETTINGS";
        let tw = measure_text(title, None, 48, 1.0).width;
        draw_text(title, (sw - tw) / 2.0, 56.0, 48.0, RUST);
        let lh = 26.0;
        let visible = (((sh - 150.0) / lh) as usize).max(4);
        let first = self
            .row
            .saturating_sub(visible - 1)
            .min(ROWS.saturating_sub(visible));
        for (i, row) in (first..ROWS.min(first + visible)).enumerate() {
            let (name, value) = label(s, row);
            let y = 100.0 + i as f32 * lh;
            let sel = row == self.row;
            if sel {
                draw_rectangle(
                    sw * 0.2,
                    y - 20.0,
                    sw * 0.6,
                    lh,
                    Color::new(0.85, 0.36, 0.2, 0.35),
                );
            }
            let clash =
                (FIXED_ROWS..RESET).contains(&row) && s.keys.conflicts(Bind::ALL[row - FIXED_ROWS]);
            let c = if clash {
                BAD
            } else if sel {
                WHITE
            } else {
                BONE
            };
            draw_text(&name, sw * 0.2 + 10.0, y, 22.0, c);
            let value = if sel && self.rebinding {
                "press a key...".to_owned()
            } else {
                value
            };
            draw_text(&value, sw * 0.58, y, 22.0, c);
        }
        let hint = if self.rebinding {
            "Press the new key, Esc cancels"
        } else {
            "Up/Down select  -  Left/Right change  -  Enter rebinds  -  Esc saves and returns  (red = key used twice)"
        };
        let hw = measure_text(hint, None, 18, 1.0).width;
        draw_text(hint, (sw - hw) / 2.0, sh - 24.0, 18.0, DIM);
    }
}
