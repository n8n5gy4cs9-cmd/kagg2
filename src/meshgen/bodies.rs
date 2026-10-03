//! Planets, rocks, rings and billboards.

use std::f32::consts::{PI, TAU};

use glam::Vec3;

use super::{Builder, Model};
use crate::texgen::noise::noise3;

fn asteroid(name: &'static str, seed: u32, rough: f32) -> Model {
    let mut b = Builder::new();
    b.tex("");
    let (segs, rings) = (12u32, 8u32);
    let point = |x: u32, y: u32| -> Vec3 {
        let lon = x as f32 / segs as f32 * TAU;
        let lat = (y as f32 / rings as f32 - 0.5) * PI;
        let d = Vec3::new(lat.cos() * lon.cos(), lat.sin(), lat.cos() * lon.sin());
        let bump = noise3(d * 1.7 + Vec3::splat(seed as f32), seed) - 0.5;
        let squash = Vec3::new(1.0 + 0.25 * ((seed % 3) as f32 - 1.0), 0.85, 1.0);
        d * squash * (1.0 + bump * rough)
    };
    for y in 0..rings {
        for x in 0..segs {
            let (a, bb, c, d) = (
                point(x, y),
                point(x + 1, y),
                point(x + 1, y + 1),
                point(x, y + 1),
            );
            b.rock_tri([a, bb, c], Vec3::ZERO, 1.0);
            b.rock_tri([a, c, d], Vec3::ZERO, 1.0);
        }
    }
    b.finish(name)
}

fn ring() -> Model {
    let mut b = Builder::new();
    b.tex("ring_saturn");
    let n = 48;
    let (r0, r1) = (1.25, 2.3);
    for i in 0..n {
        let (a0, a1) = (i as f32 / n as f32 * TAU, (i + 1) as f32 / n as f32 * TAU);
        let p = |a: f32, r: f32| Vec3::new(a.cos() * r, 0.0, a.sin() * r);
        // thin double-sided strip
        for (h, up) in [(0.002, Vec3::Y), (-0.002, Vec3::NEG_Y)] {
            let off = Vec3::new(0.0, h, 0.0);
            b.quad_full(
                [
                    p(a0, r0) + off,
                    p(a0, r1) + off,
                    p(a1, r1) + off,
                    p(a1, r0) + off,
                ],
                off - up,
            );
        }
    }
    b.finish("ring")
}

fn billboard() -> Model {
    let mut b = Builder::new();
    b.tex("");
    b.quad_full(
        [
            Vec3::new(-0.5, 0.5, 0.0),
            Vec3::new(0.5, 0.5, 0.0),
            Vec3::new(0.5, -0.5, 0.0),
            Vec3::new(-0.5, -0.5, 0.0),
        ],
        Vec3::new(0.0, 0.0, -1.0),
    );
    b.finish("quad")
}

pub fn all() -> Vec<Model> {
    let mut b = Builder::new();
    b.tex("").sphere(Vec3::ZERO, 1.0, 40, 20);
    let mut lo = Builder::new();
    lo.tex("").sphere(Vec3::ZERO, 1.0, 14, 8);
    vec![
        b.finish("sphere"),
        lo.finish("sphere_lo"),
        asteroid("asteroid_a", 3, 0.55),
        asteroid("asteroid_b", 11, 0.7),
        asteroid("asteroid_c", 19, 0.45),
        ring(),
        billboard(),
    ]
}
