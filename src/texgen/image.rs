//! RGBA8 image buffer plus the tiny drawing and palette helpers the recipes share.

use super::noise::bayer4;

/// Colour as `[r, g, b, a]` floats in `0..=1`.
pub type Col = [f32; 4];

/// Opaque colour from `0xRRGGBB`.
pub fn hex(rgb: u32) -> Col {
    [
        ((rgb >> 16) & 255) as f32 / 255.0,
        ((rgb >> 8) & 255) as f32 / 255.0,
        (rgb & 255) as f32 / 255.0,
        1.0,
    ]
}

pub fn with_alpha(c: Col, a: f32) -> Col {
    [c[0], c[1], c[2], a]
}

pub fn mix(a: Col, b: Col, t: f32) -> Col {
    let t = t.clamp(0.0, 1.0);
    std::array::from_fn(|i| a[i] + (b[i] - a[i]) * t)
}

pub fn scale(c: Col, k: f32) -> Col {
    [c[0] * k, c[1] * k, c[2] * k, c[3]]
}

/// Palette lookup with ordered dithering: `t` in `0..=1` picks one of the palette steps, the
/// Bayer offset at `(x, y)` makes neighbouring steps interleave — the classic pixel-art ramp.
pub fn ramp(pal: &[Col], t: f32, x: u32, y: u32) -> Col {
    let n = pal.len();
    let f = t.clamp(0.0, 1.0) * (n - 1) as f32 + bayer4(x, y) * 0.8;
    pal[(f + 0.5).floor().clamp(0.0, (n - 1) as f32) as usize]
}

/// Palette lookup without dithering (hard bands).
pub fn band(pal: &[Col], t: f32) -> Col {
    let n = pal.len();
    pal[((t.clamp(0.0, 0.9999)) * n as f32) as usize]
}

#[derive(Clone)]
pub struct Image {
    pub w: u32,
    pub h: u32,
    pub px: Vec<[u8; 4]>,
}

fn to_u8(c: Col) -> [u8; 4] {
    std::array::from_fn(|i| (c[i].clamp(0.0, 1.0) * 255.0 + 0.5) as u8)
}

fn from_u8(p: [u8; 4]) -> Col {
    std::array::from_fn(|i| p[i] as f32 / 255.0)
}

impl Image {
    pub fn new(w: u32, h: u32) -> Image {
        Image {
            w,
            h,
            px: vec![[0; 4]; (w * h) as usize],
        }
    }

    pub fn filled(w: u32, h: u32, c: Col) -> Image {
        Image {
            w,
            h,
            px: vec![to_u8(c); (w * h) as usize],
        }
    }

    /// Build from a per-pixel function. `block` > 1 renders at `1/block` resolution and
    /// upscales with nearest-neighbour, giving chunky "big pixel" art.
    pub fn from_fn(w: u32, h: u32, block: u32, mut f: impl FnMut(u32, u32) -> Col) -> Image {
        let mut img = Image::new(w, h);
        let b = block.max(1);
        for y in 0..h {
            for x in 0..w {
                let (sx, sy) = (x / b, y / b);
                let c = if x % b == 0 && y % b == 0 {
                    f(sx, sy)
                } else {
                    from_u8(img.px[(y - y % b) as usize * w as usize + (x - x % b) as usize])
                };
                img.px[(y * w + x) as usize] = to_u8(c);
            }
        }
        img
    }

    pub fn get(&self, x: i32, y: i32) -> Col {
        if x < 0 || y < 0 || x >= self.w as i32 || y >= self.h as i32 {
            return [0.0; 4];
        }
        from_u8(self.px[(y as u32 * self.w + x as u32) as usize])
    }

    /// Alpha-blend `c` over the pixel (clipped).
    pub fn put(&mut self, x: i32, y: i32, c: Col) {
        if x < 0 || y < 0 || x >= self.w as i32 || y >= self.h as i32 {
            return;
        }
        let i = (y as u32 * self.w + x as u32) as usize;
        let d = from_u8(self.px[i]);
        let a = c[3];
        let out_a = a + d[3] * (1.0 - a);
        let k = |i: usize| {
            if out_a > 0.0 {
                (c[i] * a + d[i] * d[3] * (1.0 - a)) / out_a
            } else {
                0.0
            }
        };
        self.px[i] = to_u8([k(0), k(1), k(2), out_a]);
    }

    /// Additive blend (glows).
    pub fn add(&mut self, x: i32, y: i32, c: Col) {
        let d = self.get(x, y);
        let a = c[3];
        let out = [
            d[0] + c[0] * a,
            d[1] + c[1] * a,
            d[2] + c[2] * a,
            d[3].max(a),
        ];
        if x >= 0 && y >= 0 && x < self.w as i32 && y < self.h as i32 {
            self.px[(y as u32 * self.w + x as u32) as usize] = to_u8(out);
        }
    }

    pub fn rect(&mut self, x: i32, y: i32, w: i32, h: i32, c: Col) {
        for j in y..y + h {
            for i in x..x + w {
                self.put(i, j, c);
            }
        }
    }

    pub fn frame(&mut self, x: i32, y: i32, w: i32, h: i32, c: Col) {
        self.rect(x, y, w, 1, c);
        self.rect(x, y + h - 1, w, 1, c);
        self.rect(x, y, 1, h, c);
        self.rect(x + w - 1, y, 1, h, c);
    }

    pub fn disc(&mut self, cx: f32, cy: f32, r: f32, c: Col) {
        let (x0, x1) = ((cx - r).floor() as i32, (cx + r).ceil() as i32);
        let (y0, y1) = ((cy - r).floor() as i32, (cy + r).ceil() as i32);
        for y in y0..=y1 {
            for x in x0..=x1 {
                let (dx, dy) = (x as f32 + 0.5 - cx, y as f32 + 0.5 - cy);
                if dx * dx + dy * dy <= r * r {
                    self.put(x, y, c);
                }
            }
        }
    }

    pub fn line(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, c: Col) {
        let (dx, dy) = ((x1 - x0).abs(), -(y1 - y0).abs());
        let (sx, sy) = (if x0 < x1 { 1 } else { -1 }, if y0 < y1 { 1 } else { -1 });
        let (mut x, mut y, mut err) = (x0, y0, dx + dy);
        loop {
            self.put(x, y, c);
            if x == x1 && y == y1 {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x += sx;
            }
            if e2 <= dx {
                err += dx;
                y += sy;
            }
        }
    }

    /// Draw `src` at `(x, y)` with alpha blending.
    pub fn blit(&mut self, src: &Image, x: i32, y: i32) {
        for j in 0..src.h as i32 {
            for i in 0..src.w as i32 {
                self.put(x + i, y + j, src.get(i, j));
            }
        }
    }

    /// Nearest-neighbour upscale by an integer factor.
    pub fn upscale(&self, k: u32) -> Image {
        Image::from_fn(self.w * k, self.h * k, 1, |x, y| {
            self.get((x / k) as i32, (y / k) as i32)
        })
    }

    /// Dark 1px outline around opaque pixels (sprite readability).
    pub fn outline(&mut self, col: Col) {
        let src = self.clone();
        for y in 0..self.h as i32 {
            for x in 0..self.w as i32 {
                if src.get(x, y)[3] < 0.1
                    && [(1, 0), (-1, 0), (0, 1), (0, -1)]
                        .iter()
                        .any(|&(dx, dy)| src.get(x + dx, y + dy)[3] > 0.5)
                {
                    self.px[(y as u32 * self.w + x as u32) as usize] = to_u8(col);
                }
            }
        }
    }

    /// Nearest-neighbour downscale by an integer factor (used for the contact sheet).
    pub fn shrink(&self, k: u32) -> Image {
        Image::from_fn(self.w / k, self.h / k, 1, |x, y| {
            self.get((x * k) as i32, (y * k) as i32)
        })
    }

    pub fn mean_rgb(&self) -> [u8; 3] {
        let n = self.px.len().max(1) as u64;
        let sum = |c: usize| (self.px.iter().map(|p| p[c] as u64).sum::<u64>() / n) as u8;
        [sum(0), sum(1), sum(2)]
    }

    /// FNV-1a hash of the pixel data (determinism tests).
    pub fn hash(&self) -> u64 {
        self.px
            .iter()
            .flatten()
            .fold(0xcbf2_9ce4_8422_2325u64, |h, &b| {
                (h ^ b as u64).wrapping_mul(0x0000_0100_0000_01b3)
            })
    }

    /// Tileability metric: mean colour step across the wrap seam divided by the mean step
    /// between ordinary neighbours (≈1 for a seamless tile, ≫1 for a visible seam).
    pub fn seam_ratio(&self) -> f32 {
        let d = |a: [u8; 4], b: [u8; 4]| -> f32 {
            (0..3)
                .map(|i| (a[i] as f32 - b[i] as f32).abs())
                .sum::<f32>()
        };
        let (w, h) = (self.w as usize, self.h as usize);
        let (mut seam, mut inner) = (0.0, 0.0);
        for y in 0..h {
            seam += d(self.px[y * w], self.px[y * w + w - 1]);
            inner += d(self.px[y * w + w / 2], self.px[y * w + w / 2 - 1]);
        }
        for x in 0..w {
            seam += d(self.px[x], self.px[(h - 1) * w + x]);
            inner += d(self.px[(h / 2) * w + x], self.px[(h / 2 - 1) * w + x]);
        }
        (seam + 1.0) / (inner + 1.0)
    }

    pub fn write_png(&self, path: &std::path::Path) -> Result<(), super::TexError> {
        let file = std::fs::File::create(path)?;
        let mut enc = png::Encoder::new(std::io::BufWriter::new(file), self.w, self.h);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        let mut writer = enc.write_header()?;
        let data: Vec<u8> = self.px.iter().flatten().copied().collect();
        writer.write_image_data(&data)?;
        Ok(())
    }
}
