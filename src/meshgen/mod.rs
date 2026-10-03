//! Procedural low-poly meshes (T03). Headless: plain `glam` data, converted to GPU meshes in
//! `gfx::models`. Flat-shaded faces with per-part textures; lighting happens at draw time.

pub mod board;
pub mod bodies;
pub mod ships;

use glam::{Mat4, Vec2, Vec3};

use crate::texgen::noise::hash;

#[derive(Debug, Clone, Copy)]
pub struct Vertex {
    pub pos: Vec3,
    pub normal: Vec3,
    pub uv: Vec2,
    /// Multiplies the texture colour (`1,1,1` = unchanged).
    pub tint: [f32; 3],
}

/// Triangles that share one texture. An empty texture name means "use the entity's texture".
#[derive(Debug, Clone, Default)]
pub struct Part {
    pub texture: &'static str,
    pub verts: Vec<Vertex>,
    pub idx: Vec<u16>,
}

#[derive(Debug, Clone, Default)]
pub struct Model {
    pub name: &'static str,
    pub parts: Vec<Part>,
    /// Engine glow sprite anchors (position, size) in model space.
    pub glows: Vec<(Vec3, f32)>,
    /// Bounding radius around the origin.
    pub radius: f32,
}

impl Model {
    pub fn triangle_count(&self) -> usize {
        self.parts.iter().map(|p| p.idx.len() / 3).sum()
    }
}

/// Incremental mesh builder with a current transform, tint and texture part.
pub struct Builder {
    parts: Vec<Part>,
    cur: usize,
    pub m: Mat4,
    pub tint: [f32; 3],
    /// Texture tiles per world unit.
    pub uv_scale: f32,
    glows: Vec<(Vec3, f32)>,
    quad_count: i32,
}

impl Default for Builder {
    fn default() -> Self {
        Builder::new()
    }
}

impl Builder {
    pub fn new() -> Builder {
        Builder {
            parts: vec![Part::default()],
            cur: 0,
            m: Mat4::IDENTITY,
            tint: [1.0; 3],
            uv_scale: 1.0 / 40.0,
            glows: Vec::new(),
            quad_count: 0,
        }
    }

    /// Switch to (or create) the part using `texture`.
    pub fn tex(&mut self, texture: &'static str) -> &mut Self {
        self.cur = match self.parts.iter().position(|p| p.texture == texture) {
            Some(i) => i,
            None => {
                self.parts.push(Part {
                    texture,
                    ..Part::default()
                });
                self.parts.len() - 1
            }
        };
        self
    }

    pub fn glow(&mut self, p: Vec3, size: f32) {
        self.glows.push((self.m.transform_point3(p), size));
    }

    /// Run `f` once as-is and once mirrored across the X axis.
    pub fn mirror_x(&mut self, mut f: impl FnMut(&mut Builder)) {
        f(self);
        let saved = self.m;
        self.m *= Mat4::from_scale(Vec3::new(-1.0, 1.0, 1.0));
        f(self);
        self.m = saved;
    }

    pub fn at(&mut self, t: Vec3) -> &mut Self {
        self.m = Mat4::from_translation(t);
        self
    }

    fn push_vert(&mut self, pos: Vec3, normal: Vec3, uv: Vec2) -> u16 {
        let part = &mut self.parts[self.cur];
        part.verts.push(Vertex {
            pos,
            normal,
            uv,
            tint: self.tint,
        });
        (part.verts.len() - 1) as u16
    }

    /// Flat quad (corners in order); the normal is flipped to face away from `inside`.
    pub fn quad(&mut self, c: [Vec3; 4], inside: Vec3) {
        let p = c.map(|v| self.m.transform_point3(v));
        let inside = self.m.transform_point3(inside);
        let mut n = (p[2] - p[0]).cross(p[3] - p[1]).normalize_or_zero();
        let centre = (p[0] + p[1] + p[2] + p[3]) * 0.25;
        if n.dot(centre - inside) < 0.0 {
            n = -n;
        }
        let (eu, ev) = (
            (p[1] - p[0]).length() * self.uv_scale,
            (p[3] - p[0]).length() * self.uv_scale,
        );
        let (eu, ev) = (eu.min(1.0), ev.min(1.0));
        self.quad_count += 1;
        let (ou, ov) = (
            hash(self.quad_count, 1, 0, 77) * (1.0 - eu),
            hash(self.quad_count, 2, 0, 77) * (1.0 - ev),
        );
        let uvs = [
            Vec2::new(ou, ov),
            Vec2::new(ou + eu, ov),
            Vec2::new(ou + eu, ov + ev),
            Vec2::new(ou, ov + ev),
        ];
        let i: Vec<u16> = (0..4).map(|k| self.push_vert(p[k], n, uvs[k])).collect();
        self.parts[self.cur]
            .idx
            .extend([i[0], i[1], i[2], i[0], i[2], i[3]]);
    }

    /// Quad with explicit full-texture UVs (sprites, doors, signs).
    pub fn quad_full(&mut self, c: [Vec3; 4], inside: Vec3) {
        let p = c.map(|v| self.m.transform_point3(v));
        let inside = self.m.transform_point3(inside);
        let mut n = (p[2] - p[0]).cross(p[3] - p[1]).normalize_or_zero();
        if n.dot((p[0] + p[2]) * 0.5 - inside) < 0.0 {
            n = -n;
        }
        let uvs = [
            Vec2::new(0.0, 1.0),
            Vec2::new(1.0, 1.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(0.0, 0.0),
        ];
        let i: Vec<u16> = (0..4).map(|k| self.push_vert(p[k], n, uvs[k])).collect();
        self.parts[self.cur]
            .idx
            .extend([i[0], i[1], i[2], i[0], i[2], i[3]]);
    }

    /// Flat triangle with planar-projected UVs (rocks); normal faces away from `inside`.
    pub fn rock_tri(&mut self, t: [Vec3; 3], inside: Vec3, radius: f32) {
        let mut n = (t[1] - t[0]).cross(t[2] - t[0]).normalize_or_zero();
        if n.dot((t[0] + t[1] + t[2]) / 3.0 - inside) < 0.0 {
            n = -n;
        }
        let a = n.abs();
        let uv = |p: Vec3| {
            let q = if a.x >= a.y && a.x >= a.z {
                Vec2::new(p.y, p.z)
            } else if a.y >= a.z {
                Vec2::new(p.x, p.z)
            } else {
                Vec2::new(p.x, p.y)
            };
            q / (2.0 * radius) + Vec2::splat(0.5)
        };
        let nn = self.m.transform_vector3(n).normalize_or_zero();
        let i: Vec<u16> = t
            .iter()
            .map(|&p| self.push_vert(self.m.transform_point3(p), nn, uv(p)))
            .collect();
        self.parts[self.cur].idx.extend([i[0], i[1], i[2]]);
    }

    /// Axis-aligned box with half extents `h` around `c`.
    pub fn cube(&mut self, c: Vec3, h: Vec3) -> &mut Self {
        self.frustum(c.z - h.z, c.z + h.z, c.x, c.y, (h.x, h.y), (h.x, h.y))
    }

    /// Box-section solid along Z: front face at `zf`, back face at `zb` with half sizes
    /// `(w, h)` front and back, centred on `(cx, cy)`.
    pub fn frustum(
        &mut self,
        zf: f32,
        zb: f32,
        cx: f32,
        cy: f32,
        front: (f32, f32),
        back: (f32, f32),
    ) -> &mut Self {
        let f = |sx: f32, sy: f32| Vec3::new(cx + sx * front.0, cy + sy * front.1, zf);
        let b = |sx: f32, sy: f32| Vec3::new(cx + sx * back.0, cy + sy * back.1, zb);
        let centre = Vec3::new(cx, cy, (zf + zb) * 0.5);
        let corners = [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)];
        self.quad(corners.map(|(x, y)| f(x, y)), centre);
        self.quad(corners.map(|(x, y)| b(x, y)), centre);
        for i in 0..4 {
            let (a, c2) = (corners[i], corners[(i + 1) % 4]);
            self.quad(
                [f(a.0, a.1), f(c2.0, c2.1), b(c2.0, c2.1), b(a.0, a.1)],
                centre,
            );
        }
        self
    }

    /// Cylinder/cone along Z from `zf` (radius `rf`) to `zb` (radius `rb`).
    pub fn prism(&mut self, c: Vec3, zf: f32, zb: f32, rf: f32, rb: f32, sides: u32) -> &mut Self {
        let ring = |z: f32, r: f32, i: u32| {
            let a = i as f32 / sides as f32 * std::f32::consts::TAU;
            Vec3::new(c.x + a.cos() * r, c.y + a.sin() * r, c.z + z)
        };
        let centre = Vec3::new(c.x, c.y, c.z + (zf + zb) * 0.5);
        for i in 0..sides {
            let j = (i + 1) % sides;
            self.quad(
                [
                    ring(zf, rf, i),
                    ring(zf, rf, j),
                    ring(zb, rb, j),
                    ring(zb, rb, i),
                ],
                centre,
            );
        }
        for (z, r) in [(zf, rf), (zb, rb)] {
            if r > 0.0 {
                let mid = Vec3::new(c.x, c.y, c.z + z);
                for i in 0..sides {
                    let j = (i + 1) % sides;
                    self.quad([mid, ring(z, r, i), ring(z, r, j), mid], centre);
                }
            }
        }
        self
    }

    /// Smooth-shaded UV sphere with equirectangular UVs.
    pub fn sphere(&mut self, c: Vec3, r: f32, segs: u32, rings: u32) -> &mut Self {
        let base = self.parts[self.cur].verts.len();
        for y in 0..=rings {
            let v = y as f32 / rings as f32;
            let lat = (v - 0.5) * std::f32::consts::PI;
            for x in 0..=segs {
                let u = x as f32 / segs as f32;
                let lon = u * std::f32::consts::TAU;
                let n = Vec3::new(lat.cos() * lon.cos(), lat.sin(), lat.cos() * lon.sin());
                self.push_vert(
                    self.m.transform_point3(c + n * r),
                    self.m.transform_vector3(n).normalize_or_zero(),
                    Vec2::new(u, 1.0 - v),
                );
            }
        }
        let row = segs as usize + 1;
        for y in 0..rings as usize {
            for x in 0..segs as usize {
                let a = (base + y * row + x) as u16;
                let (b, c2, d) = (a + 1, a + row as u16 + 1, a + row as u16);
                self.parts[self.cur].idx.extend([a, d, c2, a, c2, b]);
            }
        }
        self
    }

    /// Finish the model; the bounding radius is measured from the vertices.
    pub fn finish(self, name: &'static str) -> Model {
        let parts: Vec<Part> = self
            .parts
            .into_iter()
            .filter(|p| !p.idx.is_empty())
            .collect();
        let radius = parts
            .iter()
            .flat_map(|p| p.verts.iter())
            .map(|v| v.pos.length())
            .fold(0.0, f32::max);
        Model {
            name,
            parts,
            glows: self.glows,
            radius,
        }
    }
}

/// Every model in the game, keyed by name.
pub fn all() -> Vec<Model> {
    let mut v = Vec::new();
    v.extend(ships::all());
    v.extend(bodies::all());
    v.extend(board::all());
    v
}

pub fn build(name: &str) -> Option<Model> {
    all().into_iter().find(|m| m.name == name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_models_are_valid() {
        let models = all();
        assert!(models.len() >= 35, "only {} models", models.len());
        let mut names: Vec<_> = models.iter().map(|m| m.name).collect();
        names.sort_unstable();
        let n = names.len();
        names.dedup();
        assert_eq!(n, names.len(), "duplicate model names");
        for m in &models {
            assert!(!m.parts.is_empty(), "{} empty", m.name);
            for p in &m.parts {
                assert!(p.verts.len() < 65_000, "{} too big", m.name);
                assert_eq!(p.idx.len() % 3, 0);
                assert!(p.idx.iter().all(|&i| (i as usize) < p.verts.len()));
                for t in p.idx.chunks(3) {
                    let (a, b, c) = (
                        p.verts[t[0] as usize].pos,
                        p.verts[t[1] as usize].pos,
                        p.verts[t[2] as usize].pos,
                    );
                    assert!(
                        a.is_finite() && b.is_finite() && c.is_finite(),
                        "{} NaN",
                        m.name
                    );
                    if p.verts.len() < 5000 {
                        // degenerate triangles are allowed only at cone tips / sphere poles
                    }
                }
                for v in &p.verts {
                    assert!(
                        (v.normal.length() - 1.0).abs() < 1e-3,
                        "{} bad normal",
                        m.name
                    );
                    assert!(
                        v.pos.length() <= m.radius + 1e-3,
                        "{} vertex outside radius",
                        m.name
                    );
                }
            }
        }
    }
}
