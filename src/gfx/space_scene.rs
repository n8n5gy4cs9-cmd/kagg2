//! The space-flight scene: owns the sim `World`, turns keyboard input into `Command`s, steps the
//! sim at a fixed rate and renders it (sky, sun, planets, ships, effects) plus the HUD.

use macroquad::models::{Mesh, Vertex, draw_mesh};
use macroquad::prelude::*;

use super::board_scene::{BoardInfo, BoardResult};
use super::cockpit;
use super::dock_ui::{self, DockState};
use super::help;
use super::hud::{self, HudData};
use super::keys::{Bind, Bindings};
use super::pixfont;
use super::render3d::{
    Pose, Renderer, Shade, draw_billboard, draw_billboard_uv, draw_model, draw_streak,
};
use super::sound::Mixer;
use super::stations::{self, Frame, Tab, UiState};
use super::ui::{Action, Ctx};
use crate::assets::Assets;
use crate::audio::Sfx;
use crate::sim::comms::{self, Reply};
use crate::sim::event::{Event, Ordnance};
use crate::sim::flight::{FORWARD, RIGHT, UP};
use crate::sim::mission::{Carry, MissionState, Note};
use crate::sim::ops::{Dest, Faction};
use crate::sim::sector::{SectorId, SkyKind};
use crate::sim::{Addressed, Command, EntityId, Kind, Stepper, World, new_game, transfer};

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

/// A lance beam on screen.
struct Beam {
    from: Vec3,
    to: Vec3,
    age: f32,
    color: Color,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum FxKind {
    Explosion,
    Shield,
}

/// A short-lived visual effect.
struct Fx {
    kind: FxKind,
    pos: Vec3,
    size: f32,
    age: f32,
    life: f32,
}

const BEAM_LIFE: f32 = 0.2;

fn pressed_help() -> bool {
    is_key_pressed(KeyCode::F8)
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
    ui: UiState,
    dock: DockState,
    actions: Vec<Action>,
    queue: Vec<Addressed>,
    beams: Vec<Beam>,
    fx: Vec<Fx>,
    /// Red damage flash `0..1`.
    hurt: f32,
    /// Developer: hold the lance trigger.
    autofire: bool,
    /// Seconds since the player's ship died.
    dead_for: f32,
    alarm_cd: f32,
    hint_for: Option<EntityId>,
    /// Active campaign mission, if any (free flight has none).
    pub mission: Option<MissionState>,
    /// Dialogue captions with seconds left.
    captions: Vec<(Note, f32)>,
    /// Seconds since the mission was decided.
    ended_for: f32,
    pub keys: Bindings,
    pub invert_y: bool,
    help_station: bool,
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
            ui: UiState::default(),
            dock: DockState::default(),
            actions: Vec::new(),
            queue: Vec::new(),
            beams: Vec::new(),
            fx: Vec::new(),
            hurt: 0.0,
            autofire: false,
            dead_for: 0.0,
            alarm_cd: 0.0,
            hint_for: None,
            mission: None,
            captions: Vec::new(),
            ended_for: 0.0,
            keys: Bindings::default(),
            invert_y: false,
            help_station: false,
        }
    }

    /// Start campaign mission `id`: carry the ship over, scale enemies, spawn the first stage.
    pub fn start_mission(&mut self, id: u8, carry: &Carry, difficulty: f32) {
        self.world.difficulty = difficulty;
        carry.apply(&mut self.world);
        let mut m = MissionState::new(id);
        m.begin(&mut self.world);
        self.mission = Some(m);
    }

    /// Why the mission was lost, for the debrief.
    pub fn loss_reason(&self) -> &'static str {
        self.mission.as_ref().map_or("", MissionState::reason)
    }

    /// `Some(won)` once the mission is decided (after a short beat), or when the ship has been
    /// dead long enough.
    pub fn outcome(&self) -> Option<bool> {
        if self.dead_for > 4.0 {
            return Some(false);
        }
        self.mission
            .as_ref()
            .and_then(MissionState::result)
            .filter(|_| self.ended_for > 3.0)
    }

    /// Boarding is possible against a hostile ship that is disabled — or crippled (hull < 60 %
    /// with its shields down) — and close enough to clamp on to.
    pub fn boardable(&self) -> Option<BoardInfo> {
        use crate::board::Theme;
        let me = self.world.player.and_then(|p| self.world.get(p))?;
        let id = self.target?;
        let t = self.world.get(id)?;
        let o = t.ops.as_deref()?;
        if t.kind != Kind::Ship || !t.faction.hostile_to(me.faction) || o.dead {
            return None;
        }
        if !(o.disabled || o.crippled()) {
            return None;
        }
        if me.pos.distance(t.pos) - me.radius - t.radius > 450.0 {
            return None;
        }
        let seed = self.world.tick ^ ((id.index as u64) << 32);
        let (theme, size) = match (t.label, o.class_name) {
            (l, _) if l.starts_with("Prison Barge") => (Theme::PrisonBarge, 3),
            (l, _) if l.starts_with("Courier") => (Theme::Corvette, 2),
            (_, "Interceptor") => (Theme::Corvette, 1),
            (_, "Cutter") if seed.is_multiple_of(2) => (Theme::PrisonBarge, 2),
            (_, "Cutter") => (Theme::Corvette, 2),
            (_, "Frigate") => (Theme::Station, 3),
            (_, "Destroyer") => (Theme::Station, 4),
            _ => (Theme::DreadnoughtBridge, 5),
        };
        Some(BoardInfo {
            theme,
            size,
            label: t.label.to_owned(),
            target: id,
            seed,
            force: self.mission.as_ref().and_then(|m| m.board_wanted(t.label)),
        })
    }

    /// Fold the result of a boarding action back into the space world: loot, casualties, and the
    /// fate of the boarded ship (captured or sabotaged ships are removed).
    pub fn apply_boarding(&mut self, info: &BoardInfo, res: &BoardResult) {
        use crate::board::{Objective, Outcome};
        let won = res.outcome == Outcome::Won;
        if let Some(m) = self.mission.as_mut() {
            m.on_boarding(&info.label, res.objective, won && res.objective_done);
        }
        let casualties = (res.damage_taken / 25.0).round() as u32;
        let me = self.world.player;
        if let Some(o) = me
            .and_then(|p| self.world.get_mut(p))
            .and_then(|e| e.ops.as_deref_mut())
        {
            o.cargo.credits += res.credits;
            o.sys.crew = o.sys.crew.saturating_sub(casualties);
            if won {
                o.torpedoes = (o.torpedoes + 2).min(o.max_torpedoes());
                o.missiles = (o.missiles + 3).min(o.max_missiles());
            }
        }
        let line = match (won, res.objective) {
            (true, Objective::CaptureBridge) => format!(
                "{} CAPTURED - {} KILLS, +{} CR",
                info.label.to_uppercase(),
                res.kills,
                res.credits
            ),
            (true, _) => format!(
                "BOARDING SUCCESS - {} KILLS, +{} CR",
                res.kills, res.credits
            ),
            _ => "BOARDING PARTY RECALLED".to_owned(),
        };
        self.messages.push((line.clone(), 5.0));
        self.ui.say(line);
        if casualties > 0 {
            self.ui.say(format!("{casualties} CREW LOST"));
        }
        if won && let Some(t) = self.world.get(info.target).map(|e| e.pos) {
            self.world
                .push_event(Event::Explosion { pos: t, size: 60.0 });
            self.world.despawn(info.target);
            self.target = None;
        }
    }

    fn ops(&self) -> Option<&crate::sim::ops::Ops> {
        self.player_id()
            .and_then(|p| self.world.get(p))
            .and_then(|e| e.ops.as_deref())
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
                self.world.get(*id).is_some_and(|e| match e.kind {
                    Kind::Projectile | Kind::Debris => false,
                    Kind::Asteroid => e.radius > 100.0,
                    _ => true,
                })
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

    fn start_hail(&mut self) {
        let (Some(me), Some(t)) = (self.player_id(), self.target) else {
            self.ui.say("NO TARGET TO HAIL - PRESS T");
            return;
        };
        match comms::hail(&self.world, me, t) {
            Some(h) => {
                self.ui.say(h.greeting.clone());
                self.ui.dialog = Some((t, h));
            }
            None => self.ui.say("NO ANSWER"),
        }
    }

    fn do_reply(&mut self, i: usize, cmds: &mut Vec<Command>) {
        let Some((t, h)) = self.ui.dialog.clone() else {
            return;
        };
        let Some(&(label, reply)) = h.options.get(i) else {
            return;
        };
        self.ui.say(format!("YOU: {label}"));
        let seed = self
            .world
            .tick
            .wrapping_mul(0x9E37_79B9_7F4A_7C15)
            .rotate_left(17)
            ^ self.world.time.to_bits() as u64;
        match reply {
            Reply::RequestDock => {
                self.ui.say(format!("{}: Clearing you in, Huginn.", h.from));
                cmds.push(Command::Dock(Some(t)));
            }
            Reply::DemandSurrender => cmds.push(Command::DemandSurrender(t)),
            Reply::Order(o) => cmds.push(Command::Order(o)),
            Reply::AskNews | Reply::Taunt => {
                if let Some(line) = comms::answer(&self.world, t, reply, seed) {
                    self.ui.say(line);
                }
            }
            Reply::Goodbye => self.ui.dialog = None,
        }
    }

    fn sfx(&self, snd: Option<&Mixer>, s: Sfx, at: Vec3, near: f32) {
        let (Some(snd), Some(me)) = (snd, self.player_id().and_then(|p| self.world.get(p))) else {
            return;
        };
        let d = me.pos.distance(at) / near;
        snd.play(s, 1.0 / (1.0 + d * d));
    }

    /// React to sim events: effects, messages and sounds.
    fn on_event(&mut self, ev: Event, me: EntityId, snd: Option<&Mixer>) -> bool {
        let my_pos = self.world.get(me).map_or(Vec3::ZERO, |e| e.pos);
        match ev {
            Event::Collision { other, speed, .. } if speed > 25.0 => {
                let name = self.world.get(other).map_or("OBJECT", |o| o.label);
                self.messages.push((
                    format!("COLLISION - {} ({:.0} U/S)", name.to_uppercase(), speed),
                    3.0,
                ));
                self.hurt = self.hurt.max(0.3);
                self.sfx(snd, Sfx::HullCreak, my_pos, 1e9);
            }
            Event::LanceFired {
                shooter, from, to, ..
            } => {
                let mine = self
                    .world
                    .get(shooter)
                    .is_some_and(|e| e.faction == Faction::Mars);
                let color = if mine {
                    Color::new(0.35, 0.85, 1.0, 1.0)
                } else {
                    Color::new(1.0, 0.35, 0.2, 1.0)
                };
                self.beams.push(Beam {
                    from,
                    to,
                    age: 0.0,
                    color,
                });
                self.sfx(snd, Sfx::Lance, from, 1500.0);
            }
            Event::Launched { shooter, kind } => {
                let at = self.world.get(shooter).map_or(my_pos, |e| e.pos);
                let s = if kind == Ordnance::Torpedo {
                    Sfx::Torpedo
                } else {
                    Sfx::Missile
                };
                self.sfx(snd, s, at, 1500.0);
            }
            Event::Explosion { pos, size } => {
                self.fx.push(Fx {
                    kind: FxKind::Explosion,
                    pos,
                    size,
                    age: 0.0,
                    life: (0.45 + size / 120.0).min(1.6),
                });
                if size > 20.0 {
                    self.sfx(snd, Sfx::Explosion, pos, 2500.0);
                }
            }
            Event::ShieldHit { ship, dir, .. } => {
                if let Some(e) = self.world.get(ship) {
                    self.fx.push(Fx {
                        kind: FxKind::Shield,
                        pos: e.pos + dir * e.radius * 0.8,
                        size: e.radius * 2.2,
                        age: 0.0,
                        life: 0.35,
                    });
                    let at = e.pos;
                    self.sfx(snd, Sfx::ShieldHit, at, 1500.0);
                }
                if ship == me {
                    self.hurt = self.hurt.max(0.2);
                }
            }
            Event::HullHit { ship, .. } if ship == me => {
                self.hurt = self.hurt.max(0.55);
                self.sfx(snd, Sfx::HullCreak, my_pos, 1e9);
            }
            Event::SystemDamaged { ship, sys } if ship == me => {
                self.messages.push((format!("{} DAMAGED", sys.name()), 3.0));
            }
            Event::Destroyed { id, label, by, .. } => {
                let line = if id == me {
                    "YOUR SHIP HAS BEEN DESTROYED".to_owned()
                } else {
                    format!("{} DESTROYED", label.to_uppercase())
                };
                self.messages.push((line.clone(), 4.0));
                self.ui.say(line);
                let _ = by;
            }
            Event::Surrendered(id) => {
                let label = self.world.get(id).map_or("", |e| e.label);
                self.ui.say(format!("{label} SURRENDERS"));
            }
            Event::Docked(_) => {
                self.dock = DockState::default();
                self.sfx(snd, Sfx::DockClamp, my_pos, 1e9);
            }
            Event::Undocked => self.sfx(snd, Sfx::DockClamp, my_pos, 1e9),
            Event::Message(m) => {
                self.messages.push((m.clone(), 4.0));
                self.ui.say(m);
            }
            Event::WarpJump(to) => {
                let old = std::mem::replace(&mut self.world, World::new(0));
                self.world = transfer(old, to);
                self.target = None;
                self.ui.dialog = None;
                self.beams.clear();
                self.fx.clear();
                if let Some(e) = self.world.player.and_then(|p| self.world.get(p)) {
                    self.cam_rot = e.rot;
                }
                self.sfx(snd, Sfx::Warp, Vec3::ZERO, 1e9);
                self.messages
                    .push((format!("ARRIVED AT {}", to.name().to_uppercase()), 4.0));
                return true;
            }
            _ => {}
        }
        false
    }

    /// Read input, run the fixed-step sim, update camera and message timers.
    pub fn update(&mut self, dt: f32, snd: Option<&Mixer>) {
        self.t += dt;
        self.help_left = (self.help_left - dt).max(0.0);
        self.hurt = (self.hurt - dt * 1.4).max(0.0);
        self.alarm_cd = (self.alarm_cd - dt).max(0.0);
        for b in &mut self.beams {
            b.age += dt;
        }
        self.beams.retain(|b| b.age < BEAM_LIFE);
        for f in &mut self.fx {
            f.age += dt;
        }
        self.fx.retain(|f| f.age < f.life);
        let Some(me) = self.player_id() else { return };
        let dead = self.ops().is_none_or(|o| o.dead);
        if dead {
            self.dead_for += dt;
            if let Some(m) = self.mission.as_mut() {
                m.fail("YOUR SHIP WAS DESTROYED");
            }
        }
        if self.mission.as_ref().is_some_and(|m| m.result().is_some()) {
            self.ended_for += dt;
        }
        for c in &mut self.captions {
            c.1 -= dt;
        }
        self.captions.retain(|c| c.1 > 0.0);
        let docked = self.ops().and_then(|o| o.docked).map(|d| d.0);
        let keys = self.keys.clone();
        let kb = |b: Bind| keys.get(b);
        let k = is_key_down;
        if pressed_help() {
            self.help_station = !self.help_station;
        }
        let pressed = is_key_pressed;
        let axis = |neg: KeyCode, pos: KeyCode| (k(pos) as i32 - k(neg) as i32) as f32;
        let mut cmds: Vec<Command> = Vec::new();

        // --- tabs and crew AUTO
        for tab in Tab::ALL {
            if pressed(tab.key()) && self.ui.tab != tab {
                self.ui.tab = tab;
                if let Some(s) = snd {
                    s.play(Sfx::Beep, 0.6);
                }
            }
        }
        if pressed(KeyCode::Tab) {
            self.world.crew.toggle(self.ui.tab.station());
        }
        let tab = self.ui.tab;

        // --- panel clicks from the last frame
        for a in std::mem::take(&mut self.actions) {
            match a {
                Action::Cmd(c) => cmds.push(c),
                Action::Hail => self.start_hail(),
                Action::Reply(i) => self.do_reply(i, &mut cmds),
                Action::CloseDialog => self.ui.dialog = None,
                Action::SelectTarget(id) => {
                    self.target = Some(id);
                    cmds.push(Command::SetTarget(Some(id)));
                }
            }
        }

        if docked.is_some() {
            if pressed(kb(Bind::Dock)) {
                cmds.push(Command::Dock(None));
            }
        } else if !dead {
            // --- flight
            let thr = axis(kb(Bind::ThrottleDown), kb(Bind::ThrottleUp));
            if thr != 0.0 {
                cmds.push(Command::AdjustThrottle(thr * 0.55 * dt));
            }
            if pressed(kb(Bind::ZeroThrottle)) {
                cmds.push(Command::SetThrottle(0.0));
            }
            if pressed(kb(Bind::FullThrottle)) {
                cmds.push(Command::SetThrottle(1.0));
            }
            let turn = Command::Turn {
                // default: down = pull back = nose up
                pitch: axis(kb(Bind::PitchUp), kb(Bind::PitchDown))
                    * if self.invert_y { -1.0 } else { 1.0 },
                yaw: axis(kb(Bind::YawRight), kb(Bind::YawLeft)), // left = positive yaw
                roll: axis(kb(Bind::RollLeft), kb(Bind::RollRight)),
            };
            let strafe = Command::Strafe {
                x: axis(kb(Bind::StrafeLeft), kb(Bind::StrafeRight)),
                y: axis(kb(Bind::StrafeDown), kb(Bind::StrafeUp)),
            };
            let manual = thr != 0.0
                || [
                    Bind::Brake,
                    Bind::YawLeft,
                    Bind::YawRight,
                    Bind::RollLeft,
                    Bind::RollRight,
                    Bind::PitchUp,
                    Bind::PitchDown,
                    Bind::StrafeLeft,
                    Bind::StrafeRight,
                    Bind::StrafeUp,
                    Bind::StrafeDown,
                ]
                .iter()
                .any(|b| k(kb(*b)));
            let autopilot = self.ops().is_some_and(|o| o.nav.autopilot);
            if manual && autopilot {
                cmds.push(Command::Autopilot(false));
            }
            cmds.push(turn);
            cmds.push(strafe);
            cmds.push(Command::Brake(k(kb(Bind::Brake))));
            if pressed(kb(Bind::Assist)) {
                let assist = self
                    .world
                    .get(me)
                    .and_then(|e| e.ship)
                    .is_some_and(|s| s.assist);
                cmds.push(Command::SetAssist(!assist));
            }
            // --- weapons
            let click_fire = tab == Tab::Helm && is_mouse_button_down(MouseButton::Left);
            if k(kb(Bind::Fire)) || click_fire || self.autofire {
                cmds.push(Command::FireLance);
            }
            if pressed(kb(Bind::Torpedo)) {
                cmds.push(Command::FireTorpedo);
            }
            if pressed(kb(Bind::Missile)) {
                cmds.push(Command::FireMissile);
            }
            let (lf, sf) = self.ops().map_or((0, 0), |o| (o.lance_freq, o.shield_freq));
            if pressed(KeyCode::LeftBracket) {
                cmds.push(Command::SetLanceFreq(lf.saturating_sub(1)));
            }
            if pressed(KeyCode::RightBracket) {
                cmds.push(Command::SetLanceFreq((lf + 1).min(9)));
            }
            if pressed(KeyCode::Comma) {
                cmds.push(Command::SetShieldFreq(sf.saturating_sub(1)));
            }
            if pressed(KeyCode::Period) {
                cmds.push(Command::SetShieldFreq((sf + 1).min(9)));
            }
            // --- nav / science / starbase
            if pressed(kb(Bind::Autopilot)) {
                cmds.push(Command::Autopilot(!autopilot));
            }
            if pressed(KeyCode::O)
                && let Some(t) = self.target
            {
                cmds.push(Command::SetDest(Dest::Body(t)));
                cmds.push(Command::Autopilot(true));
            }
            if pressed(kb(Bind::Scan))
                && let Some(t) = self.target
            {
                cmds.push(Command::Scan(Some(t)));
            }
            if pressed(KeyCode::U) {
                cmds.push(Command::Sweep);
            }
            if pressed(KeyCode::K) {
                let mining = self.ops().is_some_and(|o| o.mining.is_some());
                cmds.push(Command::Mine(if mining { None } else { self.target }));
            }
            if pressed(kb(Bind::Dock))
                && let Some(t) = self.target
            {
                cmds.push(Command::Dock(Some(t)));
            }
            if tab == Tab::Engineering {
                for (key, n) in [(KeyCode::Key1, 1), (KeyCode::Key2, 2), (KeyCode::Key3, 3)] {
                    if pressed(key) {
                        cmds.push(Command::PowerPreset(n));
                    }
                }
            }
            if tab == Tab::Comms {
                if pressed(KeyCode::H) {
                    self.start_hail();
                }
                let keys = [
                    KeyCode::Key1,
                    KeyCode::Key2,
                    KeyCode::Key3,
                    KeyCode::Key4,
                    KeyCode::Key5,
                    KeyCode::Key6,
                    KeyCode::Key7,
                    KeyCode::Key8,
                    KeyCode::Key9,
                ];
                for (i, key) in keys.iter().enumerate() {
                    if pressed(*key) {
                        self.do_reply(i, &mut cmds);
                    }
                }
            } else if pressed(KeyCode::H) {
                self.help_left = if self.help_left > 0.0 { 0.0 } else { 18.0 };
            }
            if pressed(kb(Bind::Camera)) {
                self.mode = if self.mode == CamMode::Cockpit {
                    CamMode::Chase
                } else {
                    CamMode::Cockpit
                };
            }
            if pressed(kb(Bind::NextTarget)) {
                self.cycle_target(1);
            }
            if pressed(kb(Bind::ClearTarget)) {
                self.target = None;
            }
        }
        if let Some(info) = self.boardable()
            && self.hint_for != Some(info.target)
        {
            self.hint_for = Some(info.target);
            self.messages.push((
                format!("{} CAN BE BOARDED - PRESS B", info.label.to_uppercase()),
                5.0,
            ));
        }
        // keep the sim's target (missile lock, crew AI) in step with the UI
        let crew_picks = self.world.crew.is_auto(crate::sim::crew::Station::Weapons);
        let sim_target = self.ops().and_then(|o| o.target);
        if crew_picks && sim_target.is_some() && self.target != sim_target {
            self.target = sim_target;
        } else if sim_target != self.target {
            cmds.push(Command::SetTarget(self.target));
        }
        if self.world.crew.is_auto(crate::sim::crew::Station::Science)
            && let Some(t) = self.ops().and_then(|o| o.scan.target)
            && self.target.is_none()
        {
            self.target = Some(t);
        }

        // --- low-hull alarm
        if let Some(o) = self.ops()
            && !o.dead
            && self.alarm_cd == 0.0
            && (o.hull < o.max_hull * 0.25 || o.sys.worst() <= 0.0)
        {
            self.alarm_cd = 2.5;
            if let Some(s) = snd {
                s.play(Sfx::Alarm, 0.7);
            }
        }

        // Discrete commands wait in the queue until a fixed step runs; held-state commands
        // (turn, strafe, brake, target) only keep their latest value.
        self.queue.retain(|a| {
            !matches!(
                a.cmd,
                Command::Turn { .. }
                    | Command::Strafe { .. }
                    | Command::Brake(_)
                    | Command::SetTarget(_)
            )
        });
        self.queue
            .extend(cmds.iter().map(|&cmd| Addressed { ship: me, cmd }));
        let steps = self.stepper.advance(dt);
        let mut first = true;
        'steps: for _ in 0..steps {
            let queued = std::mem::take(&mut self.queue);
            let events = self.world.step(if first { &queued } else { &[] });
            first = false;
            if let Some(m) = self.mission.as_mut() {
                m.update(&mut self.world, &events);
            }
            for e in events {
                if self.on_event(e, me, snd) {
                    break 'steps;
                }
            }
        }
        if steps == 0 {
            // keep flight demands fresh even when no sim step ran this frame
            for a in &self.queue {
                if matches!(
                    a.cmd,
                    Command::Turn { .. } | Command::Strafe { .. } | Command::Brake(_)
                ) && let Some(s) = self.world.get_mut(a.ship).and_then(|e| e.ship.as_mut())
                {
                    s.apply(a.cmd);
                }
            }
        }
        if let Some(m) = self.mission.as_mut() {
            for n in m.take_notes() {
                if n.who.is_empty() {
                    self.messages.push((n.text.to_owned(), 3.0));
                } else {
                    self.ui.say(format!("{}: {}", n.who, n.text));
                    self.captions.push((n, 7.0));
                }
            }
            if self.captions.len() > 3 {
                self.captions.remove(0);
            }
        }
        for m in &mut self.messages {
            m.1 -= dt;
        }
        self.messages.retain(|m| m.1 > 0.0);
        if self.messages.len() > 8 {
            self.messages.remove(0);
        }
        if self.target.is_some_and(|t| self.world.get(t).is_none()) {
            self.target = None;
        }
        if let Some(e) = self.world.get(self.world.player.unwrap_or(me)) {
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
        let me_dead = self.ops().is_none_or(|o| o.dead);
        for (id, e) in self.world.iter() {
            if id == me_id && (self.mode == CamMode::Cockpit || me_dead) {
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
            // lance beams
            for b in &self.beams {
                let f = 1.0 - b.age / BEAM_LIFE;
                let c = b.color;
                // start no closer than 140 units to the eye so the near end does not flare
                let seg = b.to - b.from;
                let len = seg.length().max(1.0);
                let near = (140.0 - b.from.distance(cam_pos)).max(0.0);
                let a = b.from + seg / len * near.min(len * 0.9);
                draw_streak(a, b.to, 4.5, Color::new(c.r, c.g, c.b, 0.45 * f), cam_pos);
                draw_streak(a, b.to, 1.6, Color::new(1.0, 1.0, 1.0, 0.95 * f), cam_pos);
            }
            // ordnance exhaust
            for (_, e) in self.world.iter() {
                if e.kind == Kind::Projectile {
                    let pos = e.prev_pos.lerp(e.pos, alpha);
                    let dir = e.vel.normalize_or_zero();
                    draw_streak(
                        pos,
                        pos - dir * 45.0,
                        4.0,
                        Color::new(1.0, 0.6, 0.25, 0.8),
                        cam_pos,
                    );
                    draw_billboard(glow, pos, 16.0, Color::new(1.0, 0.7, 0.4, 0.9), right, up);
                }
            }
            // explosions and shield flashes
            let sheet = assets.tex("explosion_sheet");
            for f in &self.fx {
                let t = f.age / f.life;
                match f.kind {
                    FxKind::Explosion => {
                        let frame = ((t * 15.0) as u32).min(15);
                        let (cx, cy) = ((frame % 4) as f32 * 0.25, (frame / 4) as f32 * 0.25);
                        draw_billboard_uv(
                            sheet,
                            f.pos,
                            f.size * 2.2,
                            Color::new(1.0, 1.0, 1.0, 1.0 - t * t),
                            right,
                            up,
                            (cx, cy, cx + 0.25, cy + 0.25),
                        );
                        draw_billboard(
                            glow,
                            f.pos,
                            f.size * 3.0 * (1.0 - t),
                            Color::new(1.0, 0.7, 0.3, 0.35 * (1.0 - t)),
                            right,
                            up,
                        );
                    }
                    FxKind::Shield => draw_billboard(
                        glow,
                        f.pos,
                        f.size * (0.7 + 0.5 * t),
                        Color::new(0.35, 0.8, 1.0, 0.6 * (1.0 - t)),
                        right,
                        up,
                    ),
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
        let docked = self.ops().and_then(|o| o.docked).map(|d| d.0);
        if docked.is_none() {
            hud::draw_space(
                assets,
                &HudData {
                    world: &self.world,
                    player: me_id,
                    target: self.target,
                    view_proj: self.view_proj,
                    cockpit: self.mode == CamMode::Cockpit,
                    messages: &self.messages,
                    show_help: self.help_left > 0.0 && self.ui.tab == Tab::Helm,
                    fov_deg: FOV,
                    ui_scale,
                    tab: self.ui.tab.station().name(),
                },
            );
        }
        if self.hurt > 0.0 {
            draw_rectangle(
                0.0,
                0.0,
                screen_width(),
                screen_height(),
                Color::new(1.0, 0.08, 0.04, self.hurt * 0.35),
            );
        }
        self.draw_mission(assets, ui_scale);
        // --- station tabs, panels, docked screen
        let panel_fs = (ui_scale * 0.75).round().max(2.0);
        let c = Ctx::new(assets, panel_fs);
        let mut actions = Vec::new();
        if let Some(st) = docked {
            if let Some(cmd) = dock_ui::draw(&c, &mut self.dock, &mut self.world, me_id, st) {
                actions.push(Action::Cmd(cmd));
            }
        } else {
            let frame = Frame {
                world: &self.world,
                me: me_id,
                target: self.target,
            };
            let top = stations::tab_bar(&c, &mut self.ui, &frame);
            let tab = self.ui.tab;
            stations::draw_panel(&c, &mut self.ui, &frame, &mut actions, top);
            let _ = tab;
        }
        self.actions.extend(actions);
        if self.dead_for > 0.0 {
            let (sw, sh) = (screen_width(), screen_height());
            draw_rectangle(
                0.0,
                0.0,
                sw,
                sh,
                Color::new(0.0, 0.0, 0.0, (self.dead_for / 3.0).min(0.7)),
            );
            super::pixfont::draw_centered(
                assets,
                "SHIP DESTROYED",
                sw / 2.0,
                sh * 0.42,
                ui_scale * 3.0,
                Color::new(1.0, 0.3, 0.2, 1.0),
            );
        }
    }

    /// Objective line, time limit, dialogue captions, loss reason and the F8 station help.
    fn draw_mission(&self, assets: &Assets, ui: f32) {
        let (sw, sh) = (screen_width(), screen_height());
        let gold = Color::new(1.0, 0.85, 0.2, 1.0);
        let bone = Color::new(0.85, 0.8, 0.7, 1.0);
        if let Some(m) = &self.mission {
            if let Some(text) = m.objective() {
                let (n, total) = m.progress();
                let mut line = format!("OBJECTIVE {n}/{total}: {text}");
                if let Some(t) = m.time_left(&self.world) {
                    line.push_str(&format!("  [{}:{:02}]", t as u32 / 60, t as u32 % 60));
                }
                let w = pixfont::width(&line, ui * 0.5).min(sw - 20.0);
                draw_rectangle(
                    sw / 2.0 - w / 2.0 - 8.0,
                    6.0,
                    w + 16.0,
                    ui * 5.0 + 10.0,
                    Color::new(0.04, 0.07, 0.10, 0.7),
                );
                pixfont::draw_centered(
                    assets,
                    &line,
                    sw / 2.0,
                    11.0,
                    (ui * 0.5).round().max(1.0),
                    gold,
                );
            }
            match m.result() {
                Some(true) => pixfont::draw_centered(
                    assets,
                    "MISSION COMPLETE",
                    sw / 2.0,
                    sh * 0.3,
                    ui * 2.0,
                    gold,
                ),
                Some(false) if self.dead_for == 0.0 => {
                    let red = Color::new(1.0, 0.3, 0.2, 1.0);
                    pixfont::draw_centered(
                        assets,
                        "MISSION FAILED",
                        sw / 2.0,
                        sh * 0.3,
                        ui * 2.0,
                        red,
                    );
                    pixfont::draw_centered(
                        assets,
                        m.reason(),
                        sw / 2.0,
                        sh * 0.3 + ui * 14.0,
                        (ui * 0.5).round().max(1.0),
                        bone,
                    );
                }
                _ => {}
            }
        }
        let fs = (ui * 0.5).round().max(1.0);
        for (i, (n, ttl)) in self.captions.iter().enumerate() {
            let a = (ttl / 1.0).min(1.0);
            let y = sh * 0.72 - (self.captions.len() - i) as f32 * ui * 7.0;
            let line = format!("{}: {}", n.who, n.text);
            let w = pixfont::width(&line, fs).min(sw - 20.0);
            draw_rectangle(
                sw / 2.0 - w / 2.0 - 8.0,
                y - 4.0,
                w + 16.0,
                fs * 7.0 + 8.0,
                Color::new(0.04, 0.07, 0.10, 0.65 * a),
            );
            pixfont::draw_centered(
                assets,
                &line,
                sw / 2.0,
                y,
                fs,
                Color::new(bone.r, bone.g, bone.b, a),
            );
        }
        if self.help_station && self.ops().is_some_and(|o| o.docked.is_none()) {
            let lines = help::station(self.ui.tab);
            let lh = ui * 4.5;
            let w = sw * 0.6;
            let (x, y) = (sw * 0.2, sh * 0.2);
            draw_rectangle(
                x - 10.0,
                y - 10.0,
                w + 20.0,
                lines.len() as f32 * lh + 40.0,
                Color::new(0.04, 0.07, 0.10, 0.9),
            );
            for (i, l) in lines.iter().enumerate() {
                let c = if i == 0 { gold } else { bone };
                pixfont::draw(assets, l, x, y + i as f32 * lh, fs, c);
            }
            pixfont::draw(
                assets,
                "F8 CLOSES THIS HELP",
                x,
                y + lines.len() as f32 * lh + 8.0,
                fs,
                Color::new(0.5, 0.5, 0.55, 1.0),
            );
        }
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

    /// Developer hook: hold the lance trigger down.
    pub fn debug_autofire(&mut self) {
        self.autofire = true;
    }

    /// Developer hook: open station tab `n` (1-7).
    pub fn debug_tab(&mut self, n: usize) {
        if let Some(t) = Tab::ALL.get(n.saturating_sub(1)) {
            self.ui.tab = *t;
        }
    }

    /// Developer hook: spawn an attacking squad ahead of the player and target it.
    pub fn debug_fight(&mut self) {
        use crate::sim::ai::{Mode, spawn_minion};
        use crate::sim::flight::{CUTTER, FRIGATE, INTERCEPTOR};
        let Some(me) = self
            .player_id()
            .and_then(|p| self.world.get(p))
            .map(|e| (e.pos, e.rot))
        else {
            return;
        };
        let ahead = |d: f32, side: f32| me.0 + me.1 * Vec3::new(side, 0.0, -d);
        let a = spawn_minion(&mut self.world, CUTTER, ahead(1100.0, 0.0), Mode::Attack);
        spawn_minion(
            &mut self.world,
            INTERCEPTOR,
            ahead(1500.0, 250.0),
            Mode::Attack,
        );
        spawn_minion(
            &mut self.world,
            FRIGATE,
            ahead(2200.0, -400.0),
            Mode::Attack,
        );
        self.target = Some(a);
    }

    /// Developer hook: teleport next to the nearest friendly station and dock.
    pub fn debug_dock(&mut self) {
        let Some(me) = self.player_id() else { return };
        let st = self
            .world
            .iter()
            .find(|(_, e)| e.kind == Kind::Station && e.faction == Faction::Mars)
            .map(|(i, e)| (i, e.pos, e.radius));
        if let Some((id, pos, rad)) = st {
            if let Some(e) = self.world.get_mut(me) {
                e.pos = pos + Vec3::new(0.0, 0.0, rad + 150.0);
                e.vel = Vec3::ZERO;
            }
            self.target = Some(id);
            self.actions.push(Action::Cmd(Command::Dock(Some(id))));
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::{Objective, Outcome};
    use crate::sim::ai::{Mode, spawn_minion};
    use crate::sim::flight::CUTTER;

    fn scene_with_victim() -> (SpaceScene, EntityId) {
        let mut sc = SpaceScene::new(SectorId::MarsOrbit, 1);
        let me = sc
            .world
            .player
            .and_then(|p| sc.world.get(p))
            .map_or(Vec3::ZERO, |e| e.pos);
        let v = spawn_minion(
            &mut sc.world,
            CUTTER,
            me + Vec3::new(0.0, 0.0, -120.0),
            Mode::Patrol,
        );
        sc.target = Some(v);
        (sc, v)
    }

    #[test]
    fn only_crippled_nearby_hostiles_can_be_boarded() {
        let (mut sc, v) = scene_with_victim();
        assert!(sc.boardable().is_none(), "healthy ships cannot be boarded");
        if let Some(o) = sc.world.get_mut(v).and_then(|e| e.ops.as_deref_mut()) {
            o.hull = 10.0;
            o.shields = [0.0; 4];
        }
        assert!(sc.boardable().is_some());
        if let Some(e) = sc.world.get_mut(v) {
            e.pos += Vec3::new(0.0, 0.0, -3000.0);
        }
        assert!(sc.boardable().is_none(), "too far to clamp on");
    }

    #[test]
    fn winning_a_boarding_pays_loot_and_removes_the_ship() {
        let (mut sc, v) = scene_with_victim();
        if let Some(o) = sc.world.get_mut(v).and_then(|e| e.ops.as_deref_mut()) {
            o.disabled = true;
        }
        let info = sc.boardable().expect("disabled ship is boardable");
        let res = BoardResult {
            outcome: Outcome::Won,
            objective: Objective::CaptureBridge,
            objective_done: true,
            kills: 4,
            credits: 360,
            damage_taken: 52.0,
        };
        let credits = |sc: &SpaceScene| sc.ops().map_or(0, |o| o.cargo.credits);
        let (c0, crew0) = (credits(&sc), sc.ops().map_or(0, |o| o.sys.crew));
        sc.apply_boarding(&info, &res);
        assert_eq!(credits(&sc) - c0, 360);
        assert!(sc.ops().is_some_and(|o| o.sys.crew == crew0 - 2));
        assert!(sc.world.get(v).is_none());
    }

    #[test]
    fn mission_start_clears_stock_enemies_and_applies_carry() {
        let mut sc = SpaceScene::new(SectorId::MarsOrbit, 1);
        let carry = Carry {
            credits: 777,
            up: [1, 2, 1, 1, 1],
        };
        sc.start_mission(1, &carry, 0.7);
        assert_eq!(sc.world.difficulty, 0.7);
        assert_eq!(
            sc.ops().map(|o| (o.cargo.credits, o.up.shield)),
            Some((777, 2))
        );
        assert!(
            !sc.world
                .iter()
                .any(|(_, e)| e.kind == Kind::Ship && e.faction == Faction::Minion)
        );
        assert_eq!(sc.outcome(), None);
        assert!(sc.mission.as_ref().and_then(|m| m.objective()).is_some());
    }

    #[test]
    fn decided_mission_ends_after_a_beat() {
        let mut sc = SpaceScene::new(SectorId::MarsOrbit, 1);
        sc.start_mission(1, &Carry::default(), 1.0);
        if let Some(m) = sc.mission.as_mut() {
            m.fail("TEST");
        }
        assert_eq!(sc.outcome(), None, "waits for the failure banner");
        sc.ended_for = 3.5;
        assert_eq!(sc.outcome(), Some(false));
        assert_eq!(sc.loss_reason(), "TEST");
    }
}
