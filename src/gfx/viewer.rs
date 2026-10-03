//! Model viewer (debug): `cargo run -- --viewer [model]` cycles every generated mesh, textured.

use macroquad::prelude::*;

use super::pixfont;
use super::render3d::{Pose, Renderer, Shade, draw_billboard, draw_model};
use crate::assets::Assets;

pub struct Viewer {
    index: usize,
    angle: f32,
    pub paused: bool,
}

/// Texture for parts that rely on the entity texture.
fn default_texture(model: &str) -> &'static str {
    match model {
        "sphere" | "sphere_lo" => "earth",
        "asteroid_a" => "asteroid_a",
        "asteroid_b" => "asteroid_b",
        "asteroid_c" => "asteroid_c",
        "ring" => "ring_saturn",
        "quad" => "pickup_health_l",
        _ => "hull_viking_a",
    }
}

impl Viewer {
    pub fn new(assets: &Assets, start: Option<&str>) -> Viewer {
        let index = start
            .and_then(|n| assets.model_names().iter().position(|m| *m == n))
            .unwrap_or(0);
        Viewer {
            index,
            angle: 0.6,
            paused: false,
        }
    }

    pub fn update(&mut self, assets: &Assets, dt: f32) {
        let n = assets.model_names().len().max(1);
        if is_key_pressed(KeyCode::Right) || is_key_pressed(KeyCode::D) {
            self.index = (self.index + 1) % n;
        }
        if is_key_pressed(KeyCode::Left) || is_key_pressed(KeyCode::A) {
            self.index = (self.index + n - 1) % n;
        }
        if is_key_pressed(KeyCode::Space) {
            self.paused = !self.paused;
        }
        if !self.paused {
            self.angle += dt * 0.6;
        }
    }

    pub fn draw(&self, assets: &Assets, r: &mut Renderer) {
        r.ensure_target();
        let Some(&name) = assets.model_names().get(self.index) else {
            return;
        };
        let Some(model) = assets.model(name) else {
            return;
        };
        let radius = model.radius.max(0.5);
        let sun = Vec3::new(-0.5, 0.6, 0.6).normalize();
        let eye = Vec3::new(0.0, radius * 0.9, radius * 2.7);
        let cam = r.camera(eye, Vec3::ZERO, Vec3::Y, 45.0);
        set_camera(&cam);
        clear_background(Color::new(0.05, 0.06, 0.09, 1.0));
        let rot = Quat::from_rotation_y(self.angle) * Quat::from_rotation_x(-0.15);
        let smooth = name.starts_with("sphere");
        draw_model(
            assets,
            model,
            default_texture(name),
            Pose::new(Vec3::ZERO, rot, 1.0),
            Shade::new(sun, !smooth, true),
        );
        let (right, up) = (Vec3::X, Vec3::Y);
        r.additive(|| {
            if let Some(g) = assets.model(name).map(|m| &m.glows) {
                for (p, size) in g {
                    draw_billboard(
                        assets.tex("glow_engine"),
                        rot * *p,
                        *size,
                        Color::new(0.8, 0.9, 1.0, 0.9),
                        right,
                        up,
                    );
                }
            }
        });
        r.present();
        let fs = 2.0;
        pixfont::draw(
            assets,
            &format!(
                "{} / {}  {}",
                self.index + 1,
                assets.model_names().len(),
                name.to_uppercase()
            ),
            16.0,
            14.0,
            fs,
            Color::new(1.0, 0.8, 0.5, 1.0),
        );
        let parts: Vec<&str> = model
            .parts
            .iter()
            .map(|p| {
                if p.texture.is_empty() {
                    "(entity)"
                } else {
                    p.texture
                }
            })
            .collect();
        pixfont::draw(
            assets,
            &format!(
                "{} TRIS  R {:.1}  {}",
                model.triangle_count(),
                model.radius,
                parts.join(" ")
            ),
            16.0,
            14.0 + 16.0,
            fs,
            Color::new(0.6, 0.8, 0.9, 1.0),
        );
        pixfont::draw(
            assets,
            "LEFT/RIGHT MODEL  SPACE PAUSE  ESC MENU",
            16.0,
            screen_height() - 22.0,
            fs,
            Color::new(0.8, 0.8, 0.8, 0.9),
        );
    }
}
