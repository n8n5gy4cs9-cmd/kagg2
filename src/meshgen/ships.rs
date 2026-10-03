//! Ship and station models. Nose points toward −Z, up is +Y.

use std::f32::consts::{FRAC_PI_2, TAU};

use glam::{Mat4, Vec3};

use super::{Builder, Model};

const VK_A: &str = "hull_viking_a";
const VK_B: &str = "hull_viking_b";
const VK_TRIM: &str = "hull_viking_trim";
const MN_A: &str = "hull_minion_a";
const MN_B: &str = "hull_minion_b";
const MN_DARK: &str = "hull_minion_dark";

fn v3(x: f32, y: f32, z: f32) -> Vec3 {
    Vec3::new(x, y, z)
}

/// Longship hull; `k` scales everything (drakkar = bigger, two masts).
fn viking(name: &'static str, k: f32, masts: u32, hull_tex: &'static str) -> Model {
    let mut b = Builder::new();
    b.uv_scale = 1.0 / (44.0 * k);
    b.m = Mat4::from_scale(Vec3::splat(k));
    b.tex(hull_tex);
    // keel + main hull + raised prow and stern
    b.frustum(-14.0, 18.0, 0.0, 0.0, (2.6, 2.0), (6.5, 3.2));
    b.frustum(-22.0, -14.0, 0.8, 0.4, (0.5, 0.7), (2.6, 2.0));
    b.frustum(16.0, 22.0, 0.0, 1.6, (6.0, 3.0), (4.0, 3.6));
    // hull tint stripes along the gunwale
    b.tint = [1.15, 1.1, 1.0];
    b.frustum(-12.0, 17.0, 0.0, 3.0, (2.8, 0.5), (6.9, 0.5));
    b.tint = [1.0; 3];
    b.tex(VK_TRIM);
    // dragon neck + head, curving up at the prow
    b.cube(v3(0.0, 3.2, -20.5), v3(0.7, 3.0, 0.7));
    b.cube(v3(0.0, 6.6, -21.6), v3(0.8, 0.8, 1.6));
    b.cube(v3(0.0, 7.3, -23.2), v3(0.5, 0.4, 0.8));
    b.cube(v3(0.0, 8.4, -20.8), v3(0.3, 1.0, 0.3)); // horn/crest
    // tail fin at the stern
    b.cube(v3(0.0, 6.0, 21.0), v3(0.4, 3.0, 0.8));
    // gunwale shields (octagonal discs along both sides)
    b.mirror_x(|b| {
        let m = b.m;
        for i in 0..7 {
            let z = -10.0 + i as f32 * 4.2;
            let x = 2.9 + (z + 14.0) / 32.0 * 4.1 + 0.4;
            b.m = m * Mat4::from_translation(v3(x, 3.4, z)) * Mat4::from_rotation_y(FRAC_PI_2);
            b.tint = if i % 2 == 0 {
                [1.2, 0.5, 0.35]
            } else {
                [0.95, 0.95, 0.85]
            };
            b.prism(Vec3::ZERO, -0.25, 0.25, 1.5, 1.5, 8);
        }
        b.m = m;
        b.tint = [1.0; 3];
    });
    // masts + sails (solar sails, texture shows the raven)
    for i in 0..masts {
        let z = if masts == 1 {
            2.0
        } else {
            -4.0 + i as f32 * 12.0
        };
        b.tex(VK_TRIM);
        b.cube(v3(0.0, 9.0, z), v3(0.45, 9.0, 0.45));
        b.cube(v3(0.0, 15.5, z), v3(8.5, 0.35, 0.35));
        b.tex("sail_viking");
        b.tint = [1.5, 1.4, 1.3];
        b.cube(v3(0.0, 10.0, z - 0.2), v3(7.6, 5.4, 0.12));
        b.tint = [1.0; 3];
    }
    // engines at the stern
    b.tex(VK_B);
    b.mirror_x(|b| {
        b.prism(v3(3.4, 0.8, 0.0), 21.0, 25.0, 1.8, 2.2, 8);
        b.prism(v3(3.4, 0.8, 0.0), 24.9, 25.4, 2.0, 1.4, 8);
        b.glow(v3(3.4, 0.8, 26.5), 6.0);
    });
    b.finish(name)
}

fn skiff() -> Model {
    let mut b = Builder::new();
    b.uv_scale = 1.0 / 20.0;
    b.tex(VK_B);
    b.frustum(-9.0, 7.0, 0.0, 0.0, (0.3, 0.5), (2.2, 1.4));
    b.mirror_x(|b| {
        b.frustum(-2.0, 7.0, 3.6, -0.2, (2.0, 0.25), (3.2, 0.25));
        b.tex(VK_TRIM).cube(v3(5.6, 0.6, 5.5), v3(0.25, 1.4, 1.2));
        b.tex(VK_B);
    });
    b.tex(VK_TRIM).cube(v3(0.0, 1.9, -3.5), v3(0.9, 0.6, 2.0));
    b.tex(VK_B).prism(v3(0.0, 0.0, 0.0), 7.0, 9.0, 1.0, 1.3, 8);
    b.glow(v3(0.0, 0.0, 10.0), 4.0);
    b.finish("skiff")
}

fn hauler() -> Model {
    let mut b = Builder::new();
    b.uv_scale = 1.0 / 30.0;
    b.tex(MN_DARK);
    b.frustum(-16.0, 20.0, 0.0, 0.0, (1.4, 1.4), (1.8, 1.8)); // spine
    b.frustum(-26.0, -16.0, 0.0, 0.0, (2.0, 1.6), (3.6, 3.0)); // cab
    b.tex(MN_A).cube(v3(0.0, 2.6, -21.0), v3(1.2, 0.6, 2.4));
    for i in 0..4 {
        let z = -10.0 + i as f32 * 8.5;
        for (sx, sy) in [(-1.0f32, 0.0f32), (1.0, 0.0), (0.0, 1.0), (0.0, -1.0)] {
            b.tex(if (i + sx as i32 + sy as i32 * 2) % 2 == 0 {
                "cargo_container_a"
            } else {
                "cargo_container_b"
            });
            b.uv_scale = 1.0 / 8.0;
            b.cube(
                v3(sx * 4.6, sy * 4.6, z),
                v3(2.0 + sy.abs() * 0.6, 2.0 + sx.abs() * 0.6, 3.5),
            );
        }
    }
    b.tex(MN_DARK)
        .prism(v3(0.0, 0.0, 0.0), 20.0, 24.0, 2.0, 2.6, 8);
    b.glow(v3(0.0, 0.0, 25.5), 8.0);
    b.finish("hauler")
}

fn ring(b: &mut Builder, centre: Vec3, r: f32, tube: f32, n: u32, h: f32) {
    // flat ring of boxes in the XZ plane around `centre`
    let m = b.m;
    for i in 0..n {
        let a = i as f32 / n as f32 * TAU;
        b.m = m
            * Mat4::from_translation(centre + v3(a.cos() * r, 0.0, a.sin() * r))
            * Mat4::from_rotation_y(-a);
        let seg = r * (TAU / n as f32) * 0.55;
        b.cube(Vec3::ZERO, v3(tube, h, seg));
    }
    b.m = m;
}

fn forge() -> Model {
    let mut b = Builder::new();
    b.uv_scale = 1.0 / 90.0;
    b.tex("station_forge");
    // central hub, furnace stack, rotating habitat ring and docking arms
    b.prism(Vec3::ZERO, -60.0, 60.0, 16.0, 16.0, 12);
    b.prism(Vec3::ZERO, -80.0, -60.0, 6.0, 16.0, 12);
    b.prism(Vec3::ZERO, 60.0, 78.0, 16.0, 9.0, 12);
    b.prism(v3(0.0, 0.0, 0.0), 78.0, 92.0, 5.0, 5.0, 8);
    let m = b.m;
    b.m = m * Mat4::from_rotation_x(FRAC_PI_2);
    ring(&mut b, Vec3::ZERO, 70.0, 7.0, 20, 11.0);
    b.m = m;
    for i in 0..4 {
        let a = i as f32 * FRAC_PI_2 + 0.78;
        b.m = m * Mat4::from_rotation_z(a);
        b.cube(v3(0.0, 40.0, 0.0), v3(2.0, 24.0, 2.0));
    }
    b.m = m;
    b.tex(VK_TRIM);
    for i in 0..6 {
        let a = i as f32 * TAU / 6.0;
        b.cube(
            v3(a.cos() * 22.0, a.sin() * 22.0, -30.0),
            v3(2.0, 2.0, 12.0),
        );
    }
    b.tex(VK_B);
    b.mirror_x(|b| {
        b.cube(v3(28.0, 0.0, 20.0), v3(10.0, 1.5, 22.0));
    });
    b.glow(v3(0.0, 0.0, 96.0), 30.0);
    b.finish("forge")
}

fn fort() -> Model {
    let mut b = Builder::new();
    b.uv_scale = 1.0 / 90.0;
    b.tex("station_fort");
    b.prism(Vec3::ZERO, -50.0, 50.0, 22.0, 22.0, 8);
    b.cube(Vec3::ZERO, v3(40.0, 8.0, 40.0));
    b.cube(Vec3::ZERO, v3(8.0, 40.0, 40.0));
    b.tex(MN_DARK);
    for (x, y, z) in [
        (0.0, 12.0, -30.0),
        (0.0, -12.0, 30.0),
        (30.0, 0.0, 0.0),
        (-30.0, 0.0, 0.0),
    ] {
        b.prism(v3(x, y, z), -8.0, 8.0, 9.0, 9.0, 8);
        b.cube(
            v3(x * 1.0, y + y.signum() * 11.0, z - 14.0),
            v3(1.5, 1.5, 12.0),
        );
    }
    b.tex(MN_A);
    b.prism(v3(0.0, 0.0, 0.0), 50.0, 70.0, 22.0, 10.0, 8);
    b.prism(v3(0.0, 0.0, 0.0), -70.0, -50.0, 10.0, 22.0, 8);
    b.finish("fort")
}

/// Sleek Concord interceptor-style hull; shared by several Minion classes via parameters.
fn minion(
    name: &'static str,
    len: f32,
    width: f32,
    height: f32,
    fins: bool,
    pods: u32,
    tex: &'static str,
) -> Model {
    let mut b = Builder::new();
    b.uv_scale = 1.0 / len.max(30.0);
    b.tex(tex);
    let h = len * 0.5;
    b.frustum(-h, -h * 0.2, 0.0, 0.0, (0.25, 0.3), (width, height));
    b.frustum(
        -h * 0.2,
        h,
        0.0,
        0.0,
        (width, height),
        (width * 0.8, height * 0.9),
    );
    if fins {
        b.mirror_x(|b| {
            b.frustum(
                -h * 0.1,
                h * 0.6,
                width * 1.5,
                0.0,
                (width * 0.9, 0.12 * height),
                (width * 0.6, 0.12 * height),
            );
        });
        b.tex(MN_DARK).cube(
            v3(0.0, height * 1.6, h * 0.5),
            v3(0.12 * width, height * 0.7, h * 0.18),
        );
        b.tex(tex);
    }
    for i in 0..pods {
        let z = -h * 0.2 + i as f32 * (len * 0.9 / pods.max(1) as f32) * 0.8;
        b.tex(MN_DARK);
        b.mirror_x(|b| {
            b.prism(
                v3(width * 1.4, 0.0, z),
                -len * 0.12,
                len * 0.12,
                height * 0.55,
                height * 0.55,
                6,
            );
        });
        b.tex(tex);
    }
    // bridge tower
    b.tex(MN_B).cube(
        v3(0.0, height * 1.3, h * 0.15),
        v3(width * 0.3, height * 0.4, len * 0.07),
    );
    // engine block with glow
    b.tex(MN_DARK).frustum(
        h,
        h + len * 0.06,
        0.0,
        0.0,
        (width * 0.7, height * 0.8),
        (width * 0.55, height * 0.65),
    );
    b.glow(v3(0.0, 0.0, h + len * 0.1), width * 1.6);
    if width > 6.0 {
        b.mirror_x(|b| b.glow(v3(width * 0.3, 0.0, h + len * 0.1), width * 1.0));
    }
    b.finish(name)
}

fn dreadnought() -> Model {
    let mut b = Builder::new();
    b.uv_scale = 1.0 / 150.0;
    b.tex(MN_A);
    b.frustum(-110.0, -40.0, 0.0, 0.0, (4.0, 6.0), (26.0, 16.0));
    b.frustum(-40.0, 90.0, 0.0, 0.0, (26.0, 16.0), (22.0, 15.0));
    b.tex(MN_B);
    b.frustum(-20.0, 50.0, 0.0, 16.0, (14.0, 5.0), (12.0, 6.0));
    b.cube(v3(0.0, 28.0, 10.0), v3(7.0, 8.0, 14.0));
    b.tex(MN_DARK);
    b.mirror_x(|b| {
        b.frustum(-30.0, 70.0, 34.0, 0.0, (8.0, 5.0), (10.0, 6.0));
        for i in 0..4 {
            b.prism(
                v3(30.0, 12.0, -50.0 + i as f32 * 28.0),
                -8.0,
                8.0,
                5.0,
                5.0,
                8,
            );
            b.cube(v3(30.0, 12.0, -64.0 + i as f32 * 28.0), v3(1.0, 1.0, 10.0));
        }
    });
    b.tex(MN_A);
    b.frustum(90.0, 100.0, 0.0, 0.0, (16.0, 11.0), (14.0, 9.0));
    b.mirror_x(|b| b.glow(v3(10.0, 0.0, 105.0), 18.0));
    b.glow(v3(0.0, 0.0, 105.0), 22.0);
    b.finish("dreadnought")
}

fn prison_barge() -> Model {
    let mut b = Builder::new();
    b.uv_scale = 1.0 / 80.0;
    b.tex(MN_DARK);
    b.frustum(-45.0, 45.0, 0.0, 0.0, (14.0, 10.0), (18.0, 12.0));
    b.tex(MN_A);
    for i in 0..5 {
        let z = -32.0 + i as f32 * 16.0;
        b.mirror_x(|b| {
            b.cube(v3(22.0, 0.0, z), v3(5.0, 8.0, 6.0));
        });
    }
    b.cube(v3(0.0, 13.0, -30.0), v3(5.0, 3.0, 8.0));
    b.tex(MN_B)
        .prism(v3(0.0, 0.0, 0.0), 45.0, 55.0, 8.0, 10.0, 8);
    b.glow(v3(0.0, 0.0, 57.0), 14.0);
    b.finish("prison_barge")
}

fn torpedo() -> Model {
    let mut b = Builder::new();
    b.uv_scale = 1.0 / 6.0;
    b.tex(MN_DARK).prism(Vec3::ZERO, -3.0, 2.0, 0.9, 0.9, 6);
    b.tex(VK_A).prism(Vec3::ZERO, -4.5, -3.0, 0.0, 0.9, 6);
    b.mirror_x(|b| {
        b.cube(v3(1.1, 0.0, 1.5), v3(0.7, 0.1, 0.8));
    });
    b.cube(v3(0.0, 1.1, 1.5), v3(0.1, 0.7, 0.8));
    b.glow(v3(0.0, 0.0, 2.6), 3.0);
    b.finish("torpedo")
}

fn missile() -> Model {
    let mut b = Builder::new();
    b.uv_scale = 1.0 / 8.0;
    b.tex(VK_TRIM).prism(Vec3::ZERO, -2.5, 2.5, 0.45, 0.45, 6);
    b.tex(VK_A).prism(Vec3::ZERO, -4.0, -2.5, 0.0, 0.45, 6);
    b.mirror_x(|b| {
        b.cube(v3(0.8, 0.0, 2.0), v3(0.5, 0.05, 0.6));
    });
    b.cube(v3(0.0, 0.8, 2.0), v3(0.05, 0.5, 0.6));
    b.glow(v3(0.0, 0.0, 3.2), 2.0);
    b.finish("missile")
}

fn mining_bot() -> Model {
    let mut b = Builder::new();
    b.uv_scale = 1.0 / 10.0;
    b.tex(MN_DARK).cube(Vec3::ZERO, v3(1.6, 1.2, 2.0));
    b.tex(VK_A).mirror_x(|b| {
        b.cube(v3(2.6, 0.0, -2.0), v3(0.2, 0.2, 2.2));
        b.prism(v3(2.6, 0.0, -4.4), -0.8, 0.8, 0.0, 0.8, 6);
    });
    b.tex(MN_A).prism(v3(0.0, 0.0, 0.0), 2.0, 3.0, 1.0, 0.8, 6);
    b.glow(v3(0.0, 0.0, 3.4), 2.0);
    b.finish("mining_bot")
}

fn escape_pod() -> Model {
    let mut b = Builder::new();
    b.uv_scale = 1.0 / 8.0;
    b.tex(MN_A).sphere(Vec3::ZERO, 2.2, 8, 6);
    b.tex(MN_DARK)
        .prism(v3(0.0, 0.0, 2.0), 0.0, 1.5, 1.2, 1.0, 6);
    b.glow(v3(0.0, 0.0, 4.0), 2.0);
    b.finish("escape_pod")
}

fn cargo_crate() -> Model {
    let mut b = Builder::new();
    b.uv_scale = 1.0 / 3.0;
    b.tex("cargo_container_a")
        .cube(Vec3::ZERO, v3(1.5, 1.5, 3.0));
    b.finish("cargo_crate")
}

pub fn all() -> Vec<Model> {
    vec![
        viking("longship", 1.0, 1, VK_A),
        skiff(),
        viking("drakkar", 1.5, 2, VK_B),
        hauler(),
        forge(),
        fort(),
        minion("interceptor", 22.0, 4.0, 1.4, true, 0, MN_A),
        minion("cutter", 38.0, 6.0, 2.4, true, 2, MN_A),
        minion("frigate", 70.0, 9.0, 4.5, false, 2, MN_B),
        minion("destroyer", 110.0, 12.0, 6.0, true, 3, MN_A),
        dreadnought(),
        prison_barge(),
        torpedo(),
        missile(),
        mining_bot(),
        escape_pod(),
        cargo_crate(),
    ]
}
