//! Boarding-level textures: walls, floors, ceilings, doors, consoles, props, decals.

use super::image::{Col, Image, hex, mix, ramp, scale, with_alpha};
use super::noise::{fbm2, hash, noise2};
use super::ships::{hazard, hex_vent, pal, plates, rune_strip};
use super::{Ctx, Registry};

const CYAN: u32 = 0x35c8e0;

fn wall_minion(c: &Ctx, kind: u32) -> Image {
    let p = pal(&[0x8a96a2, 0xaab6c0, 0xc8d2da, 0xe8eef2]);
    let mut img = plates(c, &p, 2, 2, 2, hex(0x586470), hex(0x6a7480));
    let (w, h) = (c.w as i32, c.h as i32);
    img.rect(0, h / 2 - 2, w, 4, hex(0x2a2f36));
    img.rect(0, h / 2 - 1, w, 1, hex(CYAN));
    match kind {
        1 => {
            // conduit panel: pipes
            for i in 0..3 {
                let x = 20 + i * 36;
                img.rect(x, 8, 8, h / 2 - 20, hex(0x3a4048));
                img.rect(x + 1, 8, 2, h / 2 - 20, hex(0x6a7480));
                img.rect(x - 2, 40 + i * 8, 12, 4, hex(0x2a2f36));
            }
        }
        2 => hex_vent(&mut img, w / 2, h * 3 / 4, 26, hex(0x1a1d22), hex(0x586470)),
        _ => {
            img.rect(10, 10, 40, 4, hex(CYAN));
            img.rect(10, 16, 20, 2, scale(hex(CYAN), 0.5));
        }
    }
    img
}

fn wall_viking(c: &Ctx, cell: bool) -> Image {
    let wood = pal(&[0x2a1a10, 0x4a2e1a, 0x6a4426, 0x8a5c34]);
    let iron = pal(&[0x2a2c30, 0x4a4d52, 0x6a6e74]);
    let (w, h) = (c.w as i32, c.h as i32);
    Image::from_fn(c.w, c.h, 2, |x, y| {
        let (px, py) = (x as i32 * 2, y as i32 * 2);
        let band = px / (w / 8);
        let grain = noise2(
            x as f32 / 2.0,
            y as f32 / 30.0 + band as f32 * 5.0,
            0,
            0,
            c.seed,
        );
        let plank = (px % (w / 8)) < 2;
        let iron_strip = cell && (py % (h / 2) < 12) || (py % (h / 2) < 6 && !cell);
        if iron_strip {
            let rivet = (px % 32) < 4 && (py % (h / 2)) > 1 && (py % (h / 2)) < 6;
            if rivet {
                hex(0xd8cdb4)
            } else {
                ramp(&iron, grain, x, y)
            }
        } else if plank {
            hex(0x120a06)
        } else {
            ramp(&wood, grain * 0.8 + hash(band, 0, 0, c.seed) * 0.3, x, y)
        }
    })
}

fn wall_reactor(c: &Ctx) -> Image {
    let p = pal(&[0x0e1a22, 0x1a2c38, 0x2a4452]);
    let (w, h) = (c.w as i32, c.h as i32);
    let mut img = Image::from_fn(c.w, c.h, 2, |x, y| {
        ramp(
            &p,
            fbm2(x as f32 / 128.0, y as f32 / 128.0, 4, 3, c.seed),
            x,
            y,
        )
    });
    for i in 0..4 {
        let x = i * (w / 4) + w / 8 - 6;
        img.rect(x - 2, 0, 16, h, hex(0x0a1218));
        for y in 0..h {
            let pulse = 0.55 + 0.45 * ((y as f32 / 18.0 + i as f32).sin());
            img.rect(x, y, 12, 1, mix(hex(0x1a8aa0), hex(0xb8f4ff), pulse));
        }
    }
    img
}

fn floor_grate(c: &Ctx) -> Image {
    let p = pal(&[0x14161a, 0x2a2e34, 0x464c54, 0x6a727c]);
    Image::from_fn(c.w, c.h, 2, |x, y| {
        let (lx, ly) = (x % 8, y % 8);
        if lx < 5 && ly < 5 {
            hex(0x050608)
        } else {
            let n = fbm2(x as f32 / 128.0, y as f32 / 128.0, 4, 3, c.seed);
            ramp(&p, n + if lx == 5 || ly == 5 { 0.35 } else { 0.0 }, x, y)
        }
    })
}

fn floor_tile(c: &Ctx) -> Image {
    let p = pal(&[0xa8b4be, 0xc4ced6, 0xdce4ea, 0xf0f4f6]);
    Image::from_fn(c.w, c.h, 2, |x, y| {
        let (cx, cy) = (x / 32, y / 32);
        let grout = x % 32 == 0 || y % 32 == 0;
        if grout {
            return hex(0x6a7480);
        }
        let t = hash(cx as i32, cy as i32, 2, c.seed) * 0.6
            + fbm2(x as f32 / 128.0, y as f32 / 128.0, 4, 3, c.seed) * 0.4;
        ramp(&p, t, x, y)
    })
}

fn floor_hazard(c: &Ctx) -> Image {
    let mut img = Image::filled(c.w, c.h, hex(0x1a1d22));
    hazard(&mut img, 0, c.h as i32, 32, hex(0xe8a818), hex(0x1a1d22));
    for y in 0..c.h as i32 {
        for x in 0..c.w as i32 {
            if hash(x, y, 9, c.seed) > 0.93 {
                img.put(x, y, [0.0, 0.0, 0.0, 0.25]);
            }
        }
    }
    img
}

fn ceiling(c: &Ctx, slit: bool) -> Image {
    let p = pal(&[0x2a2f36, 0x3a4048, 0x4c545e, 0x6a7480]);
    let mut img = plates(c, &p, 2, 2, 2, hex(0x14171b), hex(0x7a8794));
    let (w, h) = (c.w as i32, c.h as i32);
    if slit {
        img.rect(w / 4, h / 2 - 5, w / 2, 10, hex(0x10161c));
        img.rect(w / 4 + 2, h / 2 - 3, w / 2 - 4, 6, hex(0xf4fbff));
        img.rect(w / 4 + 2, h / 2 - 3, w / 2 - 4, 1, hex(0xcdeaf2));
    } else {
        hex_vent(&mut img, w / 2, h / 2, 30, hex(0x0a0c0e), hex(0x4c545e));
    }
    img
}

fn door(c: &Ctx, base: [u32; 4], light: Option<u32>, bars: bool) -> Image {
    let p = pal(&base);
    let (w, h) = (c.w as i32, c.h as i32);
    let mut img = Image::from_fn(c.w, c.h, 1, |x, y| {
        ramp(
            &p,
            fbm2(x as f32 / c.w as f32, y as f32 / c.h as f32, 3, 3, c.seed) * 0.8
                + (x as f32 / c.w as f32 - 0.5).abs() * 0.4,
            x,
            y,
        )
    });
    img.frame(0, 0, w, h, hex(0x14171b));
    img.frame(2, 2, w - 4, h - 4, hex(base[0]));
    img.rect(w / 2 - 1, 2, 2, h - 4, hex(0x08090b)); // sliding seam
    if bars {
        for i in 0..6 {
            img.rect(10 + i * 18, 6, 6, h - 12, hex(0x0a0a0c));
            img.rect(10 + i * 18, 6, 2, h - 12, hex(0x6a6e74));
        }
    } else {
        for i in 0..3 {
            let y = 30 + i * 8;
            img.line(10, y + 6, w / 2 - 6, y, with_alpha(hex(0xf2b21a), 0.9));
            img.line(w - 10, y + 6, w / 2 + 6, y, with_alpha(hex(0xf2b21a), 0.9));
        }
    }
    hazard(&mut img, h - 22, 8, 4, hex(0xe8a818), hex(0x14171b));
    if let Some(l) = light {
        img.rect(w / 2 + 10, h / 2 - 8, 16, 26, hex(0x0a0c0e));
        img.rect(w / 2 + 13, h / 2 - 5, 10, 20, hex(l));
        img.rect(w / 2 + 14, h / 2 - 4, 3, 18, scale(hex(l), 1.4));
    }
    img
}

fn console(c: &Ctx, alt: bool) -> Image {
    let p = pal(&[0x14171b, 0x2a2f36, 0x3a4048]);
    let (w, _h) = (c.w as i32, c.h as i32);
    let mut img = Image::from_fn(c.w, c.h, 1, |x, y| {
        ramp(
            &p,
            fbm2(x as f32 / 128.0, y as f32 / 128.0, 4, 3, c.seed),
            x,
            y,
        )
    });
    img.rect(12, 12, w - 24, 52, hex(0x050a0c));
    for j in 0..12 {
        let len = (hash(j, 0, 0, c.seed) * 70.0) as i32 + 10;
        img.rect(
            18,
            16 + j * 4,
            len,
            2,
            if alt { hex(0xffb040) } else { hex(0x4cf0b0) },
        );
    }
    for i in 0..6 {
        for j in 0..2 {
            let on = hash(i, j, 1, c.seed) > 0.5;
            let col = [hex(0xe0443a), hex(0xf2b21a), hex(CYAN)][((i + j) % 3) as usize];
            img.rect(
                14 + i * 18,
                76 + j * 16,
                12,
                8,
                if on { col } else { scale(col, 0.35) },
            );
        }
    }
    img.frame(12, 12, w - 24, 52, hex(0x586470));
    img
}

fn terminal(c: &Ctx) -> Image {
    let mut img = Image::filled(c.w, c.h, hex(0x03120c));
    for row in 0..c.h as i32 / 6 {
        let len = (hash(row, 0, 0, c.seed) * 90.0) as i32 + 8;
        let mut x = 6;
        while x < len {
            let w = (hash(row, x, 1, c.seed) * 12.0) as i32 + 3;
            img.rect(x, row * 6 + 6, w.min(len - x), 3, hex(0x4cf0a0));
            x += w + 4;
        }
    }
    for y in (0..c.h as i32).step_by(2) {
        img.rect(0, y, c.w as i32, 1, [0.0, 0.0, 0.0, 0.25]);
    }
    img.frame(0, 0, c.w as i32, c.h as i32, hex(0x1a4a30));
    img
}

fn reactor_core(c: &Ctx) -> Image {
    let p = pal(&[0x04161c, 0x0a4a5a, 0x1aa0b8, 0x70e8ff, 0xf0ffff]);
    let m = c.w as f32 / 2.0;
    Image::from_fn(c.w, c.h, 2, |x, y| {
        let (dx, dy) = (x as f32 * 2.0 - m, y as f32 * 2.0 - m);
        let r = (dx * dx + dy * dy).sqrt() / m;
        let rings = 0.5 + 0.5 * (r * 28.0).sin();
        let n = noise2(x as f32 / 6.0, y as f32 / 6.0, 0, 0, c.seed);
        ramp(
            &p,
            (1.0 - r) * 1.1 + rings * 0.25 * (1.0 - r) + n * 0.15,
            x,
            y,
        )
    })
}

fn barrel(c: &Ctx) -> Image {
    let p = pal(&[0x6a2a1a, 0x8c3a22, 0xb5482a, 0xd46a3a]);
    Image::from_fn(c.w, c.h, 1, |x, y| {
        let t = x as f32 / c.w as f32;
        let shade = (t * std::f32::consts::PI).sin();
        let ring = y % 40 < 4 || y % 40 > 35;
        if ring {
            ramp(&pal(&[0x2a2c30, 0x4a4d52, 0x7a7e86]), shade, x, y)
        } else {
            ramp(
                &p,
                shade * 0.9 + noise2(x as f32 / 8.0, y as f32 / 20.0, 0, 0, c.seed) * 0.2,
                x,
                y,
            )
        }
    })
}

fn light(c: &Ctx, col: Col) -> Image {
    let (w, h) = (c.w as i32, c.h as i32);
    let mut img = Image::filled(c.w, c.h, scale(col, 0.4));
    img.rect(3, 3, w - 6, h - 6, scale(col, 1.0));
    img.rect(6, 6, w - 12, h - 12, mix(col, [1.0; 4], 0.7));
    img.frame(0, 0, w, h, [0.1, 0.1, 0.12, 1.0]);
    img
}

fn decal(c: &Ctx, kind: u32) -> Image {
    Image::from_fn(c.w, c.h, 1, |x, y| {
        let (dx, dy) = (x as f32 / 32.0 - 1.0, y as f32 / 32.0 - 1.0);
        let r = (dx * dx + dy * dy).sqrt();
        let n = noise2(x as f32 / 5.0, y as f32 / 5.0, 0, 0, c.seed);
        let a = ((1.0 - r) * 2.0 * (0.4 + n)).clamp(0.0, 1.0);
        match kind {
            0 => [0.03, 0.03, 0.03, a * 0.85],
            1 => {
                let spark = hash(x as i32, y as i32, 0, c.seed) > 0.96 && r < 0.9;
                if spark {
                    [1.0, 0.9, 0.4, 1.0]
                } else {
                    [0.05, 0.05, 0.05, a * 0.4]
                }
            }
            _ => [0.05, 0.03, 0.02, (a * 1.2).min(0.8)],
        }
    })
}

fn glass(c: &Ctx) -> Image {
    let mut img = Image::filled(c.w, c.h, [0.5, 0.8, 0.9, 0.12]);
    let m = c.w as i32 / 2;
    for k in 0..9 {
        let a = hash(k, 0, 0, c.seed) * std::f32::consts::TAU;
        let len = 30.0 + hash(k, 1, 0, c.seed) * 36.0;
        img.line(
            m,
            m,
            m + (a.cos() * len) as i32,
            m + (a.sin() * len) as i32,
            [0.9, 0.97, 1.0, 0.8],
        );
    }
    for k in 1..4 {
        img.disc(m as f32, m as f32, k as f32 * 12.0, [0.0; 4]);
    }
    img.frame(0, 0, c.w as i32, c.h as i32, [0.8, 0.9, 1.0, 0.6]);
    img
}

fn viewscreen(c: &Ctx) -> Image {
    let mut img = Image::filled(c.w, c.h, [0.0, 0.0, 0.02, 1.0]);
    for i in 0..120 {
        let (x, y) = (
            (hash(i, 0, 0, c.seed) * c.w as f32) as i32,
            (hash(i, 1, 0, c.seed) * c.h as f32) as i32,
        );
        let b = 0.4 + hash(i, 2, 0, c.seed) * 0.6;
        img.put(x, y, [b, b, b, 1.0]);
    }
    let p = pal(&[0x2a1410, 0x7a3520, 0xc2663a, 0xe8a070]);
    for y in 0..c.h as i32 {
        for x in 0..c.w as i32 {
            let (dx, dy) = (x as f32 - 150.0, y as f32 - 190.0);
            let r = (dx * dx + dy * dy).sqrt();
            if r < 110.0 {
                let lit = ((-dx * 0.7 - dy * 0.7) / 110.0 + 1.0) * 0.5;
                let n = noise2(x as f32 / 14.0, y as f32 / 14.0, 0, 0, c.seed);
                img.put(
                    x,
                    y,
                    ramp(&p, lit * 0.8 + n * 0.3 - 0.1, x as u32, y as u32),
                );
            }
        }
    }
    img.frame(0, 0, c.w as i32, c.h as i32, hex(0x586470));
    img
}

pub fn register(r: &mut Registry) {
    r.tile("wall_minion_a", 256, 256, |c| wall_minion(c, 0));
    r.tile("wall_minion_b", 256, 256, |c| wall_minion(c, 1));
    r.tile("wall_minion_c", 256, 256, |c| wall_minion(c, 2));
    r.tile("wall_viking_a", 256, 256, |c| wall_viking(c, false));
    r.tile("wall_viking_b", 256, 256, |c| wall_viking(c, true));
    r.tile("wall_reactor", 256, 256, wall_reactor);
    r.tile("floor_grate", 256, 256, floor_grate);
    r.tile("floor_tile", 256, 256, floor_tile);
    r.tile("floor_hazard", 256, 256, floor_hazard);
    r.tile("ceiling_a", 256, 256, |c| ceiling(c, true));
    r.tile("ceiling_b", 256, 256, |c| ceiling(c, false));
    r.add("door_minion", 128, 256, |c| {
        door(c, [0x6a7480, 0x8a96a2, 0xaab6c0, 0xc8d2da], None, false)
    });
    r.add("door_blast", 128, 256, |c| {
        door(c, [0x2a2c30, 0x3a3e44, 0x4a4e56, 0x5a5e68], None, false)
    });
    r.add("door_cell", 128, 256, |c| {
        door(c, [0x2a2a2e, 0x3a3a40, 0x4a4a52, 0x5a5a64], None, true)
    });
    r.add("door_lock_bronze", 128, 256, |c| {
        door(
            c,
            [0x6a7480, 0x8a96a2, 0xaab6c0, 0xc8d2da],
            Some(0xcd7f32),
            false,
        )
    });
    r.add("door_lock_silver", 128, 256, |c| {
        door(
            c,
            [0x6a7480, 0x8a96a2, 0xaab6c0, 0xc8d2da],
            Some(0xdde4ea),
            false,
        )
    });
    r.add("door_lock_gold", 128, 256, |c| {
        door(
            c,
            [0x6a7480, 0x8a96a2, 0xaab6c0, 0xc8d2da],
            Some(0xffc830),
            false,
        )
    });
    r.add("console_a", 128, 128, |c| console(c, false));
    r.add("console_b", 128, 128, |c| console(c, true));
    r.add("terminal_screen", 128, 128, terminal);
    r.add("reactor_core", 256, 256, reactor_core);
    r.add("crate_a", 128, 128, |c| {
        let mut img = Image::from_fn(c.w, c.h, 2, |x, y| {
            let p = pal(&[0x4a2e1a, 0x6a4426, 0x8a5c34, 0xa87444]);
            ramp(
                &p,
                noise2(x as f32 / 2.0, y as f32 / 24.0, 0, 0, c.seed),
                x,
                y,
            )
        });
        img.frame(0, 0, 128, 128, hex(0x2a1a10));
        img.frame(6, 6, 116, 116, hex(0x2a1a10));
        img.line(6, 6, 121, 121, hex(0x2a1a10));
        img
    });
    r.add("crate_b", 128, 128, |c| {
        let mut img = Image::from_fn(c.w, c.h, 2, |x, y| {
            let p = pal(&[0x2a3038, 0x3a424c, 0x4c5662]);
            ramp(
                &p,
                fbm2(x as f32 / 64.0, y as f32 / 64.0, 3, 3, c.seed),
                x,
                y,
            )
        });
        hazard(&mut img, 54, 14, 6, hex(0xe8a818), hex(0x1a1d22));
        img.frame(0, 0, 128, 128, hex(0x14171b));
        rune_strip(&mut img, 100, 2, hex(0xd8cdb4), c.seed);
        img
    });
    r.add("barrel", 128, 128, barrel);
    r.add("light_panel", 64, 64, |c| light(c, hex(0xe8f6ff)));
    r.add("light_red", 64, 64, |c| light(c, hex(0xff3a30)));
    r.add("decal_scorch", 64, 64, |c| decal(c, 0));
    r.add("decal_sparks", 64, 64, |c| decal(c, 1));
    r.add("decal_oil", 64, 64, |c| decal(c, 2));
    r.add("glass_cracked", 128, 128, glass);
    r.add("bridge_viewscreen", 256, 128, viewscreen);
}
