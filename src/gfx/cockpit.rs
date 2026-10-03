//! Cockpit frame of the longship *Huginn*: iron-and-wood struts drawn in the pixel target.

use macroquad::prelude::*;

fn hex(rgb: u32) -> Color {
    Color::from_hex(rgb)
}

/// Draw the frame over a `w`×`h` pixel image. `t` animates the lamps.
pub fn draw(w: f32, h: f32, t: f32, throttle: f32) {
    let (iron, iron_hi, iron_lo) = (hex(0x2a2c30), hex(0x5a5e66), hex(0x14161a));
    let (wood, wood_hi, bone, rust) = (hex(0x4a2e1a), hex(0x7a5030), hex(0xd8cdb4), hex(0xb5482a));

    // Side struts (wood beams edged with iron), mirrored.
    for side in [0.0f32, 1.0] {
        let s = |x: f32| if side == 0.0 { x } else { w - x };
        let quad = [
            (0.0, 0.0),
            (w * 0.085, 0.0),
            (w * 0.028, h * 0.80),
            (0.0, h * 0.90),
        ];
        let p = quad.map(|(x, y)| vec2(s(x), y));
        draw_triangle(p[0], p[1], p[2], wood);
        draw_triangle(p[0], p[2], p[3], wood);
        draw_line(p[1].x, p[1].y, p[2].x, p[2].y, 2.0, iron);
        draw_line(
            p[1].x - side * 2.0 + (1.0 - side) * 2.0,
            p[1].y,
            p[2].x - side * 2.0 + (1.0 - side) * 2.0,
            p[2].y,
            1.0,
            wood_hi,
        );
        let mut y = 10.0;
        while y < h * 0.78 {
            let x = s(w * (0.085 - 0.057 * y / (h * 0.8)) - 4.0);
            draw_rectangle(x, y, 2.0, 2.0, bone);
            y += 14.0;
        }
    }

    // Top beam.
    draw_rectangle(0.0, 0.0, w, h * 0.05, iron);
    draw_rectangle(0.0, h * 0.05 - 2.0, w, 2.0, rust);
    draw_rectangle(0.0, h * 0.05, w, 1.0, iron_lo);
    let mut x = 8.0;
    while x < w {
        draw_rectangle(x, 3.0, 2.0, 2.0, iron_hi);
        x += 16.0;
    }

    // Dashboard with raised centre console.
    let top = h * 0.84;
    draw_rectangle(0.0, top, w, h - top, iron);
    draw_rectangle(0.0, top, w, 2.0, iron_hi);
    draw_rectangle(0.0, top + 2.0, w, 1.0, iron_lo);
    let (cx, cw) = (w * 0.5, w * 0.16);
    draw_triangle(
        vec2(cx - cw, top),
        vec2(cx + cw, top),
        vec2(cx, top - h * 0.045),
        iron,
    );
    draw_line(cx - cw, top, cx, top - h * 0.045, 1.0, iron_hi);
    draw_line(cx + cw, top, cx, top - h * 0.045, 1.0, iron_hi);
    draw_rectangle(cx - 1.0, top - h * 0.03, 2.0, h * 0.02, bone); // tiny prow marker

    // Lamps along the dash; the throttle row lights up proportionally.
    for i in 0..10 {
        let x = w * 0.30 + i as f32 * (w * 0.4 / 9.0);
        let on = (i as f32 + 0.5) / 10.0 <= throttle.max(0.0);
        let c = if on { hex(0x35c8e0) } else { hex(0x123a44) };
        draw_rectangle(x - 2.0, top + 6.0, 4.0, 3.0, c);
    }
    let blink = ((t * 2.0).sin() > 0.0) as u32 as f32;
    draw_rectangle(
        w * 0.18,
        top + 6.0,
        4.0,
        3.0,
        Color::new(1.0, 0.55 + 0.2 * blink, 0.1, 1.0),
    );
    draw_rectangle(
        w * 0.80,
        top + 6.0,
        4.0,
        3.0,
        Color::new(0.3, 1.0, 0.5, 1.0),
    );
    // rune strip
    let mut rx = w * 0.12;
    let mut seed = 7u32;
    while rx < w * 0.88 {
        seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12_345);
        let hh = 3.0 + ((seed >> 16) % 3) as f32;
        draw_rectangle(rx, top + 14.0, 1.0, hh, rust);
        if (seed >> 8) & 1 == 1 {
            draw_rectangle(rx + 1.0, top + 14.0, 1.0, 1.0, rust);
        }
        rx += 4.0;
    }
}
