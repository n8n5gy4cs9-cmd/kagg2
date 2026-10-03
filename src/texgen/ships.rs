//! Ship, station and cargo hull textures, plus shared plate/rivet/stripe helpers.

use super::image::{Col, Image, hex, mix, ramp, scale, with_alpha};
use super::noise::{fbm2, hash, noise2, voronoi};
use super::{Ctx, Registry};

pub fn pal(list: &[u32]) -> Vec<Col> {
    list.iter().map(|&h| hex(h)).collect()
}

/// Tileable riveted plate field: `cols`×`rows` plates with per-plate tone, seams, wear.
pub fn plates(
    c: &Ctx,
    p: &[Col],
    cols: u32,
    rows: u32,
    block: u32,
    seam: Col,
    rivet: Col,
) -> Image {
    let (pw, ph) = ((c.w / block) / cols, (c.h / block) / rows);
    let mut img = Image::from_fn(c.w, c.h, block, |x, y| {
        let (cx, cy) = (x / pw, y / ph);
        let (lx, ly) = (x % pw, y % ph);
        let tone = hash(cx as i32, cy as i32, 1, c.seed) * 0.5 + 0.25;
        let (u, v) = (
            (x * block) as f32 / c.w as f32,
            (y * block) as f32 / c.h as f32,
        );
        let wear = fbm2(u, v, 6, 4, c.seed) - 0.5;
        // light from the top-left of each plate gives a bevel
        let bevel = if lx <= 1 || ly <= 1 {
            0.18
        } else if lx >= pw - 2 || ly >= ph - 2 {
            -0.18
        } else {
            0.0
        };
        let scratch = if hash((x / 3) as i32, (y * 7) as i32, 4, c.seed) > 0.992 {
            0.2
        } else {
            0.0
        };
        let col = ramp(p, tone + wear * 0.5 + bevel + scratch, x, y);
        if lx == 0 || ly == 0 { seam } else { col }
    });
    let b = block as i32;
    for cy in 0..rows as i32 {
        for cx in 0..cols as i32 {
            for (dx, dy) in [
                (3, 3),
                (pw as i32 - 4, 3),
                (3, ph as i32 - 4),
                (pw as i32 - 4, ph as i32 - 4),
            ] {
                let (x, y) = ((cx * pw as i32 + dx) * b, (cy * ph as i32 + dy) * b);
                img.rect(x, y, b, b, rivet);
                img.rect(x + b, y + b, b, b, scale(seam, 0.8));
            }
        }
    }
    img
}

/// Diagonal hazard stripes over rows `y..y+h` (works at any pixel scale).
pub fn hazard(img: &mut Image, y: i32, h: i32, size: i32, a: Col, b: Col) {
    for j in y..y + h {
        for i in 0..img.w as i32 {
            img.put(i, j, if ((i + j) / size) % 2 == 0 { a } else { b });
        }
    }
}

/// Row of tiny rune-like glyphs (random strokes in a 5×7 cell).
pub fn rune_strip(img: &mut Image, y: i32, cell: i32, col: Col, seed: u32) {
    let n = img.w as i32 / (cell * 6);
    for i in 0..n {
        let x0 = i * cell * 6 + cell;
        let hseed = |k: i32| hash(i, k, 7, seed);
        img.rect(x0 + 2 * cell, y, cell, 7 * cell, col); // staff
        for k in 0..3 {
            let (a, b) = (hseed(k * 2), hseed(k * 2 + 1));
            let yy = y + (a * 5.0) as i32 * cell;
            if b > 0.5 {
                img.rect(x0 + 3 * cell, yy, cell, cell, col);
                img.rect(x0 + 4 * cell, yy + cell, cell, cell, col);
            } else {
                img.rect(x0 + cell, yy, cell, cell, col);
                img.rect(x0, yy + cell, cell, cell, col);
            }
        }
    }
}

pub fn hex_vent(img: &mut Image, cx: i32, cy: i32, r: i32, dark: Col, edge: Col) {
    for y in -r..=r {
        for x in -r..=r {
            let hx = x.abs() as f32 + y.abs() as f32 * 0.5;
            if hx <= r as f32 && y.abs() <= r * 4 / 5 {
                let slat = (y + r) % 3 == 0;
                img.put(cx + x, cy + y, if slat { edge } else { dark });
            }
        }
    }
}

fn viking(c: &Ctx, alt: bool) -> Image {
    let p = if alt {
        pal(&[0x2a2c30, 0x4a4d52, 0x6a6e74, 0x8a4a2a, 0xb5482a])
    } else {
        pal(&[0x3a1a12, 0x6a2a1a, 0x8c3a22, 0xb5482a, 0xd46a3a])
    };
    let mut img = plates(c, &p, 4, 4, 2, hex(0x1a1010), hex(0xd8cdb4));
    let b = 2;
    rune_strip(
        &mut img,
        (c.h as i32 / 4) * 2 - 18 * b / 2,
        b,
        with_alpha(hex(0xd8cdb4), 0.9),
        c.seed,
    );
    img
}

fn minion(c: &Ctx, alt: bool) -> Image {
    let p = pal(&[0x9aa6b0, 0xc4ced6, 0xe0e8ee, 0xf2f6f8]);
    let mut img = plates(c, &p, 4, 4, 2, hex(0x586470), hex(0x7a8794));
    let cyan = hex(0x35c8e0);
    let step = c.h as i32 / 4;
    for i in 0..4 {
        let y = i * step + step / 2;
        img.rect(8, y, c.w as i32 - 16, 3, scale(cyan, 0.6));
        img.rect(8, y + 1, c.w as i32 - 16, 1, cyan);
    }
    let (vx, vy) = if alt {
        (step * 2, step)
    } else {
        (step, step * 2)
    };
    hex_vent(&mut img, vx + 20, vy + 20, 14, hex(0x2a2f36), hex(0x586470));
    img
}

fn minion_dark(c: &Ctx) -> Image {
    let p = pal(&[0x1a1d22, 0x2a2f36, 0x3a4048, 0x4c545e]);
    let mut img = plates(c, &p, 2, 2, 2, hex(0x0e1013), hex(0x6a7480));
    let h = c.h as i32;
    hazard(&mut img, h / 2 - 8, 16, 8, hex(0xf2b21a), hex(0x1a1d22));
    img
}

fn station_forge(c: &Ctx) -> Image {
    let p = pal(&[0x24201e, 0x3c3632, 0x58504a, 0x786c60, 0x8a4a2a]);
    let mut img = plates(c, &p, 4, 4, 2, hex(0x12100e), hex(0xb08a4a));
    // furnace windows
    let glow = [hex(0xff8a2b), hex(0xffc060), hex(0xfff0b0)];
    for k in 0..6 {
        let (x, y) = (24 + (k % 3) * 160, 40 + (k / 3) * 220);
        img.rect(x, y, 40, 22, hex(0x120a06));
        for j in 0..18 {
            for i in 0..36 {
                let n = noise2(
                    i as f32 / 5.0,
                    j as f32 / 5.0 + k as f32 * 3.0,
                    0,
                    0,
                    c.seed,
                );
                img.put(
                    x + 2 + i,
                    y + 2 + j,
                    ramp(
                        &glow,
                        n * 1.2 - 0.1 + (18 - j) as f32 / 40.0,
                        i as u32,
                        j as u32,
                    ),
                );
            }
        }
    }
    img
}

fn station_fort(c: &Ctx) -> Image {
    let p = pal(&[0x6a7480, 0x8a96a2, 0xaab4be, 0xc8d0d8]);
    let mut img = plates(c, &p, 4, 4, 2, hex(0x2a3038), hex(0x3a4048));
    // turret decals: ring + barrel
    for (cx, cy) in [(128, 128), (384, 384)] {
        img.disc(cx as f32, cy as f32, 30.0, hex(0x2a2f36));
        img.disc(cx as f32, cy as f32, 22.0, hex(0x586470));
        img.rect(cx - 3, cy - 36, 6, 30, hex(0x1a1d22));
        img.disc(cx as f32, cy as f32, 7.0, hex(0xe0443a));
    }
    img
}

fn crate_tex(c: &Ctx, base: [u32; 4], stencil: u32, n: u32) -> Image {
    let p = pal(&base);
    let mut img = Image::from_fn(c.w, c.h, 2, |x, y| {
        let t = fbm2(
            x as f32 * 2.0 / c.w as f32,
            y as f32 * 2.0 / c.h as f32,
            3,
            3,
            c.seed,
        ) * 0.6
            + 0.3;
        ramp(&p, t, x, y)
    });
    let (w, h) = (c.w as i32, c.h as i32);
    img.frame(0, 0, w, h, scale(hex(base[0]), 0.7));
    img.frame(4, 4, w - 8, h - 8, hex(base[1]));
    img.line(4, 4, w - 5, h - 5, hex(base[0]));
    img.line(w - 5, 4, 4, h - 5, hex(base[0]));
    let s = hex(stencil);
    img.rect(w / 2 - 18, h / 2 - 8, 36, 4, s);
    img.rect(w / 2 - 18, h / 2 + 4, 24, 4, s);
    img.rect(w - 24, h - 14 - n as i32, 12, 3 + n as i32, s);
    img
}

pub fn register(r: &mut Registry) {
    r.tile("hull_viking_a", 512, 512, |c| viking(c, false));
    r.tile("hull_viking_b", 512, 512, |c| viking(c, true));
    r.tile("hull_viking_trim", 256, 256, |c| {
        let p = pal(&[0x6a5a3a, 0x9a8450, 0xc8b878, 0xd8cdb4]);
        let mut img = plates(c, &p, 4, 2, 2, hex(0x2a2010), hex(0xf0e0a0));
        rune_strip(&mut img, 56, 2, hex(0x3a2a14), c.seed);
        img
    });
    r.add("sail_viking", 256, 256, |c| {
        let cloth = pal(&[0x7a2a1a, 0x9a3820, 0xb5482a, 0xc85a34]);
        let mut img = Image::from_fn(c.w, c.h, 2, |x, y| {
            let weave = if (x + y) % 3 == 0 { 0.1 } else { 0.0 };
            ramp(
                &cloth,
                fbm2(x as f32 / 128.0, y as f32 / 128.0, 4, 3, c.seed) + weave,
                x,
                y,
            )
        });
        // raven rune: stylised bird silhouette from rects
        let k = hex(0x181010);
        for (x, y, w, h) in [
            (96, 80, 60, 40),
            (80, 100, 40, 20),
            (140, 70, 40, 14),
            (176, 66, 16, 10),
            (110, 120, 12, 50),
            (130, 120, 12, 44),
            (70, 90, 30, 10),
        ] {
            img.rect(x, y, w, h, k);
        }
        img.rect(172, 68, 4, 4, hex(0xf2b21a));
        img.frame(6, 6, 244, 244, hex(0xd8cdb4));
        img
    });
    r.tile("hull_minion_a", 512, 512, |c| minion(c, false));
    r.tile("hull_minion_b", 512, 512, |c| minion(c, true));
    r.tile("hull_minion_dark", 256, 256, minion_dark);
    r.add("engine_glow", 64, 64, |c| {
        Image::from_fn(c.w, c.h, 1, |x, y| {
            let (dx, dy) = (x as f32 / 32.0 - 1.0, y as f32 / 32.0 - 1.0);
            let t = (1.0 - (dx * dx + dy * dy).sqrt()).max(0.0);
            [0.4 + 0.6 * t, 0.7 + 0.3 * t, 1.0, t * t * 1.5]
        })
    });
    r.tile("station_forge", 512, 512, station_forge);
    r.tile("station_fort", 512, 512, station_fort);
    r.add("cargo_container_a", 128, 128, |c| {
        crate_tex(c, [0x6a2a1a, 0x8c3a22, 0xb5482a, 0xc85a34], 0xd8cdb4, 0)
    });
    r.add("cargo_container_b", 128, 128, |c| {
        crate_tex(c, [0x6a5020, 0x8c6a2a, 0xb08a38, 0xc8a050], 0x2a2010, 3)
    });
    r.tile("window_lights", 128, 128, |c| {
        Image::from_fn(c.w, c.h, 1, |x, y| {
            let (cx, cy) = (x / 8, y / 8);
            if x % 8 < 5 && y % 8 < 3 && hash(cx as i32, cy as i32, 0, c.seed) > 0.45 {
                let warm = hash(cx as i32, cy as i32, 1, c.seed) > 0.3;
                if warm { hex(0xffc860) } else { hex(0x9ee8ff) }
            } else {
                let (d, _) = voronoi(x as f32 / 128.0, y as f32 / 128.0, 4, c.seed);
                mix(hex(0x0c0e12), hex(0x181c22), d)
            }
        })
    });
}
