//! First-person boarding scene (T19): textured level geometry batched per texture, vertex-baked
//! room lighting, boxy monsters, billboard pickups, viewmodel, HUD. Renders through the same
//! pixel-art target and post shader as the space scene.

use macroquad::models::{Mesh, Vertex, draw_mesh};
use macroquad::prelude::*;

use super::pixfont;
use super::render3d::{Pose, Renderer, Shade, draw_billboard, draw_billboard_uv, draw_model};
use super::sound::Mixer;
use crate::assets::Assets;
use crate::audio::Sfx;
use crate::board::bot::BoardBot;
use crate::board::level::{CELL, ItemKind, Key, Level, Objective, RoomKind, Theme, WALL_H};
use crate::board::monster::{State, stats};
use crate::board::pmove::Input;
use crate::board::weapons::Weapon;
use crate::board::{BEvent, BoardWorld, Outcome, generate};
use crate::cheats::Cheat;
use crate::sim::EntityId;

const FOV: f32 = 78.0;
const MAX_VERTS: usize = 60_000;
const STEP: f32 = 1.0 / 60.0;

/// What the space game tells the boarding scene about the ship being boarded.
#[derive(Debug, Clone)]
pub struct BoardInfo {
    pub theme: Theme,
    /// Level size `1..=5` (enemy class).
    pub size: u32,
    pub label: String,
    pub target: EntityId,
    pub seed: u64,
    /// Campaign missions demand one objective; `None` lets the player choose.
    pub force: Option<Objective>,
}

/// What came out of the boarding action.
#[derive(Debug, Clone)]
pub struct BoardResult {
    pub outcome: Outcome,
    pub objective: Objective,
    pub objective_done: bool,
    pub kills: u32,
    pub credits: i32,
    /// Health lost, used for crew casualties.
    pub damage_taken: f32,
}

#[derive(PartialEq, Eq)]
enum Phase {
    Prep,
    Play,
    Done,
}

struct Spark {
    pos: Vec3,
    age: f32,
    life: f32,
    size: f32,
    blast: bool,
}

pub struct BoardScene {
    pub info: BoardInfo,
    phase: Phase,
    choice: usize,
    world: Option<BoardWorld>,
    meshes: Vec<(&'static str, Mesh)>,
    sparks: Vec<Spark>,
    messages: Vec<(String, f32)>,
    acc: f32,
    last_mouse: Option<Vec2>,
    hurt: f32,
    flash: f32,
    t: f32,
    step_t: f32,
    health_start: f32,
    result: Option<BoardResult>,
    /// Developer: skip the objective screen and face the longest sightline.
    pub auto_start: bool,
    /// Enemy toughness scale from the settings.
    pub difficulty: f32,
    /// Mouse-look multiplier and Y inversion from the settings.
    pub sens: f32,
    pub invert_y: bool,
    /// AI shield-brothers fighting beside the player (`0..=2`, T33); `P` on the objective screen.
    pub partners: usize,
    bots: Vec<BoardBot>,
    /// Cheat: the boarding party cannot be hurt.
    pub god: bool,
}

/// One-element objective list for a campaign-forced objective.
fn single(o: Objective) -> &'static [Objective] {
    match o {
        Objective::Sabotage => &[Objective::Sabotage],
        Objective::CaptureBridge => &[Objective::CaptureBridge],
        Objective::Rescue => &[Objective::Rescue],
        Objective::StealData => &[Objective::StealData],
        Objective::KillBoss => &[Objective::KillBoss],
    }
}

impl BoardScene {
    pub fn new(info: BoardInfo) -> BoardScene {
        BoardScene {
            info,
            phase: Phase::Prep,
            choice: 0,
            world: None,
            meshes: Vec::new(),
            sparks: Vec::new(),
            messages: Vec::new(),
            acc: 0.0,
            last_mouse: None,
            hurt: 0.0,
            flash: 0.0,
            t: 0.0,
            step_t: 0.0,
            health_start: 100.0,
            result: None,
            auto_start: false,
            difficulty: 1.0,
            sens: 1.0,
            invert_y: false,
            partners: 0,
            bots: Vec::new(),
            god: false,
        }
    }

    /// Whether the mouse should be captured for looking around.
    pub fn wants_grab(&self) -> bool {
        self.phase == Phase::Play
    }

    /// Apply a console cheat to the running boarding action; returns the reply line.
    pub fn cheat(&mut self, c: Cheat) -> String {
        if let Cheat::God(v) = c {
            self.god = v.unwrap_or(!self.god);
            return format!("GOD MODE {}", if self.god { "ON" } else { "OFF" });
        }
        if self.phase != Phase::Play {
            return "start the boarding first (pick an objective)".into();
        }
        let Some(w) = self.world.as_mut() else {
            return "no boarding in progress".into();
        };
        match c {
            Cheat::Heal => {
                w.player.health = w.player.max_health;
                w.player.armor = 200;
                "HEALTH AND ARMOUR FULL".into()
            }
            Cheat::Ammo => {
                w.player.weapons = [true; 6];
                for a in crate::board::weapons::Ammo::ALL {
                    w.player.add_ammo(a, a.max());
                }
                w.player.keys = [true; 3];
                "ALL WEAPONS, AMMO AND KEYS".into()
            }
            Cheat::Kill => format!("{} MONSTERS KILLED", w.kill_all()),
            Cheat::Win => {
                w.force_objective();
                self.finish(Outcome::Won);
                "BOARDING WON".into()
            }
            Cheat::Lose => {
                w.player.health = 0.0;
                self.finish(Outcome::Died);
                "BOARDING LOST".into()
            }
            Cheat::Credits(_) => "credits only work in space".into(),
            _ => "not a boarding command".into(),
        }
    }

    pub fn result(&self) -> Option<&BoardResult> {
        self.result.as_ref()
    }

    fn objectives(&self) -> &'static [Objective] {
        match self.info.force {
            Some(o) => single(o),
            None => Objective::for_theme(self.info.theme),
        }
    }

    fn start(&mut self, assets: &Assets) {
        let obj = self.objectives()[self.choice.min(self.objectives().len() - 1)];
        let level = generate(self.info.seed, self.info.theme, obj, self.info.size);
        self.meshes = build_meshes(&level, assets);
        let mut world = BoardWorld::new(level, self.info.seed);
        world.difficulty = self.difficulty;
        world.add_mates(self.partners);
        self.bots = (0..self.partners).map(|_| BoardBot::new(0.8)).collect();
        self.health_start = world.player.health;
        self.world = Some(world);
        if self.auto_start
            && let Some(w) = self.world.as_mut()
        {
            // face the direction with the longest clear line of sight (for screenshots)
            let eye = w.player.eye();
            let best = (0..16)
                .map(|i| i as f32 * std::f32::consts::TAU / 16.0)
                .max_by(|a, b| {
                    let reach = |yaw: f32| {
                        let d = Vec3::new(-yaw.sin(), 0.0, -yaw.cos());
                        (1..120)
                            .take_while(|k| w.level.los(eye, eye + d * *k as f32 * 0.5))
                            .count()
                    };
                    reach(*a).cmp(&reach(*b))
                })
                .unwrap_or(0.0);
            w.player.yaw = best;
        }
        self.phase = Phase::Play;
        self.messages
            .push((format!("OBJECTIVE: {}", obj.name()), 6.0));
    }

    fn finish(&mut self, outcome: Outcome) {
        if let Some(w) = &self.world {
            let done = w.progress.done;
            let credits = w.progress.kills as i32 * 15
                + if done && outcome == Outcome::Won {
                    300
                } else {
                    0
                };
            self.result = Some(BoardResult {
                outcome,
                objective: w.level.objective,
                objective_done: done,
                kills: w.progress.kills,
                credits,
                damage_taken: (self.health_start - w.player.health).max(0.0),
            });
        } else {
            self.result = Some(BoardResult {
                outcome,
                objective: self.objectives()[0],
                objective_done: false,
                kills: 0,
                credits: 0,
                damage_taken: 0.0,
            });
        }
        self.phase = Phase::Done;
    }

    pub fn update(&mut self, dt: f32, assets: &Assets, snd: Option<&Mixer>) {
        self.t += dt;
        self.hurt = (self.hurt - dt * 1.5).max(0.0);
        self.flash = (self.flash - dt * 6.0).max(0.0);
        for s in &mut self.sparks {
            s.age += dt;
        }
        self.sparks.retain(|s| s.age < s.life);
        for m in &mut self.messages {
            m.1 -= dt;
        }
        self.messages.retain(|m| m.1 > 0.0);
        match self.phase {
            Phase::Prep => {
                if self.auto_start {
                    self.start(assets);
                    return;
                }
                let n = self.objectives().len();
                if is_key_pressed(KeyCode::Down) || is_key_pressed(KeyCode::S) {
                    self.choice = (self.choice + 1) % n;
                }
                if is_key_pressed(KeyCode::Up) || is_key_pressed(KeyCode::W) {
                    self.choice = (self.choice + n - 1) % n;
                }
                for (i, k) in [KeyCode::Key1, KeyCode::Key2, KeyCode::Key3]
                    .iter()
                    .enumerate()
                {
                    if i < n && is_key_pressed(*k) {
                        self.choice = i;
                        self.start(assets);
                        return;
                    }
                }
                if is_key_pressed(KeyCode::P) {
                    self.partners = (self.partners + 1) % 3;
                }
                if is_key_pressed(KeyCode::Enter) {
                    self.start(assets);
                } else if is_key_pressed(KeyCode::Escape) {
                    self.finish(Outcome::Fled);
                }
            }
            Phase::Play => self.play(dt, snd),
            Phase::Done => {}
        }
    }

    fn play(&mut self, dt: f32, snd: Option<&Mixer>) {
        if is_key_pressed(KeyCode::Escape) {
            self.finish(Outcome::Fled);
            return;
        }
        let Some(w) = self.world.as_mut() else { return };
        if self.god {
            w.player.health = w.player.max_health;
        }
        // mouse look (pixel deltas; the cursor is grabbed while playing)
        let (mx, my) = mouse_position();
        let m = vec2(mx, my);
        if let Some(l) = self.last_mouse {
            let d = m - l;
            let k = 0.0028 * self.sens;
            let dy = if self.invert_y { -d.y } else { d.y };
            w.player.yaw -= d.x * k;
            w.player.pitch = (w.player.pitch - dy * k).clamp(-1.45, 1.45);
        }
        self.last_mouse = Some(m);
        let k = is_key_down;
        let axis = |neg: KeyCode, pos: KeyCode| (k(pos) as i32 - k(neg) as i32) as f32;
        let mut input = Input {
            forward: axis(KeyCode::S, KeyCode::W),
            right: axis(KeyCode::A, KeyCode::D),
            jump: k(KeyCode::Space),
            crouch: k(KeyCode::C) || k(KeyCode::LeftControl),
            fire: is_mouse_button_down(MouseButton::Left) || k(KeyCode::Enter),
            use_key: false,
            weapon: None,
        };
        for (i, key) in [
            KeyCode::Key1,
            KeyCode::Key2,
            KeyCode::Key3,
            KeyCode::Key4,
            KeyCode::Key5,
            KeyCode::Key6,
        ]
        .iter()
        .enumerate()
        {
            if is_key_pressed(*key) {
                input.weapon = Some(i);
            }
        }
        let use_pressed = is_key_pressed(KeyCode::E);
        self.acc += dt.min(0.25);
        let mut events = Vec::new();
        let mut use_left = use_pressed;
        while self.acc >= STEP {
            self.acc -= STEP;
            input.use_key = std::mem::take(&mut use_left);
            let mut inputs = vec![input];
            for (k, bot) in self.bots.iter_mut().enumerate() {
                inputs.push(w.with_player(k + 1, |w| bot.act(w)));
            }
            events.extend(w.step_team(STEP, &inputs));
            input.weapon = None;
            if w.outcome != Outcome::InProgress {
                break;
            }
        }
        let outcome = w.outcome;
        let p = w.player.pos;
        let speed = w.player.vel.length();
        self.step_t -= dt;
        if speed > 2.0 && w.player.on_ground && self.step_t <= 0.0 {
            self.step_t = 0.42;
            if let Some(s) = snd {
                s.play(Sfx::Footstep, 0.35);
            }
        }
        let loud = |snd: Option<&Mixer>, s: Sfx, at: Vec3, near: f32| {
            if let Some(m) = snd {
                let d = at.distance(p) / near;
                m.play(s, 1.0 / (1.0 + d * d));
            }
        };
        for e in events {
            match e {
                BEvent::Fired(wp) => {
                    self.flash = 1.0;
                    if let Some(m) = snd {
                        m.play(
                            match wp {
                                Weapon::Axe => Sfx::Door,
                                Weapon::Blaster | Weapon::Valkyrie => Sfx::Lance,
                                Weapon::Mjolnir => Sfx::Missile,
                                _ => Sfx::Gunshot,
                            },
                            0.8,
                        );
                    }
                }
                BEvent::Impact(pos) => self.sparks.push(Spark {
                    pos,
                    age: 0.0,
                    life: 0.25,
                    size: 0.5,
                    blast: false,
                }),
                BEvent::MonsterHurt(pos) => self.sparks.push(Spark {
                    pos,
                    age: 0.0,
                    life: 0.3,
                    size: 0.7,
                    blast: false,
                }),
                BEvent::Explosion(pos) => {
                    self.sparks.push(Spark {
                        pos,
                        age: 0.0,
                        life: 0.7,
                        size: 6.0,
                        blast: true,
                    });
                    loud(snd, Sfx::Explosion, pos, 14.0);
                }
                BEvent::MonsterDied(_, pos) => loud(snd, Sfx::Gunshot, pos, 10.0),
                BEvent::PlayerHurt(d) => {
                    self.hurt = (self.hurt + d / 40.0 + 0.2).min(0.8);
                    if let Some(m) = snd {
                        m.play(Sfx::ShieldHit, 0.6);
                    }
                }
                BEvent::Pickup => {
                    if let Some(m) = snd {
                        m.play(Sfx::Beep, 0.6);
                    }
                }
                BEvent::DoorMoved => loud(snd, Sfx::Door, p, 1.0),
                BEvent::Alarm => {
                    if let Some(m) = snd {
                        m.play(Sfx::Alarm, 0.8);
                    }
                }
                BEvent::Message(s) => self.messages.push((s, 3.5)),
                BEvent::ObjectiveDone => {
                    self.messages.push(("OBJECTIVE COMPLETE".into(), 4.0));
                    if let Some(m) = snd {
                        m.play(Sfx::Scan, 0.8);
                    }
                }
            }
        }
        if self.messages.len() > 6 {
            self.messages.remove(0);
        }
        if outcome != Outcome::InProgress {
            self.finish(outcome);
        }
    }

    /// Draw the scene (or the objective-selection screen).
    pub fn draw(&mut self, assets: &Assets, r: &mut Renderer) {
        let (sw, sh) = (screen_width(), screen_height());
        let ui = ((r.scale as f32) * 2.0 / 3.0).round().max(2.0);
        if self.phase == Phase::Prep || self.world.is_none() {
            clear_background(Color::new(0.03, 0.03, 0.06, 1.0));
            pixfont::draw_centered(
                assets,
                &format!("BOARDING: {}", self.info.label.to_uppercase()),
                sw / 2.0,
                sh * 0.2,
                ui * 2.0,
                Color::new(0.85, 0.36, 0.2, 1.0),
            );
            pixfont::draw_centered(
                assets,
                self.info.theme.name(),
                sw / 2.0,
                sh * 0.2 + 28.0 * ui,
                ui,
                Color::new(0.7, 0.7, 0.75, 1.0),
            );
            pixfont::draw_centered(
                assets,
                if self.info.force.is_some() {
                    "MISSION OBJECTIVE"
                } else {
                    "CHOOSE YOUR OBJECTIVE"
                },
                sw / 2.0,
                sh * 0.42,
                ui,
                Color::new(0.21, 0.78, 0.88, 1.0),
            );
            for (i, o) in self.objectives().iter().enumerate() {
                let sel = i == self.choice;
                let col = if sel {
                    Color::new(1.0, 0.85, 0.2, 1.0)
                } else {
                    Color::new(0.85, 0.8, 0.7, 1.0)
                };
                let line = format!("{} {}  {}", i + 1, if sel { ">" } else { " " }, o.name());
                pixfont::draw_centered(
                    assets,
                    &line,
                    sw / 2.0,
                    sh * 0.5 + i as f32 * 16.0 * ui,
                    ui,
                    col,
                );
            }
            pixfont::draw_centered(
                assets,
                "ENTER OR 1-3 TO BOARD   ESC TO ABORT",
                sw / 2.0,
                sh * 0.8,
                ui,
                Color::new(0.5, 0.5, 0.55, 1.0),
            );
            pixfont::draw_centered(
                assets,
                &format!("P: SHIELD-BROTHERS (AI) {}", self.partners),
                sw / 2.0,
                sh * 0.8 + 10.0 * ui,
                ui,
                Color::new(0.21, 0.78, 0.88, 1.0),
            );
            for (i, line) in super::help::BOARDING.iter().enumerate() {
                pixfont::draw_centered(
                    assets,
                    line,
                    sw / 2.0,
                    sh * 0.86 + i as f32 * 5.0 * ui,
                    (ui * 0.5).round().max(1.0),
                    Color::new(0.5, 0.5, 0.55, 1.0),
                );
            }
            return;
        }
        let (iw, ih) = r.ensure_target();
        let Some(w) = self.world.as_ref() else { return };
        let eye = w.player.eye();
        let look = w.player.look();
        let cam = r.camera(eye, eye + look, Vec3::Y, FOV);
        set_camera(&cam);
        clear_background(Color::new(0.02, 0.02, 0.04, 1.0));
        for (tex, mesh) in &self.meshes {
            let _ = tex;
            draw_mesh(mesh);
        }
        let right = look.cross(Vec3::Y).normalize_or_zero();
        let up = right.cross(look).normalize_or_zero();
        let lit = Shade::new(Vec3::Y, true, true);
        // doors
        for d in &w.level.doors {
            let c = w.level.centre(d.x, d.y);
            let tex = match d.lock {
                Some(Key::Bronze) => "door_lock_bronze",
                Some(Key::Silver) => "door_lock_silver",
                Some(Key::Gold) => "door_lock_gold",
                None => "door_minion",
            };
            let (a, b) = if d.along_x {
                (Vec3::new(0.0, 0.0, CELL / 2.0), Vec3::ZERO)
            } else {
                (Vec3::new(CELL / 2.0, 0.0, 0.0), Vec3::ZERO)
            };
            let (y0, y1) = (d.open * WALL_H, WALL_H);
            if y1 - y0 > 0.02 {
                let q = [
                    c - a + Vec3::Y * y0 + b,
                    c + a + Vec3::Y * y0 + b,
                    c + a + Vec3::Y * y1 + b,
                    c - a + Vec3::Y * y1 + b,
                ];
                let col = [235, 235, 235, 255];
                let v = |p: Vec3, u: f32, vv: f32| Vertex {
                    position: p,
                    uv: vec2(u, vv),
                    color: col,
                    normal: Vec4::ZERO,
                };
                draw_mesh(&Mesh {
                    vertices: vec![
                        v(q[0], 0.0, 1.0 - d.open),
                        v(q[1], 1.0, 1.0 - d.open),
                        v(q[2], 1.0, 0.0),
                        v(q[3], 0.0, 0.0),
                    ],
                    indices: vec![0, 1, 2, 0, 2, 3],
                    texture: Some(assets.tex(tex).clone()),
                });
            }
        }
        // objective props
        let g = w.level.centre(w.level.goal.0, w.level.goal.1);
        match w.level.objective {
            Objective::Sabotage => {
                if let Some(m) = assets.model("reactor") {
                    draw_model(
                        assets,
                        m,
                        "reactor_core",
                        Pose::new(g, Quat::IDENTITY, 1.0),
                        lit,
                    );
                }
            }
            Objective::CaptureBridge | Objective::StealData => {
                if let Some(m) = assets.model("console") {
                    draw_model(
                        assets,
                        m,
                        "console_a",
                        Pose::new(g, Quat::IDENTITY, 1.2),
                        lit,
                    );
                }
            }
            _ => {}
        }
        for p in &w.prisoners {
            if let Some(m) = assets.model("marine") {
                draw_model(
                    assets,
                    m,
                    "marine_armor",
                    Pose::new(
                        p.pos,
                        Quat::from_rotation_y(if p.freed { self.t } else { 0.5 }),
                        1.0,
                    ),
                    lit,
                );
            }
        }
        for m in w.mates.iter().filter(|m| m.health > 0.0) {
            if let Some(model) = assets.model("marine") {
                draw_model(
                    assets,
                    model,
                    "marine_armor",
                    Pose::new(m.pos, Quat::from_rotation_y(m.yaw), 1.0),
                    lit,
                );
            }
        }
        // monsters
        for m in &w.monsters {
            let st = stats(m.kind);
            let Some(model) = assets.model(st.model) else {
                continue;
            };
            let (rot, pos) = if m.state == State::Dead {
                let f = (m.dead_for * 3.0).min(1.0);
                (
                    Quat::from_rotation_y(m.yaw)
                        * Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2 * f),
                    m.pos + Vec3::Y * 0.25 * f,
                )
            } else {
                (Quat::from_rotation_y(m.yaw), m.pos)
            };
            draw_model(assets, model, "", Pose::new(pos, rot, 1.0), lit);
        }
        // pickups (billboards, bobbing)
        for it in w.items.iter().filter(|i| !i.taken) {
            let tex = item_texture(it.kind);
            let bob = 0.55 + 0.1 * (self.t * 3.0 + it.pos.x).sin();
            draw_billboard(
                assets.tex(tex),
                it.pos + Vec3::Y * bob,
                0.9,
                WHITE,
                right,
                up,
            );
        }
        for rk in &w.rockets {
            draw_billboard(assets.tex("rocket"), rk.pos, 0.7, WHITE, right, up);
        }
        r.additive(|| {
            for rk in &w.rockets {
                draw_billboard(
                    assets.tex("glow_soft"),
                    rk.pos,
                    2.4,
                    Color::new(1.0, 0.6, 0.25, 0.8),
                    right,
                    up,
                );
            }
            let sheet = assets.tex("explosion_sheet");
            for s in &self.sparks {
                let t = s.age / s.life;
                if s.blast {
                    let frame = ((t * 15.0) as u32).min(15);
                    let (cx, cy) = ((frame % 4) as f32 * 0.25, (frame / 4) as f32 * 0.25);
                    draw_billboard_uv(
                        sheet,
                        s.pos,
                        s.size,
                        Color::new(1.0, 1.0, 1.0, 1.0 - t * t),
                        right,
                        up,
                        (cx, cy, cx + 0.25, cy + 0.25),
                    );
                } else {
                    draw_billboard(
                        assets.tex("glow_soft"),
                        s.pos,
                        s.size,
                        Color::new(1.0, 0.85, 0.5, 1.0 - t),
                        right,
                        up,
                    );
                }
            }
        });
        set_camera(&r.camera_2d());
        let _ = (iw, ih);
        r.present();
        self.draw_hud(assets, w, ui);
    }

    fn draw_hud(&self, assets: &Assets, w: &BoardWorld, ui: f32) {
        let (sw, sh) = (screen_width(), screen_height());
        let p = &w.player;
        let c = vec2(sw / 2.0, sh / 2.0);
        // viewmodel
        let tex = assets.tex(weapon_texture(p.current));
        let size = sh * 0.5;
        let bob = (self.t * 8.0).sin() * (p.vel.length() / 6.5) * 6.0;
        let kick = (1.0 - (p.kick * 6.0).min(1.0)) * 40.0;
        draw_texture_ex(
            tex,
            c.x - size * 0.2 + bob,
            sh - size * 0.92 + kick + bob.abs(),
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(size, size)),
                ..Default::default()
            },
        );
        if self.flash > 0.0 && p.current != Weapon::Axe {
            draw_texture_ex(
                assets.tex("muzzle_flash"),
                c.x - size * 0.02,
                sh - size * 0.95,
                Color::new(1.0, 1.0, 1.0, self.flash),
                DrawTextureParams {
                    dest_size: Some(vec2(size * 0.4, size * 0.4)),
                    ..Default::default()
                },
            );
        }
        // crosshair
        let cc = Color::new(0.5, 1.0, 0.8, 0.9);
        draw_line(c.x - 8.0, c.y, c.x - 3.0, c.y, 2.0, cc);
        draw_line(c.x + 3.0, c.y, c.x + 8.0, c.y, 2.0, cc);
        draw_line(c.x, c.y - 8.0, c.x, c.y - 3.0, 2.0, cc);
        draw_line(c.x, c.y + 3.0, c.x, c.y + 8.0, 2.0, cc);
        if self.hurt > 0.0 {
            draw_rectangle(
                0.0,
                0.0,
                sw,
                sh,
                Color::new(1.0, 0.05, 0.0, self.hurt * 0.45),
            );
        }
        if w.progress.alarm_raised {
            let a = 0.08 + 0.07 * (self.t * 6.0).sin().abs();
            draw_rectangle(0.0, 0.0, sw, sh, Color::new(1.0, 0.0, 0.0, a));
        }
        // status
        let hp = p.health.max(0.0);
        let hcol = if hp > 50.0 {
            Color::new(0.4, 0.9, 0.5, 1.0)
        } else if hp > 25.0 {
            Color::new(0.95, 0.75, 0.2, 1.0)
        } else {
            Color::new(1.0, 0.3, 0.25, 1.0)
        };
        pixfont::draw(
            assets,
            &format!("HEALTH {hp:.0}"),
            24.0,
            sh - 40.0 * ui,
            ui * 1.5,
            hcol,
        );
        pixfont::draw(
            assets,
            &format!("ARMOR {}", p.armor),
            24.0,
            sh - 22.0 * ui,
            ui * 1.5,
            Color::new(0.45, 0.8, 1.0, 1.0),
        );
        let def = p.current.def();
        let ammo = def
            .ammo
            .map_or("INF".to_owned(), |a| p.ammo[a.idx()].to_string());
        pixfont::draw_right(
            assets,
            &format!("{}  {}", def.name, ammo),
            sw - 24.0,
            sh - 40.0 * ui,
            ui * 1.5,
            Color::new(1.0, 0.54, 0.17, 1.0),
        );
        let keys: Vec<&str> = Key::ALL
            .iter()
            .filter(|k| p.keys[**k as usize])
            .map(|k| k.name())
            .collect();
        pixfont::draw_right(
            assets,
            &format!(
                "KEYS: {}",
                if keys.is_empty() {
                    "NONE".to_owned()
                } else {
                    keys.join(" ")
                }
            ),
            sw - 24.0,
            sh - 22.0 * ui,
            ui * 1.2,
            Color::new(0.85, 0.8, 0.7, 1.0),
        );
        // objective + compass
        pixfont::draw_centered(
            assets,
            &w.objective_text(),
            sw / 2.0,
            14.0,
            ui,
            Color::new(1.0, 0.85, 0.2, 1.0),
        );
        let heading = (p.look().x.atan2(-p.look().z))
            .to_degrees()
            .rem_euclid(360.0);
        pixfont::draw_centered(
            assets,
            &format!("HDG {heading:03.0}"),
            sw / 2.0,
            14.0 + 12.0 * ui,
            ui,
            Color::new(0.21, 0.78, 0.88, 1.0),
        );
        if let Some(t) = w.progress.charges {
            pixfont::draw_centered(
                assets,
                &format!("DETONATION IN {t:.0}"),
                sw / 2.0,
                14.0 + 24.0 * ui,
                ui * 1.5,
                Color::new(1.0, 0.3, 0.2, 1.0),
            );
        }
        for (i, (m, ttl)) in self.messages.iter().rev().take(5).enumerate() {
            pixfont::draw_centered(
                assets,
                m,
                sw / 2.0,
                sh * 0.62 + i as f32 * 12.0 * ui,
                ui,
                Color::new(1.0, 1.0, 1.0, ttl.min(1.0)),
            );
        }
        let (tx, ty) = w.level.tile_of(p.pos);
        if w.level.rooms[0].contains(tx, ty) {
            pixfont::draw_centered(
                assets,
                "AIRLOCK - PRESS E TO RETURN TO YOUR SHIP",
                sw / 2.0,
                sh * 0.72,
                ui,
                Color::new(0.5, 1.0, 0.8, 1.0),
            );
        }
    }
}

fn weapon_texture(w: Weapon) -> &'static str {
    match w {
        Weapon::Axe => "weapon_axe",
        Weapon::Blaster => "weapon_blaster",
        Weapon::Scatter => "weapon_shotgun",
        Weapon::Rattler => "weapon_rattler",
        Weapon::Mjolnir => "weapon_mjolnir",
        Weapon::Valkyrie => "weapon_valkyrie",
    }
}

fn item_texture(k: ItemKind) -> &'static str {
    match k {
        ItemKind::HealthSmall => "pickup_health_s",
        ItemKind::HealthLarge => "pickup_health_l",
        ItemKind::ArmorShard => "pickup_armor_shard",
        ItemKind::ArmorJacket => "pickup_armor_jacket",
        ItemKind::ArmorBody => "pickup_armor_plate",
        ItemKind::Shells => "pickup_ammo_shells",
        ItemKind::Bullets => "pickup_ammo_bullets",
        ItemKind::Rockets => "pickup_ammo_rockets",
        ItemKind::Slugs => "pickup_ammo_slugs",
        ItemKind::Weapon(_) => "pickup_ammo_cells",
        ItemKind::Key(Key::Bronze) => "pickup_key_bronze",
        ItemKind::Key(Key::Silver) => "pickup_key_silver",
        ItemKind::Key(Key::Gold) => "pickup_key_gold",
        ItemKind::DataChip => "pickup_chip",
    }
}

/// Baked light at a point: dim ambient plus a falloff from every room's lamp.
fn light_at(level: &Level, p: Vec3) -> [u8; 4] {
    let mut c = [0.2f32, 0.2, 0.24];
    for r in &level.rooms {
        let (cx, cy) = r.center();
        let lp = level.centre(cx, cy) + Vec3::Y * 2.8;
        let reach = 7.0 + (r.w.max(r.h) as f32) * CELL * 0.4;
        let f = (1.0 - p.distance(lp) / reach).clamp(0.0, 1.0);
        let f = f * f;
        let tint = match r.kind {
            RoomKind::Reactor => [1.0, 0.55, 0.3],
            RoomKind::Bridge => [0.6, 0.85, 1.0],
            RoomKind::Cells => [0.7, 0.9, 0.7],
            RoomKind::Airlock => [0.8, 1.0, 0.9],
            _ => [1.0, 0.95, 0.85],
        };
        for i in 0..3 {
            c[i] += tint[i] * f * 0.85;
        }
    }
    for l in &level.lamps {
        let lp = level.centre(l.x, l.y)
            + Vec3::Y * (level.ceil_h(l.x, l.y) - level.floor_h(l.x, l.y) - 0.4);
        let f = (1.0 - p.distance(lp) / l.reach).clamp(0.0, 1.0);
        let f = f * f;
        for (ci, li) in c.iter_mut().zip(l.color) {
            *ci += li * f * 0.9;
        }
    }
    [
        (c[0].min(1.0) * 255.0) as u8,
        (c[1].min(1.0) * 255.0) as u8,
        (c[2].min(1.0) * 255.0) as u8,
        255,
    ]
}

struct Batch {
    tex: &'static str,
    verts: Vec<Vertex>,
    idx: Vec<u16>,
}

fn add_quad(batches: &mut Vec<Batch>, tex: &'static str, level: &Level, q: [Vec3; 4]) {
    add_quad_col(batches, tex, q, |p| light_at(level, p));
}

fn add_quad_col(
    batches: &mut Vec<Batch>,
    tex: &'static str,
    q: [Vec3; 4],
    color: impl Fn(Vec3) -> [u8; 4],
) {
    let b = match batches
        .iter()
        .position(|b| b.tex == tex && b.verts.len() + 4 < MAX_VERTS)
    {
        Some(i) => i,
        None => {
            batches.push(Batch {
                tex,
                verts: Vec::new(),
                idx: Vec::new(),
            });
            batches.len() - 1
        }
    };
    let b = &mut batches[b];
    let base = b.verts.len() as u16;
    let uvs = [
        vec2(0.0, 1.0),
        vec2(1.0, 1.0),
        vec2(1.0, 0.0),
        vec2(0.0, 0.0),
    ];
    for (p, uv) in q.iter().zip(uvs) {
        b.verts.push(Vertex {
            position: *p,
            uv,
            color: color(*p),
            normal: Vec4::ZERO,
        });
    }
    b.idx
        .extend([base, base + 1, base + 2, base, base + 2, base + 3]);
}

/// Bake the static level geometry: floor, ceiling and wall quads per texture.
pub fn build_meshes(level: &Level, assets: &Assets) -> Vec<(&'static str, Mesh)> {
    let mut batches: Vec<Batch> = Vec::new();
    let walls = level.theme.walls();
    for (x, y) in level.floor_tiles() {
        let (x0, z0) = (x as f32 * CELL, y as f32 * CELL);
        let (x1, z1) = (x0 + CELL, z0 + CELL);
        let (fy, cy) = (level.floor_h(x, y), level.ceil_h(x, y));
        let kind = level.room_at(x, y).map(|r| r.kind);
        let floor = match kind {
            Some(RoomKind::Reactor) => "floor_hazard",
            Some(RoomKind::Bridge | RoomKind::Airlock) => "floor_tile",
            None => "floor_grate",
            Some(_) => {
                if (x + y) % 5 == 0 {
                    "floor_grate"
                } else {
                    "floor_tile"
                }
            }
        };
        add_quad(
            &mut batches,
            floor,
            level,
            [
                Vec3::new(x0, fy, z1),
                Vec3::new(x1, fy, z1),
                Vec3::new(x1, fy, z0),
                Vec3::new(x0, fy, z0),
            ],
        );
        let ceil = if (x + y) % 2 == 0 {
            "ceiling_a"
        } else {
            "ceiling_b"
        };
        add_quad(
            &mut batches,
            ceil,
            level,
            [
                Vec3::new(x0, cy, z0),
                Vec3::new(x1, cy, z0),
                Vec3::new(x1, cy, z1),
                Vec3::new(x0, cy, z1),
            ],
        );
        let wall = |i: i32| walls[(i.rem_euclid(2)) as usize];
        let tex = match kind {
            Some(RoomKind::Reactor) => "wall_reactor",
            Some(RoomKind::Cells) => "wall_viking_b",
            Some(RoomKind::Airlock) => "wall_viking_a",
            _ => wall(x / 3 + y / 3),
        };
        // faces on the tile's four edges: walls, stair risers (neighbour floor higher) and
        // ceiling drops (neighbour ceiling lower)
        for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
            let (nx, ny) = (x + dx, y + dy);
            let spans: Vec<(f32, f32, &'static str)> =
                if level.tile(nx, ny) == crate::board::level::Tile::Wall {
                    vec![(fy, cy, tex)]
                } else {
                    let mut v = Vec::new();
                    let nf = level.floor_h(nx, ny);
                    if nf > fy + 0.001 {
                        v.push((fy, nf, "floor_hazard"));
                    }
                    let nc = level.ceil_h(nx, ny);
                    if nc < cy - 0.001 {
                        v.push((nc, cy, tex));
                    }
                    v
                };
            for (lo, hi, t) in spans {
                let q = match (dx, dy) {
                    (-1, 0) => [
                        Vec3::new(x0, lo, z0),
                        Vec3::new(x0, lo, z1),
                        Vec3::new(x0, hi, z1),
                        Vec3::new(x0, hi, z0),
                    ],
                    (1, 0) => [
                        Vec3::new(x1, lo, z1),
                        Vec3::new(x1, lo, z0),
                        Vec3::new(x1, hi, z0),
                        Vec3::new(x1, hi, z1),
                    ],
                    (0, -1) => [
                        Vec3::new(x1, lo, z0),
                        Vec3::new(x0, lo, z0),
                        Vec3::new(x0, hi, z0),
                        Vec3::new(x1, hi, z0),
                    ],
                    _ => [
                        Vec3::new(x0, lo, z1),
                        Vec3::new(x1, lo, z1),
                        Vec3::new(x1, hi, z1),
                        Vec3::new(x0, hi, z1),
                    ],
                };
                add_quad(&mut batches, t, level, q);
            }
        }
    }
    // ceiling light fixtures, full bright
    for l in &level.lamps {
        let c = level.centre(l.x, l.y);
        let y = level.ceil_h(l.x, l.y) - 0.02;
        let (x0, x1, z0, z1) = (c.x - 0.9, c.x + 0.9, c.z - 0.9, c.z + 0.9);
        let col = [
            (l.color[0] * 255.0) as u8,
            (l.color[1] * 255.0) as u8,
            (l.color[2] * 255.0) as u8,
            255,
        ];
        add_quad_col(
            &mut batches,
            "light_panel",
            [
                Vec3::new(x0, y, z0),
                Vec3::new(x1, y, z0),
                Vec3::new(x1, y, z1),
                Vec3::new(x0, y, z1),
            ],
            |_| col,
        );
    }
    batches
        .into_iter()
        .map(|b| {
            (
                b.tex,
                Mesh {
                    vertices: b.verts,
                    indices: b.idx,
                    texture: Some(assets.tex(b.tex).clone()),
                },
            )
        })
        .collect()
}
