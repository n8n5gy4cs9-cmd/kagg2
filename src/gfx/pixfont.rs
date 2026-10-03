//! Pixel-font text drawn from the generated `font_atlas` texture (5×7 glyphs in 8×8 cells).

use macroquad::prelude::*;

use crate::assets::Assets;
use crate::texgen::font::{ATLAS_CELL, ATLAS_COLS};

/// Width in screen pixels of `text` at integer pixel `scale`.
pub fn width(text: &str, scale: f32) -> f32 {
    (text.chars().count() as f32 * 6.0 - 1.0).max(0.0) * scale
}

/// Draw `text` with its top-left at `(x, y)`; a one-pixel shadow keeps it readable.
pub fn draw(assets: &Assets, text: &str, x: f32, y: f32, scale: f32, color: Color) {
    let atlas = assets.tex("font_atlas");
    let shadow = Color::new(0.0, 0.0, 0.0, color.a * 0.8);
    for (pass, col) in [(1.0, shadow), (0.0, color)] {
        let (mut cx, cy) = (x + pass * scale, y + pass * scale);
        for ch in text.chars() {
            let code = (ch as u32).clamp(32, 126) - 32;
            let src = Rect::new(
                (code % ATLAS_COLS * ATLAS_CELL) as f32,
                (code / ATLAS_COLS * ATLAS_CELL) as f32,
                ATLAS_CELL as f32,
                ATLAS_CELL as f32,
            );
            draw_texture_ex(
                atlas,
                cx.round(),
                cy.round(),
                col,
                DrawTextureParams {
                    dest_size: Some(vec2(ATLAS_CELL as f32 * scale, ATLAS_CELL as f32 * scale)),
                    source: Some(src),
                    ..Default::default()
                },
            );
            cx += 6.0 * scale;
        }
    }
}

pub fn draw_centered(assets: &Assets, text: &str, cx: f32, y: f32, scale: f32, color: Color) {
    draw(assets, text, cx - width(text, scale) / 2.0, y, scale, color);
}

pub fn draw_right(assets: &Assets, text: &str, right: f32, y: f32, scale: f32, color: Color) {
    draw(assets, text, right - width(text, scale), y, scale, color);
}
