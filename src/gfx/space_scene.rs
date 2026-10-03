//! The space-flight scene: owns the sim `World`, turns keyboard input into `Command`s, steps the
//! sim at a fixed rate and renders it (sky, sun, planets, ships, effects) plus the HUD.

use macroquad::models::{Mesh, Vertex, draw_mesh};
use macroquad::prelude::*;

use super::cockpit;
use super::hud::{self, HudData};
use super::render3d::{Pose, Renderer, Shade, draw_billboard, draw_model, draw_streak};
use crate::assets::Assets;
use crate::sim::event::Event;
use crate::sim::flight::{FORWARD, RIGHT, UP};
use crate::sim::sector::{SectorId, SkyKind};
use crate::sim::{Addressed, Command, EntityId, Kind, Stepper, World, new_game};

const FOV: f32 = 68.0;
const SKY_DIST: f32 = 250_000.0;
const DUST_COUNT: usize = 140;
const DUST_BOX: f32 = 1000.0;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum CamMode {
    Cockpit,
    Chase,
}

struct Star {
    dir: Vec3,
    size: f32,
    color: [u8; 3],
}

pub struct SpaceScene {
    pub world: World,
    stepper: Stepper,
    mode: CamMode,
    target: Option<EntityId>,
    cam_rot: Quat,
    cam_pos: Vec3,
    view_proj: Mat4,
    stars: Vec<Star>,
    messages: Vec<(String, f32)>,
    help_left: f32,
    t: f32,
}

fn star_field() -> Vec<Star> {
    let mut rng = crate::sim::rng::Rng::new(0x57A2_F1E1);
    (0..2600)
        .map(|_| {
            let b = rng.f32();
            let (size, lum) = if b > 0.985 {
                (2.0, 1.0)
            } else if b > 0.9 {
                (1.0, 0.95)
            } else {
                (1.0, 0.45 + 0.4 * rng.f32())
            };
            let tint = rng.f32();
            let c = if tint < 0.25 {
                [0.7, 0.82, 1.0]
            } else if tint > 0.85 {
                [1.0, 0.8, 0.6]
            } else {
                [1.0, 1.0, 1.0]
            };
            Star {
                dir: rng.unit_vec(),
                size,
                color: c.map(|v| (v * lum * 255.0) as u8),
            }
        })
        .collect()
}

impl SpaceScene {
    pub fn new(sector: SectorId, seed: u64) -> SpaceScene {
        let world = new_game(seed, sector);
        let rot = world
            .player
            .and_then(|p| world.get(p))
            .map_or(Quat::IDENTITY, |e| e.rot);
        // Look toward the sector's main body at start.
        SpaceScene {
            world,
            stepper: Stepper::default(),
            mode: CamMode::Cockpit,
            target: None,
            cam_rot: rot,
            cam_pos: Vec3::ZERO,
            view_proj: Mat4::IDENTITY,
            stars: star_field(),
            messages: Vec::new(),
            help_left: 18.0,
            t: 0.0,
        }
    }

    fn player_id(&self) -> Option<EntityId> {
        self.world.player
    }

    fn cycle_target(&mut self, dir: i32) {
        let Some(me) = self.player_id() else { return };
        let list: Vec<EntityId> = self
            .world
            .nearest(me, 24, 120_000.0)
            .into_iter()
            .filter(|id| {
                self.world
                    .get(*id)
                    .is_some_and(|e| e.kind != Kind::Asteroid || e.radius > 100.0)
            })
            .collect();
        if list.is_empty() {
            self.target = None;
            return;
        }
        let cur = self.target.and_then(|t| list.iter().position(|&i| i == t));
        let next = match cur {
            Some(i) => (i as i32 + dir).rem_euclid(list.len() as i32) as usize,
            None => 0,
        };
        self.target = Some(list[next]);
    }

    /// Read input, run the fixed-step sim, update camera and message timers.
    pub fn update(&mut self, dt: f32) {
        self.t += dt;
        self.help_left = (self.help_left - dt).max(0.0);
        let Some(me) = self.player_id() else { return };
        let k = is_key_down;
        let axis = |neg: KeyCode, pos: KeyCode| (k(pos) as i32 - k(neg) as i32) as f32;
        let mut cmds: Vec<Addressed> = Vec::new();
        let mut push = |cmd: Command| cmds.push(Addressed { ship: me, cmd });

        let thr = axis(KeyCode::S, KeyCode::W);
        if thr != 0.0 {
            push(Command::AdjustThrottle(thr * 0.55 * dt));
        }
        if is_key_pressed(KeyCode::X) {
            push(Command::SetThrottle(0.0));
        }
        if is_key_pressed(KeyCode::LeftShift) {
            push(Command::SetThrottle(1.0));
        }
        push(Command::Turn {
            pitch: axis(KeyCode::Up, KeyCode::Down), // down = pull back = nose up
            yaw: axis(KeyCode::D, KeyCode::A),       // A = left = positive yaw
            roll: axis(KeyCode::Q, KeyCode::E),
        });
        push(Command::Strafe {
            x: axis(KeyCode::Left, KeyCode::Right),
            y: axis(KeyCode::F, KeyCode::R),
        });
        push(Command::Brake(k(KeyCode::Space)));
        if is_key_pressed(KeyCode::M) {
            let assist = self
                .world
                .get(me)
                .and_then(|e| e.ship)
                .is_some_and(|s| s.assist);
            push(Command::SetAssist(!assist));
        }
        if is_key_pressed(KeyCode::C) {
            self.mode = if self.mode == CamMode::Cockpit {
                CamMode::Chase
            } else {
                CamMode::Cockpit
            };
        }
        if is_key_pressed(KeyCode::H) {
            self.help_left = if self.help_left > 0.0 { 0.0 } else { 18.0 };
        }
        if is_key_pressed(KeyCode::T) {
            self.cycle_target(1);
        }
        if is_key_pressed(KeyCode::G) {
            self.target = None;
        }

        let steps = self.stepper.advance(dt);
        let mut first = true;
        for _ in 0..steps {
            let events = self.world.step(if first { &cmds } else { &[] });
            first = false;
            for e in events {
                if let Event::Collision { other, speed, .. } = e
                    && speed > 25.0
                {
                    let name = self.world.get(other).map_or("OBJECT", |o| o.label);
                    self.messages.push((
                        format!("COLLISION - {} ({:.0} U/S)", name.to_uppercase(), speed),
                        3.0,
                    ));
                }
            }
        }
        if steps == 0 && first {
            // keep demands fresh even when no sim step ran this frame
            for a in &cmds {
                if let Some(s) = self.world.get_mut(a.ship).and_then(|e| e.ship.as_mut()) {
                    s.apply(a.cmd);
                }
            }
        }
        for m in &mut self.messages {
            m.1 -= dt;
        }
        self.messages.retain(|m| m.1 > 0.0);
        if self.target.is_some_and(|t| self.world.get(t).is_none()) {
            self.target = None;
        }
        if let Some(e) = self.world.get(me) {
            self.cam_rot = self
                .cam_rot
                .slerp(e.rot, 1.0 - (-7.0 * dt).exp())
                .normalize();
        }
    }

    /// Render the frame: pixel-target 3D scene, post-process, then the crisp HUD.
    pub fn draw(&mut self, assets: &Assets, r: &mut Renderer) {
        let (iw, ih) = r.ensure_target();
        let Some(me_id) = self.player_id() else {
            return;
        };
        let alpha = self.stepper.alpha();
        let Some(me) = self.world.get(me_id) else {
            return;
        };
        let me_pos = me.prev_pos.lerp(me.pos, alpha);
        let me_rot = me.prev_rot.slerp(me.rot, alpha);
        let throttle = me.ship.map_or(0.0, |s| s.throttle);
        let vel = me.vel;

        let (cam_pos, cam_rot) = match self.mode {
            CamMode::Cockpit => (me_pos + me_rot * Vec3::new(0.0, 3.6, -8.5), me_rot),
            CamMode::Chase => {
                let rot = self.cam_rot;
                (me_pos + rot * Vec3::new(0.0, 24.0, 78.0), rot)
            }
        };
        self.cam_pos = cam_pos;
        let fwd = cam_rot * FORWARD;
        let (right, up) = (cam_rot * RIGHT, cam_rot * UP);
        let target_pt = match self.mode {
            CamMode::Cockpit => cam_pos + fwd,
            CamMode::Chase => me_pos + cam_rot * Vec3::new(0.0, 4.0, -60.0),
        };
        let look = (target_pt - cam_pos).normalize_or_zero();
        let cam = r.camera(cam_pos, cam_pos + look, up, FOV);
        self.view_proj = {
            use macroquad::camera::Camera;
            cam.matrix()
        };
        set_camera(&cam);
        clear_background(Color::new(0.008, 0.008, 0.02, 1.0));

        let sun = self.world.sun_dir();
        let sphere = assets.model("sphere");
        let sphere_lo = assets.model("sphere_lo");

        // --- sky: nebula, stars, distant bodies
        if let Some(m) = sphere {
            draw_model(
                assets,
                m,
                "sky_nebula",
                Pose::new(cam_pos, Quat::from_rotation_y(0.9), SKY_DIST * 1.12),
                Shade::new(sun, false, false),
            );
        }
        self.draw_stars(cam_pos, right, up, ih);
        for b in &self.world.sky {
            let radius = SKY_DIST * b.ang_radius.tan();
            let pos = cam_pos + b.dir * SKY_DIST;
            if (b.dir.dot(look)) < -0.2 {
                continue;
            }
            match (b.kind, sphere, sphere_lo) {
                (SkyKind::Star, Some(m), _) => draw_model(
                    assets,
                    m,
                    b.texture,
                    Pose::new(pos, Quat::IDENTITY, radius),
                    Shade::new(sun, false, false),
                ),
                (SkyKind::Planet, _, Some(m)) => {
                    draw_model(
                        assets,
                        m,
                        b.texture,
                        Pose::new(pos, Quat::from_rotation_y(0.5), radius),
                        Shade::new(sun, false, true),
                    );
                    if b.name == "Saturn"
                        && let Some(ring) = assets.model("ring")
                    {
                        draw_model(
                            assets,
                            ring,
                            "ring_saturn",
                            Pose::new(pos, Quat::from_rotation_x(0.45), radius),
                            Shade::new(sun, false, false),
                        );
                    }
                }
                _ => {}
            }
        }

        // --- world entities
        for (id, e) in self.world.iter() {
            if id == me_id && self.mode == CamMode::Cockpit {
                continue;
            }
            let pos = e.prev_pos.lerp(e.pos, alpha);
            let rot = e.prev_rot.slerp(e.rot, alpha);
            let rel = pos - cam_pos;
            let dist = rel.length();
            if dist > e.radius * 1.2 + 200.0 && rel.dot(look) < -e.radius * 1.5 {
                continue;
            }
            if e.kind == Kind::Asteroid && e.radius < 100.0 && dist > 9000.0 {
                continue;
            }
            let (model_name, scale) = match e.model {
                "sphere" | "asteroid_a" | "asteroid_b" | "asteroid_c" => {
                    let lod = e.model == "sphere" && e.radius / dist.max(1.0) < 0.012;
                    (if lod { "sphere_lo" } else { e.model }, e.radius)
                }
                m => (m, 1.0),
            };
            if let Some(m) = assets.model(model_name) {
                let smooth = e.model == "sphere";
                draw_model(
                    assets,
                    m,
                    e.texture,
                    Pose::new(pos, rot, scale),
                    Shade::new(sun, !smooth, true),
                );
            }
        }

        // --- additive effects: planet halos, sun glow, engine glows, dust
        let glow = assets.tex("glow_soft");
        let eng = assets.tex("glow_engine");
        r.additive(|| {
            for b in &self.world.sky {
                if b.kind == SkyKind::Star && b.dir.dot(look) > 0.0 {
                    let radius = SKY_DIST * b.ang_radius.tan();
                    let pos = cam_pos + b.dir * (SKY_DIST - 100.0);
                    draw_billboard(
                        glow,
                        pos,
                        radius * 9.0,
                        Color::new(1.0, 0.8, 0.45, 0.55),
                        right,
                        up,
                    );
                    draw_billboard(
                        glow,
                        pos,
                        radius * 28.0,
                        Color::new(1.0, 0.6, 0.3, 0.14),
                        right,
                        up,
                    );
                }
            }
            for (id, e) in self.world.iter() {
                let pos = e.prev_pos.lerp(e.pos, alpha);
                let rot = e.prev_rot.slerp(e.rot, alpha);
                let rel = pos - cam_pos;
                if rel.dot(look) < -e.radius {
                    continue;
                }
                if e.kind == Kind::Planet {
                    let halo = match e.label {
                        "Mars" => Some(Color::new(1.0, 0.5, 0.3, 0.30)),
                        "Earth" => Some(Color::new(0.4, 0.7, 1.0, 0.42)),
                        "Jupiter" => Some(Color::new(1.0, 0.8, 0.55, 0.25)),
                        _ => None,
                    };
                    if let Some(c) = halo {
                        let behind = pos + rel.normalize_or_zero() * e.radius * 0.35;
                        draw_billboard(glow, behind, e.radius * 2.55, c, right, up);
                    }
                }
                let own = id == me_id;
                if own && self.mode == CamMode::Cockpit {
                    continue;
                }
                if let Some(m) = assets.model(e.model) {
                    let level = if own {
                        0.35 + 0.9 * throttle.max(0.0) + 0.3 * (vel.length() / 300.0)
                    } else {
                        0.5
                    };
                    for (gp, size) in &m.glows {
                        let wp = pos + rot * *gp;
                        let flick = 1.0 + 0.08 * (self.t * 31.0 + gp.x).sin();
                        draw_billboard(
                            eng,
                            wp,
                            size * level * flick,
                            Color::new(0.8, 0.9, 1.0, 0.9),
                            right,
                            up,
                        );
                    }
                }
            }
            // space dust: streaks that show speed and direction of travel
            let streak = vel * 0.02;
            for i in 0..DUST_COUNT {
                let h = |k: i32| crate::texgen::noise::hash(i as i32, k, 3, 99);
                let base = Vec3::new(h(0), h(1), h(2));
                let cell = (base - cam_pos / DUST_BOX).fract_gl();
                let p = cam_pos + (cell - Vec3::splat(0.5)) * DUST_BOX;
                let dist = p.distance(cam_pos);
                if (p - cam_pos).dot(look) < 0.0 || dist < 25.0 {
                    continue;
                }
                let fade = (1.0 - dist / (DUST_BOX * 0.5)).clamp(0.0, 1.0);
                let width = dist * 0.0042;
                draw_streak(
                    p,
                    p - streak - Vec3::splat(0.001),
                    width,
                    Color::new(0.7, 0.8, 1.0, 0.7 * fade),
                    cam_pos,
                );
            }
        });

        // --- cockpit frame in the pixel target
        if self.mode == CamMode::Cockpit {
            set_camera(&r.camera_2d());
            cockpit::draw(iw as f32, ih as f32, self.t, throttle);
        }

        r.present();
        let ui_scale = ((r.scale as f32) * 2.0 / 3.0).round().max(2.0);
        hud::draw_space(
            assets,
            &HudData {
                world: &self.world,
                player: me_id,
                target: self.target,
                view_proj: self.view_proj,
                cockpit: self.mode == CamMode::Cockpit,
                messages: &self.messages,
                show_help: self.help_left > 0.0,
                fov_deg: FOV,
                ui_scale,
            },
        );
    }

    fn draw_stars(&self, cam_pos: Vec3, right: Vec3, up: Vec3, ih: u32) {
        let px = FOV.to_radians() / ih as f32;
        let dist = SKY_DIST * 1.05;
        let mut verts = Vec::with_capacity(self.stars.len() * 4);
        let mut idx: Vec<u16> = Vec::with_capacity(self.stars.len() * 6);
        for s in &self.stars {
            let c = cam_pos + s.dir * dist;
            let half = dist * px * s.size * 0.5;
            let (r, u) = (right * half, up * half);
            let col = [s.color[0], s.color[1], s.color[2], 255];
            let base = verts.len() as u16;
            for p in [c - r + u, c + r + u, c + r - u, c - r - u] {
                verts.push(Vertex {
                    position: p,
                    uv: vec2(0.5, 0.5),
                    color: col,
                    normal: Vec4::ZERO,
                });
            }
            idx.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
        }
        // chunks keep indices inside u16
        for (v, i) in verts.chunks(60_000).zip(idx.chunks(90_000)) {
            draw_mesh(&Mesh {
                vertices: v.to_vec(),
                indices: i.to_vec(),
                texture: None,
            });
        }
    }

    /// Developer hook: aim the camera and spawn point for screenshots.
    pub fn debug_place(&mut self, pos: Vec3, look_at: Vec3) {
        if let Some(e) = self.world.player.and_then(|p| self.world.get_mut(p)) {
            e.pos = pos;
            e.prev_pos = pos;
            let dir = (look_at - pos).normalize_or_zero();
            e.rot = Quat::from_rotation_arc(FORWARD, dir);
            e.prev_rot = e.rot;
            self.cam_rot = e.rot;
        }
    }

    /// Developer hook: start already moving at `throttle` (screenshots of speed effects).
    pub fn debug_throttle(&mut self, throttle: f32) {
        if let Some(e) = self.world.player.and_then(|p| self.world.get_mut(p))
            && let Some(s) = e.ship.as_mut()
        {
            s.throttle = throttle;
            e.vel = e.rot * FORWARD * throttle * s.class.max_speed;
        }
    }

    pub fn set_chase(&mut self, chase: bool) {
        self.mode = if chase {
            CamMode::Chase
        } else {
            CamMode::Cockpit
        };
    }

    pub fn cycle_target_for_shot(&mut self) {
        self.cycle_target(1);
    }
}
