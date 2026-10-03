//! Boarding-action models: boxy people (separate skin per part), mechs, drones, props, pickups.
//! Units are metres; characters face −Z and are ~1.8 m tall with feet at y = 0.

use glam::Vec3;

use super::{Builder, Model};

fn v3(x: f32, y: f32, z: f32) -> Vec3 {
    Vec3::new(x, y, z)
}

/// Humanoid built from boxes. `armor`/`visor` are the skin textures for each body part.
fn person(
    name: &'static str,
    armor: &'static str,
    visor: &'static str,
    scale: f32,
    bulk: f32,
) -> Model {
    let mut b = Builder::new();
    b.uv_scale = 1.0 / 0.9;
    b.m = glam::Mat4::from_scale(Vec3::splat(scale));
    b.tex(armor);
    // legs, torso, arms
    b.mirror_x(|b| {
        b.cube(
            v3(0.17 * bulk, 0.45, 0.0),
            v3(0.12 * bulk, 0.45, 0.13 * bulk),
        );
        b.cube(v3(0.43 * bulk, 1.25, 0.0), v3(0.1 * bulk, 0.34, 0.1 * bulk));
    });
    b.cube(v3(0.0, 1.25, 0.0), v3(0.3 * bulk, 0.36, 0.17 * bulk));
    b.cube(v3(0.0, 0.92, 0.0), v3(0.26 * bulk, 0.1, 0.15 * bulk));
    // helmet + visor
    b.cube(v3(0.0, 1.78, 0.0), v3(0.17, 0.17, 0.17));
    b.tex(visor)
        .cube(v3(0.0, 1.78, -0.16), v3(0.13, 0.06, 0.03));
    // weapon in the right hand
    b.tex(armor)
        .cube(v3(-0.45 * bulk, 1.18, -0.35), v3(0.05, 0.07, 0.35));
    b.finish(name)
}

fn mech() -> Model {
    let mut b = Builder::new();
    b.uv_scale = 1.0 / 2.0;
    b.tex("minion_mech");
    b.mirror_x(|b| {
        b.cube(v3(0.7, 1.1, 0.0), v3(0.25, 1.1, 0.3));
        b.cube(v3(0.7, 2.5, 0.0), v3(0.3, 0.2, 0.35));
        b.cube(v3(1.15, 3.1, -0.2), v3(0.22, 0.3, 0.22));
        b.cube(v3(1.15, 2.7, -0.7), v3(0.12, 0.12, 0.7));
    });
    b.cube(v3(0.0, 3.0, 0.0), v3(0.8, 0.7, 0.5));
    b.cube(v3(0.0, 2.35, 0.0), v3(0.5, 0.2, 0.4));
    b.cube(v3(0.0, 3.9, -0.1), v3(0.35, 0.3, 0.35));
    b.tex("minion_visor")
        .cube(v3(0.0, 3.92, -0.45), v3(0.3, 0.1, 0.03));
    b.finish("mech")
}

fn drone() -> Model {
    let mut b = Builder::new();
    b.uv_scale = 1.0;
    b.tex("drone_body").sphere(Vec3::ZERO, 0.4, 8, 6);
    b.mirror_x(|b| {
        b.cube(v3(0.6, 0.0, 0.0), v3(0.35, 0.03, 0.2));
    });
    b.tex("minion_visor")
        .cube(v3(0.0, 0.0, -0.38), v3(0.1, 0.1, 0.03));
    b.finish("drone")
}

fn door_frame() -> Model {
    let mut b = Builder::new();
    b.uv_scale = 1.0 / 2.0;
    b.tex("wall_minion_a");
    b.mirror_x(|b| {
        b.cube(v3(1.2, 1.4, 0.0), v3(0.2, 1.4, 0.3));
    });
    b.cube(v3(0.0, 2.7, 0.0), v3(1.4, 0.15, 0.3));
    b.tex("door_minion");
    b.cube(v3(0.0, 1.25, 0.0), v3(1.0, 1.25, 0.1));
    b.finish("door_frame")
}

fn console() -> Model {
    let mut b = Builder::new();
    b.uv_scale = 1.0 / 1.2;
    b.tex("console_a")
        .cube(v3(0.0, 0.5, 0.0), v3(0.6, 0.5, 0.35));
    b.tex("terminal_screen")
        .cube(v3(0.0, 1.15, 0.12), v3(0.5, 0.3, 0.04));
    b.finish("console")
}

fn crate_model() -> Model {
    let mut b = Builder::new();
    b.uv_scale = 1.0;
    b.tex("crate_a").cube(v3(0.0, 0.5, 0.0), v3(0.5, 0.5, 0.5));
    b.finish("crate")
}

fn barrel() -> Model {
    let mut b = Builder::new();
    b.uv_scale = 1.0;
    let m = glam::Mat4::from_rotation_x(std::f32::consts::FRAC_PI_2);
    b.m = glam::Mat4::from_translation(v3(0.0, 0.45, 0.0)) * m;
    b.tex("barrel")
        .prism(Vec3::ZERO, -0.45, 0.45, 0.32, 0.32, 10);
    b.finish("barrel")
}

fn reactor() -> Model {
    let mut b = Builder::new();
    b.uv_scale = 1.0;
    b.m = glam::Mat4::from_translation(v3(0.0, 1.5, 0.0))
        * glam::Mat4::from_rotation_x(std::f32::consts::FRAC_PI_2);
    b.tex("reactor_core")
        .prism(Vec3::ZERO, -1.5, 1.5, 0.9, 0.9, 12);
    b.tex("wall_reactor")
        .prism(Vec3::ZERO, -1.5, -1.3, 1.2, 1.2, 12);
    b.prism(Vec3::ZERO, 1.3, 1.5, 1.2, 1.2, 12);
    b.finish("reactor")
}

pub fn all() -> Vec<Model> {
    vec![
        person("minion_grunt", "minion_armor", "minion_visor", 1.0, 1.0),
        person("minion_guard", "minion_heavy", "minion_visor", 1.05, 1.15),
        person("minion_gunner", "minion_heavy", "minion_visor", 1.1, 1.3),
        person("minion_medic", "minion_medic", "minion_visor", 1.0, 1.0),
        person(
            "minion_enforcer",
            "minion_enforcer",
            "minion_visor",
            1.2,
            1.5,
        ),
        person("prefect_voss", "prefect_voss", "minion_visor", 1.35, 1.6),
        person("marine", "marine_armor", "marine_helmet", 1.0, 1.1),
        mech(),
        drone(),
        door_frame(),
        console(),
        crate_model(),
        barrel(),
        reactor(),
    ]
}
