//! Developer console overlay: edit line plus the last output lines (see `cheats`).

use macroquad::prelude::*;

use super::pixfont;
use crate::assets::Assets;
use crate::cheats::Console;

/// Read typed characters and editing keys; returns true when Enter was pressed.
pub fn input(c: &mut Console) -> bool {
    while let Some(ch) = get_char_pressed() {
        if !matches!(ch, '`' | '§' | '´') {
            c.push_char(ch);
        }
    }
    if is_key_pressed(KeyCode::Backspace) {
        c.backspace();
    }
    if is_key_pressed(KeyCode::Up) {
        c.history(true);
    }
    if is_key_pressed(KeyCode::Down) {
        c.history(false);
    }
    is_key_pressed(KeyCode::Enter)
}

/// Whether the key press that toggles the console happened (`` ` `` / `§` / F12).
pub fn toggle_pressed() -> bool {
    is_key_pressed(KeyCode::GraveAccent)
        || is_key_pressed(KeyCode::World1)
        || is_key_pressed(KeyCode::F12)
}

pub fn draw(assets: &Assets, c: &Console) {
    const LINES: usize = 12;
    let w = screen_width();
    let h = 16.0 * LINES as f32 + 40.0;
    draw_rectangle(0.0, 0.0, w, h, Color::new(0.0, 0.0, 0.05, 0.86));
    draw_line(0.0, h, w, h, 2.0, Color::new(0.3, 0.9, 0.5, 1.0));
    let start = c.log.len().saturating_sub(LINES);
    for (i, line) in c.log[start..].iter().enumerate() {
        let col = if line.starts_with('>') {
            WHITE
        } else {
            Color::new(0.6, 0.9, 0.7, 1.0)
        };
        pixfont::draw(
            assets,
            &line.to_uppercase(),
            10.0,
            8.0 + i as f32 * 16.0,
            2.0,
            col,
        );
    }
    let cursor = if ((get_time() * 2.0) as u32).is_multiple_of(2) {
        "_"
    } else {
        " "
    };
    pixfont::draw(
        assets,
        &format!("> {}{cursor}", c.input.to_uppercase()),
        10.0,
        h - 24.0,
        2.0,
        YELLOW,
    );
}
