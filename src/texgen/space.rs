//! Space environment textures: sky, sun, planets, moons, asteroids, rings, effects sprites.

use std::f32::consts::{PI, TAU};

use glam::Vec3;

use super::image::{Col, Image, hex, mix, ramp, scale, with_alpha};
use super::noise::{fbm2, fbm3, hash, noise2, noise3, voronoi};
use super::{Ctx, Registry};

fn dir(u: f32, v: f32) -> (Vec3, f32) {
    let (lon, lat) = (u * TAU, (v - 0.5) * PI);
    (
        Vec3::new(lat.cos() * lon.cos(), lat.sin(), lat.cos() * lon.sin()),
        lat,
    )
}

/// Equirectangular globe: `f(direction, latitude, x, y)` per logical pixel.
fn globe(c: &Ctx, block: u32, f: impl Fn(Vec3, f32, u32, u32) -> Col) -> Image {
    Image::from_fn(c.w, c.h, block, |x, y| {
        let u = (x as f32 + 0.5) * block as f32 / c.w as f32;
        let v = (y as f32 + 0.5) * block as f32 / c.h as f32;
        let (d, lat) = dir(u, v);
        f(d, lat, x, y)
    })
}

/// Crater shading in `[-0.5, 0.35]`: dark bowl, bright rim (tileable, unit square).
fn craters(u: f32, v: f32, seed: u32) -> f32 {
    let mut s = 0.0;
    for (cells, r) in [(5, 0.38), (11, 0.33), (23, 0.30)] {
        let (d, _) = voronoi(u, v, cells, seed + cells as u32);
        if d < r {
            s -= (1.0 - d / r) * 0.45;
        } else if d < r + 0.09 {
            s += 0.30 * (1.0 - (d - r) / 0.09);
        }
    }
    s
}

fn rocky(c: &Ctx, pal: &[Col], crater_amt: f32, freq: f32) -> Image {
    globe(c, 2, |d, _, x, y| {
        let n = fbm3(d * freq, 5, c.seed);
        // sample craters on a sphere-ish unwrap (u,v from direction)
        let (u, v) = (
            (d.z.atan2(d.x) / TAU).rem_euclid(1.0),
            d.y.asin() / PI + 0.5,
        );
        let t = n * 0.75 + 0.2 + craters(u, v, c.seed) * crater_amt;
        ramp(pal, t, x, y)
    })
}

fn bands(c: &Ctx, pal: &[Col], freq: f32, warp: f32, spot: Option<(f32, f32, Col)>) -> Image {
    globe(c, 2, |d, lat, x, y| {
        let w = fbm3(d * 3.0, 4, c.seed) - 0.5;
        let t = 0.5 + 0.5 * (lat * freq + w * warp).sin();
        let t = t * 0.85 + fbm3(d * 8.0, 3, c.seed + 7) * 0.15;
        let mut col = ramp(pal, t, x, y);
        if let Some((slon, slat, scol)) = spot {
            let lon = d.z.atan2(d.x);
            let (dl, dp) = ((lon - slon + PI).rem_euclid(TAU) - PI, lat - slat);
            let r = (dl * dl / 0.09 + dp * dp / 0.02).sqrt();
            if r < 1.0 {
                col = mix(col, scol, ((1.0 - r) * 3.0).min(1.0));
            }
        }
        col
    })
}

fn pal(list: &[u32]) -> Vec<Col> {
    list.iter().map(|&h| hex(h)).collect()
}

fn mars(c: &Ctx) -> Image {
    let p = pal(&[0x2a1410, 0x4e2216, 0x7a3520, 0xa24a2a, 0xc2663a, 0xdb8a58]);
    globe(c, 2, |d, lat, x, y| {
        let n = fbm3(d * 2.6, 6, c.seed);
        let mut t = n * 1.1 - 0.05;
        // Valles Marineris: dark ridged scar along the equator
        let ridge = 1.0 - (noise3(d * 6.0, c.seed + 3) * 2.0 - 1.0).abs();
        if lat.abs() < 0.22 && d.z.atan2(d.x) > -1.4 && d.z.atan2(d.x) < 0.3 {
            t -= (ridge.powf(6.0) * 0.5) * (1.0 - lat.abs() / 0.22);
        }
        let cap = 1.18 + (fbm3(d * 5.0, 3, c.seed + 9) - 0.5) * 0.25;
        if lat.abs() > cap {
            return ramp(&pal(&[0xc8d4dc, 0xeef4f8]), n * 1.4 - 0.2, x, y);
        }
        ramp(&p, t, x, y)
    })
}

fn earth(c: &Ctx) -> Image {
    let ocean = pal(&[0x061a3a, 0x0a2e62, 0x124a8c, 0x2a74b8]);
    let land = pal(&[0x1e5a2a, 0x3a7a30, 0x6a8a3c, 0xa89a58, 0xc8b878]);
    let cloud = pal(&[0xdfe8f0, 0xffffff]);
    globe(c, 2, |d, lat, x, y| {
        let n = fbm3(d * 2.2, 6, c.seed);
        let mut col = if n > 0.5 {
            let h = ((n - 0.5) * 5.0).min(1.0);
            let dry = (lat.abs() - 0.5).abs() < 0.28;
            ramp(&land, h * 0.6 + if dry { 0.4 } else { 0.0 }, x, y)
        } else {
            ramp(&ocean, (n - 0.2) * 3.0, x, y)
        };
        if lat.abs() > 1.36 {
            col = hex(0xf2f6fa);
        }
        let cl = fbm3(d * 4.0 + Vec3::splat(9.0), 5, c.seed + 5);
        if cl > 0.66 {
            col = mix(col, ramp(&cloud, (cl - 0.66) * 5.0, x, y), 0.7);
        }
        col
    })
}

fn sun(c: &Ctx) -> Image {
    let p = pal(&[0xc03a08, 0xe8600c, 0xff8a1a, 0xffb52e, 0xffd95a, 0xfff2a8]);
    globe(c, 2, |d, _, x, y| {
        let g = fbm3(d * 7.0, 5, c.seed);
        let cells = noise3(d * 18.0, c.seed + 1);
        let spots = (fbm3(d * 3.0, 3, c.seed + 4) - 0.62).max(0.0) * 3.0;
        ramp(&p, g * 0.7 + cells * 0.35 - spots, x, y)
    })
}

fn sky_stars(c: &Ctx) -> Image {
    let mut img = Image::filled(c.w, c.h, [0.0, 0.0, 0.0, 1.0]);
    for (layer, (density, bright, size)) in [(0.010, 0.45, 1), (0.0035, 0.75, 1), (0.0009, 1.0, 2)]
        .into_iter()
        .enumerate()
    {
        for y in 0..c.h as i32 {
            let lat = ((y as f32 + 0.5) / c.h as f32 - 0.5) * PI;
            let k = lat.cos().max(0.05);
            for x in 0..c.w as i32 {
                if hash(x, y, layer as i32, c.seed) > density * k {
                    continue;
                }
                let t = hash(x, y, 9, c.seed);
                let tint = if t < 0.3 {
                    [0.7, 0.8, 1.0]
                } else if t > 0.8 {
                    [1.0, 0.8, 0.6]
                } else {
                    [1.0, 1.0, 1.0]
                };
                let b = bright * (0.6 + 0.4 * hash(x, y, 5, c.seed));
                let col = [tint[0] * b, tint[1] * b, tint[2] * b, 1.0];
                img.rect(x, y, size, size, col);
                if layer == 2 {
                    let glow = with_alpha(col, 0.45);
                    for (dx, dy) in [(-1, 0), (2, 0), (0, -1), (0, 2)] {
                        img.put(x + dx, y + dy, glow);
                    }
                }
            }
        }
    }
    img
}

fn sky_nebula(c: &Ctx) -> Image {
    let p = pal(&[0x1a0b3a, 0x3a1470, 0x6a2a9a, 0x1f6a8a, 0x35c8e0]);
    globe(c, 4, |d, _, x, y| {
        let a = fbm3(d * 1.6, 6, c.seed);
        let b = fbm3(d * 3.2 + Vec3::splat(5.0), 4, c.seed + 2);
        let dens = ((a - 0.46) * 3.2).clamp(0.0, 1.0);
        let col = ramp(&p, b * 1.1, x, y);
        with_alpha(col, dens * dens * 0.55)
    })
}

fn ring(c: &Ctx) -> Image {
    let p = pal(&[0x6a5a44, 0x8c7a5c, 0xb09c78, 0xd2c09a]);
    Image::from_fn(c.w, c.h, 1, |x, y| {
        let r = x as f32 / c.w as f32;
        let n =
            noise2(r * 90.0, 0.0, 0, 0, c.seed) * 0.6 + noise2(r * 23.0, 1.0, 0, 0, c.seed) * 0.4;
        let gap = (r - 0.62).abs() < 0.035;
        let a = if gap {
            0.05
        } else {
            (0.35 + n * 0.75).min(0.95)
        };
        let edge = ((r * (1.0 - r)) * 18.0).min(1.0);
        with_alpha(ramp(&p, n, x, y), a * edge)
    })
}

/// Soft radial sprite: `core` in the middle fading to transparent.
fn radial(c: &Ctx, inner: Col, outer: Col, power: f32) -> Image {
    Image::from_fn(c.w, c.h, 1, |x, y| {
        let (dx, dy) = (
            (x as f32 + 0.5) / c.w as f32 * 2.0 - 1.0,
            (y as f32 + 0.5) / c.h as f32 * 2.0 - 1.0,
        );
        let r = (dx * dx + dy * dy).sqrt().min(1.0);
        let t = (1.0 - r).powf(power);
        let col = mix(outer, inner, t);
        with_alpha(col, t)
    })
}

fn explosion(c: &Ctx) -> Image {
    let mut img = Image::new(c.w, c.h);
    let cell = c.w / 4;
    let p = pal(&[0x1a0a08, 0x5a1408, 0xc0300c, 0xff7a14, 0xffc03a, 0xfff6c8]);
    for f in 0..16u32 {
        let (ox, oy) = ((f % 4 * cell) as i32, (f / 4 * cell) as i32);
        let life = f as f32 / 15.0;
        let radius = 0.18 + 0.78 * life.sqrt();
        for y in 0..cell {
            for x in 0..cell {
                let (dx, dy) = (
                    (x as f32 + 0.5) / cell as f32 * 2.0 - 1.0,
                    (y as f32 + 0.5) / cell as f32 * 2.0 - 1.0,
                );
                let a = dy.atan2(dx);
                let wob = noise2(
                    a.cos() * 2.0 + 20.0,
                    a.sin() * 2.0 + 20.0 + f as f32 * 0.3,
                    0,
                    0,
                    c.seed,
                ) * 0.35;
                let r = (dx * dx + dy * dy).sqrt() / (radius * (0.75 + wob));
                if r > 1.0 {
                    continue;
                }
                let n = noise2(
                    x as f32 / 9.0,
                    y as f32 / 9.0 + f as f32 * 3.1,
                    0,
                    0,
                    c.seed,
                );
                let heat = (1.0 - r) * (1.2 - life) + n * 0.25 - life * 0.25;
                let col = ramp(&p, heat * 1.3, x, y);
                let alpha = ((1.0 - r) * 4.0).min(1.0) * (1.0 - life * life);
                img.put(ox + x as i32, oy + y as i32, with_alpha(col, alpha));
            }
        }
    }
    img
}

fn beam(c: &Ctx, core_w: f32) -> Image {
    Image::from_fn(c.w, c.h, 1, |x, y| {
        let v = ((y as f32 + 0.5) / c.h as f32 * 2.0 - 1.0).abs();
        let end = ((x as f32 / c.w as f32).min(1.0 - x as f32 / c.w as f32) * 8.0).min(1.0);
        if v < core_w {
            [1.0, 1.0, 1.0, end]
        } else {
            let t = (1.0 - v).powf(1.5);
            [0.25 + 0.3 * (1.0 - t), 0.85, 1.0, t * end]
        }
    })
}

fn star_sprite(c: &Ctx, col: Col, arms: f32) -> Image {
    Image::from_fn(c.w, c.h, 1, |x, y| {
        let (dx, dy) = (
            (x as f32 + 0.5) / c.w as f32 * 2.0 - 1.0,
            (y as f32 + 0.5) / c.h as f32 * 2.0 - 1.0,
        );
        let r = (dx * dx + dy * dy).sqrt();
        let cross = (1.0 - dx.abs().min(dy.abs()) * 8.0).max(0.0) * (1.0 - r).max(0.0);
        let core = (1.0 - r * 2.2).max(0.0);
        let a = (core + cross * arms).min(1.0);
        with_alpha(mix(col, [1.0, 1.0, 1.0, 1.0], core), a)
    })
}

fn shield_hit(c: &Ctx) -> Image {
    Image::from_fn(c.w, c.h, 1, |x, y| {
        let (dx, dy) = (
            (x as f32 + 0.5) / c.w as f32 * 2.0 - 1.0,
            (y as f32 + 0.5) / c.h as f32 * 2.0 - 1.0,
        );
        let r = (dx * dx + dy * dy).sqrt();
        let ringv = (1.0 - ((r - 0.7).abs() * 10.0)).max(0.0);
        let fill = (1.0 - r).max(0.0) * 0.5;
        let a = (ringv + fill).min(1.0) * (r < 1.0) as u32 as f32;
        [0.4, 0.85, 1.0, a]
    })
}

fn hex_grid(c: &Ctx) -> Image {
    Image::from_fn(c.w, c.h, 1, |x, y| {
        let s = 16.0;
        let (px, py) = (x as f32 / s, y as f32 / (s * 0.866));
        // axial hex distance to nearest centre
        let (qx, qy) = (px - 0.5 * (py.floor() as i32 & 1) as f32, py);
        let (fx, fy) = (qx - qx.floor() - 0.5, qy - qy.floor() - 0.5);
        let e = fx.abs().max(fy.abs() * 1.15);
        let edge = ((e - 0.38) * 14.0).clamp(0.0, 1.0);
        let n = noise2(x as f32 / 24.0, y as f32 / 24.0, 0, 0, c.seed);
        [0.3, 0.8, 1.0, edge * (0.45 + 0.4 * n) + 0.04]
    })
}

pub fn register(r: &mut Registry) {
    r.add("sky_stars", 1024, 512, sky_stars);
    r.add("sky_nebula", 1024, 512, sky_nebula);
    r.add("sun", 512, 512, sun);
    r.add("mercury", 512, 512, |c| {
        rocky(
            c,
            &pal(&[0x2c2b2a, 0x4a4845, 0x6e6a64, 0x948e84, 0xb8b0a0]),
            1.0,
            3.0,
        )
    });
    r.add("venus", 512, 512, |c| {
        bands(
            c,
            &pal(&[0x8a5a22, 0xb88438, 0xd8a850, 0xecc878, 0xf6e0a0]),
            5.0,
            6.0,
            None,
        )
    });
    r.add("jupiter", 512, 512, |c| {
        let spot = Some((1.2, -0.3, hex(0xc04a2a)));
        bands(
            c,
            &pal(&[0x7a5a3c, 0xa07a58, 0xc8a478, 0xdcc096, 0xe8d6b4]),
            16.0,
            3.5,
            spot,
        )
    });
    r.add("saturn", 512, 512, |c| {
        bands(
            c,
            &pal(&[0x8a7448, 0xb09a68, 0xd0bc88, 0xe8d8a8, 0xf4ecc8]),
            16.0,
            1.2,
            None,
        )
    });
    r.add("uranus", 512, 512, |c| {
        bands(
            c,
            &pal(&[0x5aa6b4, 0x78c0cc, 0x98d8e0, 0xb8e8ee]),
            8.0,
            0.8,
            None,
        )
    });
    r.add("neptune", 512, 512, |c| {
        let spot = Some((-0.6, 0.35, hex(0x0a1c5a)));
        bands(
            c,
            &pal(&[0x14307a, 0x1f4aa8, 0x3a6ed0, 0x6a98e8, 0xa8c4f8]),
            11.0,
            3.0,
            spot,
        )
    });
    r.add("mars", 512, 512, mars);
    r.add("earth", 512, 512, earth);
    r.add("luna", 512, 512, |c| {
        rocky(
            c,
            &pal(&[0x2a2a2c, 0x4a4a4e, 0x707074, 0x9a9a9e, 0xc4c4c8]),
            1.2,
            2.4,
        )
    });
    r.add("phobos", 256, 256, |c| {
        rocky(c, &pal(&[0x2a2420, 0x453a32, 0x66584a, 0x8a7a68]), 1.0, 4.0)
    });
    r.add("deimos", 256, 256, |c| {
        rocky(c, &pal(&[0x36302c, 0x544a42, 0x7a6e60, 0xa09484]), 0.7, 3.0)
    });
    r.tile("asteroid_a", 256, 256, |c| {
        asteroid(c, &[0x2a2a2c, 0x4a4a4c, 0x6e6c6a, 0x908e88], 0xe8b830)
    });
    r.tile("asteroid_b", 256, 256, |c| {
        asteroid(c, &[0x2c2018, 0x4a3828, 0x6e5238, 0x947456], 0x35c8e0)
    });
    r.tile("asteroid_c", 256, 256, |c| {
        asteroid(c, &[0x40586a, 0x6a8aa0, 0xa0c0d4, 0xe0f0fa], 0xffffff)
    });
    r.add("ring_saturn", 512, 64, ring);
    r.add("glow_soft", 128, 128, |c| {
        radial(c, [1.0, 1.0, 1.0, 1.0], [1.0, 1.0, 1.0, 1.0], 2.0)
    });
    r.add("glow_engine", 128, 128, |c| {
        radial(c, [0.8, 0.95, 1.0, 1.0], [0.1, 0.5, 1.0, 1.0], 1.6)
    });
    r.add("explosion_sheet", 512, 512, explosion);
    r.add("beam_lance", 64, 16, |c| beam(c, 0.18));
    r.add("beam_core", 64, 16, |c| beam(c, 0.5));
    r.add("torpedo", 64, 64, |c| {
        radial(c, [0.9, 0.95, 1.0, 1.0], [0.2, 0.5, 1.0, 1.0], 1.2)
    });
    r.add("missile_trail", 64, 64, |c| {
        Image::from_fn(c.w, c.h, 1, |x, y| {
            let (dx, dy) = ((x as f32 / 32.0 - 1.0), (y as f32 / 32.0 - 1.0));
            let r = (dx * dx + dy * dy).sqrt();
            let n = noise2(x as f32 / 8.0, y as f32 / 8.0, 0, 0, c.seed);
            let a = ((1.0 - r) * (0.5 + n)).clamp(0.0, 1.0) * 0.8;
            scale([0.8, 0.8, 0.85, a], 0.8 + 0.2 * n)
        })
    });
    r.add("spark", 64, 64, |c| {
        star_sprite(c, [1.0, 0.85, 0.4, 1.0], 1.0)
    });
    r.add("shield_hit", 64, 64, shield_hit);
    r.add("shield_bubble", 256, 256, hex_grid);
}

fn asteroid(c: &Ctx, rock: &[u32], fleck: u32) -> Image {
    let p = pal(rock);
    let fleck = hex(fleck);
    Image::from_fn(c.w, c.h, 2, |x, y| {
        let (u, v) = ((x * 2) as f32 / c.w as f32, (y * 2) as f32 / c.h as f32);
        let n = fbm2(u, v, 4, 5, c.seed);
        let t = n * 0.9 + craters(u, v, c.seed) * 0.8 - 0.05;
        let col = ramp(&p, t + 0.1, x, y);
        if hash(x as i32, y as i32, 3, c.seed) > 0.985 && n > 0.45 {
            fleck
        } else {
            col
        }
    })
}
