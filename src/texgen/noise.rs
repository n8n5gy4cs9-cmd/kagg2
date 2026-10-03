//! Deterministic noise primitives: hashing, periodic (tileable) value noise, fbm, voronoi.

use glam::Vec3;

/// Integer hash → `[0, 1)`.
pub fn hash(x: i32, y: i32, z: i32, seed: u32) -> f32 {
    let mut h = (x as u32).wrapping_mul(0x8da6_b343)
        ^ (y as u32).wrapping_mul(0xd816_3841)
        ^ (z as u32).wrapping_mul(0xcb1a_b31f)
        ^ seed.wrapping_mul(0x9e37_79b9);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2c1b_3c6d);
    h ^= h >> 12;
    h = h.wrapping_mul(0x297a_2d39);
    h ^= h >> 15;
    (h >> 8) as f32 / (1u32 << 24) as f32
}

fn smooth(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn wrap(i: i32, period: i32) -> i32 {
    if period > 0 { i.rem_euclid(period) } else { i }
}

/// 2D value noise that tiles with the given integer lattice periods (0 = no wrap).
pub fn noise2(x: f32, y: f32, px: i32, py: i32, seed: u32) -> f32 {
    let (xi, yi) = (x.floor(), y.floor());
    let (fx, fy) = (smooth(x - xi), smooth(y - yi));
    let (xi, yi) = (xi as i32, yi as i32);
    let v = |dx: i32, dy: i32| hash(wrap(xi + dx, px), wrap(yi + dy, py), 0, seed);
    lerp(lerp(v(0, 0), v(1, 0), fx), lerp(v(0, 1), v(1, 1), fx), fy)
}

/// Tileable fbm over the unit square: `base` lattice cells across, doubling per octave.
pub fn fbm2(u: f32, v: f32, base: i32, octaves: u32, seed: u32) -> f32 {
    let (mut sum, mut amp, mut norm, mut cells) = (0.0, 0.5, 0.0, base);
    for o in 0..octaves {
        sum += amp
            * noise2(
                u * cells as f32,
                v * cells as f32,
                cells,
                cells,
                seed + o * 31,
            );
        norm += amp;
        amp *= 0.5;
        cells *= 2;
    }
    sum / norm
}

/// 3D value noise (non-periodic) — used on sphere directions so planets have no seams.
pub fn noise3(p: Vec3, seed: u32) -> f32 {
    let f = p.floor();
    let t = p - f;
    let (sx, sy, sz) = (smooth(t.x), smooth(t.y), smooth(t.z));
    let (xi, yi, zi) = (f.x as i32, f.y as i32, f.z as i32);
    let v = |dx: i32, dy: i32, dz: i32| hash(xi + dx, yi + dy, zi + dz, seed);
    let x00 = lerp(v(0, 0, 0), v(1, 0, 0), sx);
    let x10 = lerp(v(0, 1, 0), v(1, 1, 0), sx);
    let x01 = lerp(v(0, 0, 1), v(1, 0, 1), sx);
    let x11 = lerp(v(0, 1, 1), v(1, 1, 1), sx);
    lerp(lerp(x00, x10, sy), lerp(x01, x11, sy), sz)
}

pub fn fbm3(p: Vec3, octaves: u32, seed: u32) -> f32 {
    let (mut sum, mut amp, mut norm, mut q) = (0.0, 0.5, 0.0, p);
    for o in 0..octaves {
        sum += amp * noise3(q, seed + o * 17);
        norm += amp;
        amp *= 0.5;
        q *= 2.03;
    }
    sum / norm
}

/// Tileable voronoi over the unit square with `cells` × `cells` cells.
/// Returns (distance to nearest, distance to second nearest) in cell units.
pub fn voronoi(u: f32, v: f32, cells: i32, seed: u32) -> (f32, f32) {
    let (x, y) = (u * cells as f32, v * cells as f32);
    let (cx, cy) = (x.floor() as i32, y.floor() as i32);
    let (mut d1, mut d2) = (f32::MAX, f32::MAX);
    for j in -1..=1 {
        for i in -1..=1 {
            let (gx, gy) = (cx + i, cy + j);
            let (wx, wy) = (wrap(gx, cells), wrap(gy, cells));
            let px = gx as f32 + hash(wx, wy, 1, seed);
            let py = gy as f32 + hash(wx, wy, 2, seed);
            let d = ((px - x).powi(2) + (py - y).powi(2)).sqrt();
            if d < d1 {
                d2 = d1;
                d1 = d;
            } else if d < d2 {
                d2 = d;
            }
        }
    }
    (d1, d2)
}

/// 4×4 Bayer matrix value in `[-0.5, 0.5)` for ordered dithering.
pub fn bayer4(x: u32, y: u32) -> f32 {
    const M: [[u8; 4]; 4] = [[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]];
    (M[(y & 3) as usize][(x & 3) as usize] as f32 + 0.5) / 16.0 - 0.5
}

/// Stable 32-bit hash of a string (seed per texture name).
pub fn seed_of(name: &str) -> u32 {
    name.bytes().fold(0x811c_9dc5u32, |h, b| {
        (h ^ b as u32).wrapping_mul(0x0100_0193)
    })
}
