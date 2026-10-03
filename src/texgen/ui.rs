//! UI textures: panels, buttons, sliders, title logo, bitmap font atlas, HUD frame, crosshairs.

use super::font::{ATLAS_CELL, ATLAS_COLS, glyph};
use super::image::ramp;
use super::image::{Image, hex, scale, with_alpha};
use super::noise::noise2;
use super::ships::{pal, rune_strip};
use super::{Ctx, Registry};

/// Draw `text` with the 5×7 font at pixel scale `k`.
pub fn draw_text(img: &mut Image, text: &str, x: i32, y: i32, k: i32, col: [f32; 4]) {
    for (i, ch) in text.chars().enumerate() {
        let rows = glyph(ch);
        for (j, row) in rows.iter().enumerate() {
            for b in 0..5 {
                if row >> (4 - b) & 1 == 1 {
                    img.rect(x + (i as i32 * 6 + b) * k, y + j as i32 * k, k, k, col);
                }
            }
        }
    }
}

fn bevel_panel(c: &Ctx, fill: [u32; 3], edge: u32, glow: u32) -> Image {
    let p = pal(&fill);
    let mut img = Image::from_fn(c.w, c.h, 1, |x, y| {
        let n = noise2(x as f32 / 6.0, y as f32 / 6.0, 0, 0, c.seed);
        ramp(&p, n * 0.5 + (1.0 - y as f32 / c.h as f32) * 0.5, x, y)
    });
    let (w, h) = (c.w as i32, c.h as i32);
    img.frame(0, 0, w, h, hex(edge));
    img.frame(1, 1, w - 2, h - 2, with_alpha(hex(glow), 0.5));
    for (x, y) in [(3, 3), (w - 5, 3), (3, h - 5), (w - 5, h - 5)] {
        img.rect(x, y, 2, 2, hex(edge));
    }
    img
}

fn title_logo(c: &Ctx) -> Image {
    let mut img = Image::new(c.w, c.h);
    let text = "KAQQ2";
    let k = 16;
    let tw = text.len() as i32 * 6 * k - k;
    let x0 = (c.w as i32 - tw) / 2;
    let y0 = (c.h as i32 - 7 * k) / 2 - 4;
    // drop shadow, iron outline, ember fill with vertical gradient
    draw_text(&mut img, text, x0 + k / 2, y0 + k / 2 + 4, k, hex(0x120806));
    for (dx, dy) in [(-2, 0), (2, 0), (0, -2), (0, 2)] {
        draw_text(&mut img, text, x0 + dx, y0 + dy, k, hex(0x3a1a10));
    }
    let mut fill = Image::new(c.w, c.h);
    draw_text(&mut fill, text, x0, y0, k, [1.0; 4]);
    let p = pal(&[0x8c2a10, 0xb5482a, 0xdc6a24, 0xff8a2b, 0xffc060]);
    for y in 0..c.h as i32 {
        for x in 0..c.w as i32 {
            if fill.get(x, y)[3] > 0.5 {
                let t = 1.0 - (y - y0) as f32 / (7 * k) as f32;
                let pix = (x / 4) as u32;
                img.put(x, y, ramp(&p, t, pix, (y / 4) as u32));
            }
        }
    }
    let rune = hex(0xd8cdb4);
    rune_strip(&mut img, 6, 2, rune, c.seed);
    rune_strip(&mut img, c.h as i32 - 20, 2, rune, c.seed + 1);
    img
}

fn font_atlas(c: &Ctx) -> Image {
    let mut img = Image::new(c.w, c.h);
    for code in 32u32..=126 {
        let i = code - 32;
        let (cx, cy) = (i % ATLAS_COLS * ATLAS_CELL, i / ATLAS_COLS * ATLAS_CELL);
        draw_text(
            &mut img,
            &char::from_u32(code).unwrap_or(' ').to_string(),
            cx as i32,
            cy as i32,
            1,
            [1.0; 4],
        );
    }
    img
}

fn crosshair(c: &Ctx, style: u32) -> Image {
    let mut img = Image::new(c.w, c.h);
    let (m, col) = (c.w as i32 / 2, hex(0x7affc8));
    let glow = with_alpha(col, 0.35);
    match style {
        0 => {
            for (a, b) in [(-28, -10), (10, 28)] {
                img.rect(m + a, m - 1, b - a, 2, col);
                img.rect(m - 1, m + a, 2, b - a, col);
            }
            img.rect(m - 1, m - 1, 2, 2, col);
        }
        1 => {
            img.disc(m as f32, m as f32, 24.0, glow);
            img.disc(m as f32, m as f32, 21.0, [0.0; 4]);
            for t in 0..4 {
                let (dx, dy) = [(1, 0), (0, 1), (-1, 0), (0, -1)][t];
                img.rect(m + dx * 24 - 2, m + dy * 24 - 2, 4, 4, col);
            }
            img.rect(m - 1, m - 1, 2, 2, col);
        }
        _ => {
            for k in 0..4 {
                let (sx, sy) = [(-1, -1), (1, -1), (-1, 1), (1, 1)][k];
                img.line(m + sx * 8, m + sy * 20, m + sx * 20, m + sy * 20, col);
                img.line(m + sx * 20, m + sy * 8, m + sx * 20, m + sy * 20, col);
            }
            img.rect(m - 1, m - 1, 2, 2, col);
        }
    }
    img
}

pub fn register(r: &mut Registry) {
    r.add("ui_panel", 128, 128, |c| {
        bevel_panel(c, [0x10161c, 0x182028, 0x202a34], 0x586470, 0x35c8e0)
    });
    r.add("ui_button", 128, 32, |c| {
        bevel_panel(c, [0x5a2414, 0x8c3a22, 0xb5482a], 0xd8cdb4, 0xff8a2b)
    });
    r.add("ui_slider", 128, 32, |c| {
        let mut img = Image::filled(c.w, c.h, with_alpha(hex(0x10161c), 0.9));
        img.frame(0, 12, 128, 8, hex(0x586470));
        img.rect(2, 14, 76, 4, hex(0xff8a2b));
        img.rect(74, 6, 8, 20, hex(0xd8cdb4));
        img.frame(74, 6, 8, 20, hex(0x3a1a10));
        img
    });
    r.add("title_logo", 512, 128, title_logo);
    r.add("font_atlas", 128, 48, font_atlas);
    r.add("hud_frame", 128, 128, |c| {
        let mut img = Image::new(c.w, c.h);
        let col = hex(0x35c8e0);
        for (sx, sy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
            let (x, y) = (sx * 127, sy * 127);
            let (dx, dy) = (if sx == 0 { 1 } else { -1 }, if sy == 0 { 1 } else { -1 });
            img.line(x, y + dy * 30, x, y, col);
            img.line(x, y, x + dx * 30, y, col);
            img.line(
                x + dx * 4,
                y + dy * 40,
                x + dx * 4,
                y + dy * 6,
                scale(col, 0.4),
            );
        }
        img
    });
    r.add("crosshair_a", 128, 128, |c| crosshair(c, 0));
    r.add("crosshair_b", 128, 128, |c| crosshair(c, 1));
    r.add("crosshair_c", 128, 128, |c| crosshair(c, 2));
}
