//! Character skins, pickups, first-person weapon textures and combat effect sprites.

use super::image::{Col, Image, hex, mix, ramp, scale, with_alpha};
use super::noise::{fbm2, hash, noise2};
use super::ships::{hazard, pal};
use super::{Ctx, Registry};

/// Body-part skin: noisy base ramp, panel seams, one coloured trim stripe.
fn skin(c: &Ctx, base: &[u32], trim: u32, seam: u32) -> Image {
    let p = pal(base);
    let (w, h) = (c.w as i32, c.h as i32);
    let mut img = Image::from_fn(c.w, c.h, 1, |x, y| {
        let t = fbm2(x as f32 / c.w as f32, y as f32 / c.h as f32, 3, 3, c.seed) * 0.7
            + (1.0 - y as f32 / c.h as f32) * 0.3;
        ramp(&p, t, x, y)
    });
    img.rect(0, h / 2, w, 1, hex(seam));
    img.rect(w / 2, 0, 1, h / 2, hex(seam));
    img.rect(0, h / 4, w, (h / 16).max(2), hex(trim));
    img.frame(0, 0, w, h, hex(seam));
    img
}

fn visor(c: &Ctx, col: u32) -> Image {
    let (w, h) = (c.w as i32, c.h as i32);
    let mut img = Image::from_fn(c.w, c.h, 1, |_x, y| {
        mix(
            hex(0x03181c),
            hex(col),
            (1.0 - y as f32 / c.h as f32).powi(2) * 0.9,
        )
    });
    for y in (0..h).step_by(3) {
        img.rect(0, y, w, 1, [0.0, 0.0, 0.0, 0.3]);
    }
    img.rect(w / 6, h / 5, w / 3, (h / 12).max(1), [1.0, 1.0, 1.0, 0.6]);
    img
}

fn cross(img: &mut Image, cx: i32, cy: i32, s: i32, col: Col) {
    img.rect(cx - s, cy - s / 3, s * 2, s * 2 / 3 + 1, col);
    img.rect(cx - s / 3, cy - s, s * 2 / 3 + 1, s * 2, col);
}

fn sprite(c: &Ctx, f: impl Fn(&mut Image)) -> Image {
    let mut img = Image::new(c.w, c.h);
    f(&mut img);
    img.outline(hex(0x0a0a0c));
    img
}

/// Vertical-gradient bar (light on top) — the building block of weapon sprites.
fn bar(img: &mut Image, x: i32, y: i32, w: i32, h: i32, shades: [u32; 3]) {
    for j in 0..h {
        let t = j as f32 / h.max(1) as f32;
        let col = if t < 0.3 {
            shades[0]
        } else if t < 0.75 {
            shades[1]
        } else {
            shades[2]
        };
        img.rect(x, y + j, w, 1, hex(col));
    }
}

const STEEL: [u32; 3] = [0x9aa0aa, 0x6a707a, 0x3a4048];
const DARK: [u32; 3] = [0x4a4e56, 0x2a2e34, 0x14161a];
const WOOD: [u32; 3] = [0xa87444, 0x7a5030, 0x4a2e1a];
const GOLD: [u32; 3] = [0xe8c860, 0xb08a38, 0x6a5020];

/// 64×64 pixel-art weapon, drawn then upscaled to 256².
fn weapon(c: &Ctx, draw: impl Fn(&mut Image)) -> Image {
    let mut small = Image::new(64, 64);
    draw(&mut small);
    small.outline(hex(0x08090b));
    small.upscale(c.w / 64)
}

fn draw_axe(i: &mut Image) {
    bar(i, 30, 16, 5, 44, WOOD);
    bar(i, 18, 14, 22, 14, STEEL);
    bar(i, 12, 18, 8, 10, STEEL);
    bar(i, 38, 17, 6, 8, STEEL);
    i.rect(14, 24, 24, 2, hex(0xe8f0f8));
    bar(i, 28, 52, 9, 6, GOLD);
}

fn draw_blaster(i: &mut Image) {
    bar(i, 8, 26, 36, 12, DARK);
    bar(i, 40, 28, 16, 8, STEEL);
    bar(i, 10, 36, 12, 20, DARK);
    i.rect(24, 28, 14, 3, hex(0x35c8e0));
    i.rect(54, 29, 4, 6, hex(0x9ef4ff));
    bar(i, 20, 22, 8, 5, STEEL);
}

fn draw_shotgun(i: &mut Image) {
    bar(i, 4, 28, 20, 10, WOOD);
    bar(i, 22, 26, 38, 6, STEEL);
    bar(i, 22, 33, 38, 5, STEEL);
    bar(i, 30, 36, 12, 5, WOOD);
    bar(i, 20, 24, 6, 16, GOLD);
    i.rect(58, 27, 2, 4, hex(0x08090b));
    i.rect(58, 34, 2, 3, hex(0x08090b));
}

fn draw_rattler(i: &mut Image) {
    bar(i, 6, 26, 40, 11, DARK);
    bar(i, 44, 28, 14, 7, STEEL);
    bar(i, 20, 36, 12, 16, DARK);
    bar(i, 34, 36, 8, 10, GOLD);
    bar(i, 14, 20, 20, 6, STEEL);
    i.rect(16, 28, 20, 2, hex(0xff8a2b));
    for k in 0..4 {
        i.rect(46 + k * 3, 29, 1, 5, hex(0x08090b));
    }
}

fn draw_mjolnir(i: &mut Image) {
    bar(i, 6, 22, 44, 18, STEEL);
    bar(i, 46, 18, 14, 26, DARK);
    bar(i, 8, 26, 10, 10, GOLD);
    bar(i, 14, 40, 10, 16, WOOD);
    i.rect(22, 26, 20, 3, hex(0xff8a2b));
    i.rect(52, 24, 6, 14, hex(0xff5a1a));
}

fn draw_valkyrie(i: &mut Image) {
    bar(i, 4, 28, 54, 7, STEEL);
    bar(i, 8, 24, 30, 4, DARK);
    bar(i, 8, 36, 20, 12, DARK);
    for k in 0..5 {
        i.rect(14 + k * 8, 25, 3, 12, hex(0x35c8e0));
    }
    bar(i, 54, 26, 8, 11, GOLD);
    i.rect(58, 30, 6, 3, hex(0xd8f8ff));
}

fn pickup_box(img: &mut Image, col: u32, label: u32) {
    bar(
        img,
        14,
        22,
        36,
        28,
        [
            col,
            scale(hex(col), 0.75).map_hex(),
            scale(hex(col), 0.5).map_hex(),
        ],
    );
    img.rect(14, 32, 36, 4, hex(label));
}

trait MapHex {
    fn map_hex(self) -> u32;
}

impl MapHex for Col {
    fn map_hex(self) -> u32 {
        let b = |v: f32| (v.clamp(0.0, 1.0) * 255.0) as u32;
        b(self[0]) << 16 | b(self[1]) << 8 | b(self[2])
    }
}

fn key_card(img: &mut Image, shades: [u32; 3]) {
    bar(img, 10, 18, 44, 28, shades);
    img.rect(14, 22, 12, 8, hex(0x14171b));
    img.rect(14, 34, 36, 3, hex(0x14171b));
    img.rect(14, 40, 20, 2, hex(0x14171b));
    img.rect(40, 22, 10, 8, scale(hex(shades[0]), 1.2));
}

fn flash(c: &Ctx) -> Image {
    Image::from_fn(c.w, c.h, 1, |x, y| {
        let (dx, dy) = (x as f32 / 32.0 - 1.0, y as f32 / 32.0 - 1.0);
        let r = (dx * dx + dy * dy).sqrt();
        let a = dy.atan2(dx);
        let spikes = 0.45 + 0.55 * (a * 5.0).cos().abs().powf(3.0);
        let t = (1.0 - r / spikes).max(0.0);
        let p = [hex(0xff8a1a), hex(0xffc83a), hex(0xfff6c8)];
        with_alpha(ramp(&p, t * 1.5, x, y), (t * 3.0).min(1.0))
    })
}

pub fn register(r: &mut Registry) {
    // Minions: white/cyan Concord armour, one skin per body part
    r.add("minion_armor", 64, 64, |c| {
        skin(
            c,
            &[0x9aa6b0, 0xc4ced6, 0xe0e8ee, 0xf2f6f8],
            0x35c8e0,
            0x586470,
        )
    });
    r.add("minion_visor", 64, 64, |c| visor(c, 0x35c8e0));
    r.add("minion_heavy", 128, 128, |c| {
        skin(
            c,
            &[0x1a1d22, 0x2a2f36, 0x3a4048, 0x4c545e],
            0xf2a21a,
            0x0e1013,
        )
    });
    r.add("minion_medic", 128, 128, |c| {
        let mut img = skin(c, &[0xc4ced6, 0xe0e8ee, 0xf2f6f8], 0x3acf7a, 0x586470);
        cross(&mut img, 64, 80, 16, hex(0x3acf7a));
        img
    });
    r.add("minion_enforcer", 128, 128, |c| {
        skin(
            c,
            &[0x3a0e0e, 0x5a1a1a, 0x7a2626, 0x9a3a32],
            0xe0b030,
            0x1a0606,
        )
    });
    r.add("minion_mech", 128, 128, |c| {
        let mut img = skin(
            c,
            &[0x3a4048, 0x586470, 0x7a8794, 0x9aa6b2],
            0xf2b21a,
            0x14171b,
        );
        hazard(&mut img, 100, 12, 6, hex(0xf2b21a), hex(0x14171b));
        img
    });
    r.add("prefect_voss", 128, 128, |c| {
        let mut img = skin(
            c,
            &[0x0e0e12, 0x1a1a22, 0x2a2a34, 0x3a3a46],
            0xe8c040,
            0x050507,
        );
        img.rect(0, 96, 128, 16, hex(0x8a1a1a));
        img.rect(0, 100, 128, 3, hex(0xe8c040));
        img
    });
    r.add("drone_body", 64, 64, |c| {
        let mut img = skin(c, &[0xaab6c0, 0xd0dae2, 0xf0f4f8], 0x35c8e0, 0x586470);
        img.disc(32.0, 40.0, 8.0, hex(0x0a1418));
        img.disc(32.0, 40.0, 5.0, hex(0xff4a3a));
        img
    });
    // Allied marines (Vikings of Marsgard)
    r.add("marine_armor", 64, 64, |c| {
        skin(
            c,
            &[0x3a1a12, 0x6a2a1a, 0x8c3a22, 0xb5482a],
            0xd8cdb4,
            0x1a0e0a,
        )
    });
    r.add("marine_helmet", 64, 64, |c| {
        skin(
            c,
            &[0x3a3e44, 0x5a5e66, 0x7a808a, 0x9aa0aa],
            0xd8cdb4,
            0x1a1c20,
        )
    });
    r.add("marine_cloth", 64, 64, |c| {
        let p = pal(&[0x3a2a1a, 0x5a4228, 0x7a5a38]);
        Image::from_fn(c.w, c.h, 1, |x, y| {
            ramp(
                &p,
                noise2(x as f32 / 2.0, y as f32 / 2.0, 0, 0, c.seed)
                    + if (x + y) % 4 == 0 { 0.2 } else { 0.0 },
                x,
                y,
            )
        })
    });
    // Pickups
    r.add("pickup_health_s", 64, 64, |c| {
        sprite(c, |i| {
            pickup_box(i, 0xe8eef2, 0xc4ced6);
            cross(i, 32, 36, 9, hex(0xe0302a));
        })
    });
    r.add("pickup_health_l", 64, 64, |c| {
        sprite(c, |i| {
            i.rect(8, 14, 48, 40, hex(0xe8eef2));
            i.rect(8, 44, 48, 10, hex(0xb8c2ca));
            cross(i, 32, 32, 16, hex(0xe0302a));
        })
    });
    r.add("pickup_armor_shard", 64, 64, |c| {
        sprite(c, |i| {
            for k in 0..16 {
                let w = 16 - (k - 8i32).abs() * 2;
                i.rect(
                    32 - w / 2,
                    12 + k * 2,
                    w.max(2),
                    2,
                    if k < 8 { hex(0x5aff8a) } else { hex(0x1a9a4a) },
                );
            }
        })
    });
    r.add("pickup_armor_jacket", 64, 64, |c| {
        sprite(c, |i| {
            bar(i, 14, 16, 36, 34, [0xf2d04a, 0xc8a030, 0x8a6a1a]);
            i.rect(8, 16, 8, 14, hex(0xc8a030));
            i.rect(48, 16, 8, 14, hex(0xc8a030));
            i.rect(30, 16, 4, 34, hex(0x6a4a10));
        })
    });
    r.add("pickup_armor_plate", 64, 64, |c| {
        sprite(c, |i| {
            for k in 0..36 {
                let w = if k < 24 { 40 } else { 40 - (k - 24) * 3 };
                i.rect(
                    32 - w / 2,
                    10 + k,
                    w.max(2),
                    1,
                    if k < 14 { hex(0xff5a4a) } else { hex(0xb02a22) },
                );
            }
            i.rect(30, 14, 4, 28, hex(0xffd0c8));
        })
    });
    r.add("pickup_ammo_shells", 64, 64, |c| {
        sprite(c, |i| {
            for k in 0..3 {
                bar(i, 12 + k * 14, 20, 11, 28, [0xe0443a, 0xb02a22, 0x7a1a14]);
                i.rect(12 + k * 14, 40, 11, 8, hex(0xd8b048));
            }
        })
    });
    r.add("pickup_ammo_bullets", 64, 64, |c| {
        sprite(c, |i| {
            for k in 0..5 {
                bar(i, 8 + k * 10, 24, 7, 24, GOLD);
                i.rect(9 + k * 10, 18, 5, 6, hex(0xc86a3a));
            }
        })
    });
    r.add("pickup_ammo_rockets", 64, 64, |c| {
        sprite(c, |i| {
            bar(i, 24, 8, 16, 40, [0x8a9a5a, 0x5a6a3a, 0x3a4a22]);
            i.rect(24, 8, 16, 8, hex(0xe0443a));
            i.rect(16, 44, 8, 10, hex(0x3a4a22));
            i.rect(40, 44, 8, 10, hex(0x3a4a22));
        })
    });
    r.add("pickup_ammo_cells", 64, 64, |c| {
        sprite(c, |i| {
            bar(i, 18, 14, 28, 36, DARK);
            i.rect(22, 20, 20, 24, hex(0x35c8e0));
            i.rect(24, 22, 6, 20, hex(0xb8f4ff));
            i.rect(26, 10, 12, 4, hex(0xaab0ba));
        })
    });
    r.add("pickup_ammo_slugs", 64, 64, |c| {
        sprite(c, |i| {
            for k in 0..3 {
                bar(i, 12 + k * 16, 12, 10, 40, STEEL);
                i.rect(14 + k * 16, 12, 6, 8, hex(0x9ef4ff));
            }
        })
    });
    r.add("pickup_key_bronze", 64, 64, |c| {
        sprite(c, |i| key_card(i, [0xe0a060, 0xcd7f32, 0x8a4a1a]))
    });
    r.add("pickup_key_silver", 64, 64, |c| {
        sprite(c, |i| key_card(i, [0xf4f8fa, 0xc4ced6, 0x7a8794]))
    });
    r.add("pickup_key_gold", 64, 64, |c| {
        sprite(c, |i| key_card(i, [0xffe070, 0xffc830, 0xb08a10]))
    });
    r.add("pickup_chip", 64, 64, |c| {
        sprite(c, |i| {
            bar(i, 16, 16, 32, 32, [0x1a3a28, 0x0e2a1a, 0x08180e]);
            for k in 0..4 {
                i.rect(10, 20 + k * 7, 6, 3, hex(0xd8cdb4));
                i.rect(48, 20 + k * 7, 6, 3, hex(0xd8cdb4));
            }
            i.rect(24, 24, 16, 16, hex(0x4cf0a0));
            i.rect(28, 28, 8, 8, hex(0x0e2a1a));
        })
    });
    // First-person weapons
    r.add("weapon_axe", 256, 256, |c| weapon(c, draw_axe));
    r.add("weapon_blaster", 256, 256, |c| weapon(c, draw_blaster));
    r.add("weapon_shotgun", 256, 256, |c| weapon(c, draw_shotgun));
    r.add("weapon_rattler", 256, 256, |c| weapon(c, draw_rattler));
    r.add("weapon_mjolnir", 256, 256, |c| weapon(c, draw_mjolnir));
    r.add("weapon_valkyrie", 256, 256, |c| weapon(c, draw_valkyrie));
    // Effects
    r.add("muzzle_flash", 64, 64, flash);
    r.add("bolt_blue", 64, 64, |c| {
        Image::from_fn(c.w, c.h, 1, |x, y| {
            let (dx, dy) = ((x as f32 / 32.0 - 1.0) * 0.6, y as f32 / 32.0 - 1.0);
            let t = (1.0 - (dx * dx * 4.0 + dy * dy * 4.0).sqrt()).max(0.0);
            [0.3 + 0.7 * t, 0.7 + 0.3 * t, 1.0, t.sqrt()]
        })
    });
    r.add("rocket", 64, 64, |c| {
        sprite(c, |i| {
            let _ = c;
            bar(i, 12, 26, 36, 10, [0xc8d0d8, 0x8a929c, 0x4a525c]);
            i.rect(44, 26, 8, 10, hex(0xe0443a));
            i.rect(6, 22, 8, 4, hex(0x4a525c));
            i.rect(6, 36, 8, 4, hex(0x4a525c));
            i.rect(0, 28, 8, 6, hex(0xff9a2a));
        })
    });
    r.add("rail_trail", 64, 64, |c| {
        Image::from_fn(c.w, c.h, 1, |x, y| {
            let v = ((y as f32 + 0.5) / 32.0 - 1.0).abs();
            let spiral = 0.5 + 0.5 * (x as f32 * 0.5 + (y as f32 * 0.4)).sin();
            let t = (1.0 - v).max(0.0).powf(2.0);
            let _ = hash(0, 0, 0, c.seed);
            [0.5 + 0.5 * spiral, 0.9, 1.0, t * (0.6 + 0.4 * spiral)]
        })
    });
}
