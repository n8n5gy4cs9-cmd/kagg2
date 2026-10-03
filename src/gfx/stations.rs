//! Station tabs (T10): Helm (the flight HUD), Weapons, Engineering, Damage, Science, Comms, Map.
//!
//! Panels are drawn over the live 3D view; the sim keeps running. They never touch the world:
//! every click becomes an [`Action`] (mostly a sim `Command`).

use macroquad::prelude::*;

use super::ui::{self, Action, BAD, BONE, CYAN, Ctx, EMBER, GOOD, WARN, health_color};
use crate::sim::command::Command;
use crate::sim::comms::Hail;
use crate::sim::crew::Station;
use crate::sim::flight::FORWARD;
use crate::sim::ops::{Dest, FACING_NAMES, Faction, Ops};
use crate::sim::science::{self, sensor_range};
use crate::sim::systems::{POWERED, SYS_COUNT, Sys};
use crate::sim::{EntityId, Kind, SectorId, World};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Helm,
    Weapons,
    Engineering,
    Damage,
    Science,
    Comms,
    Map,
}

impl Tab {
    pub const ALL: [Tab; 7] = [
        Tab::Helm,
        Tab::Weapons,
        Tab::Engineering,
        Tab::Damage,
        Tab::Science,
        Tab::Comms,
        Tab::Map,
    ];

    pub fn station(self) -> Station {
        Station::ALL[self as usize]
    }

    pub fn key(self) -> KeyCode {
        match self {
            Tab::Helm => KeyCode::F1,
            Tab::Weapons => KeyCode::F2,
            Tab::Engineering => KeyCode::F3,
            Tab::Damage => KeyCode::F4,
            Tab::Science => KeyCode::F5,
            Tab::Comms => KeyCode::F6,
            Tab::Map => KeyCode::F7,
        }
    }
}

/// UI state that outlives a frame.
pub struct UiState {
    pub tab: Tab,
    pub map_range: f32,
    pub dialog: Option<(EntityId, Hail)>,
    pub log: Vec<String>,
}

impl Default for UiState {
    fn default() -> Self {
        UiState {
            tab: Tab::Helm,
            map_range: 9000.0,
            dialog: None,
            log: Vec::new(),
        }
    }
}

impl UiState {
    pub fn say(&mut self, line: impl Into<String>) {
        self.log.push(line.into());
        if self.log.len() > 60 {
            self.log.remove(0);
        }
    }
}

pub struct Frame<'a> {
    pub world: &'a World,
    pub me: EntityId,
    pub target: Option<EntityId>,
}

fn ops<'a>(f: &Frame<'a>) -> Option<&'a Ops> {
    f.world.get(f.me).and_then(|e| e.ops.as_deref())
}

/// Tab bar along the top; returns its bottom edge.
pub fn tab_bar(c: &Ctx, ui: &mut UiState, f: &Frame) -> f32 {
    let sw = screen_width();
    let h = c.lh() + 10.0;
    let w = (sw * 0.9 / 7.0).min(180.0 * c.fs / 2.0);
    let x0 = (sw - w * 7.0) / 2.0;
    for (i, tab) in Tab::ALL.iter().enumerate() {
        let st = tab.station();
        let auto = f.world.crew.is_auto(st);
        let r = Rect::new(x0 + i as f32 * w, 4.0, w - 3.0, h);
        let label = format!("F{} {}", i + 1, st.name());
        if c.button(r, &label, ui.tab == *tab) {
            ui.tab = *tab;
        }
        if auto {
            c.text_center("AUTO", r.x + r.w / 2.0, r.y + h + 1.0, GOOD);
        }
    }
    4.0 + h + c.lh()
}

/// Draw the panel of the current tab (nothing for Helm).
pub fn draw_panel(c: &Ctx, ui: &mut UiState, f: &Frame, actions: &mut Vec<Action>, top: f32) {
    if ui.tab == Tab::Helm {
        return;
    }
    let (sw, sh) = (screen_width(), screen_height());
    let r = Rect::new(sw * 0.07, top, sw * 0.86, sh * 0.74 - top * 0.5);
    let st = ui.tab.station();
    let auto = f.world.crew.is_auto(st);
    let title = format!(
        "{} STATION{}   (TAB = TOGGLE CREW AUTO)",
        st.name(),
        if auto { "  [CREW AUTO]" } else { "" }
    );
    c.panel(r, &title);
    let inner = Rect::new(
        r.x + 10.0,
        r.y + c.lh() + 12.0,
        r.w - 20.0,
        r.h - c.lh() - 22.0,
    );
    match ui.tab {
        Tab::Weapons => weapons(c, inner, f, actions),
        Tab::Engineering => engineering(c, inner, f, actions),
        Tab::Damage => damage(c, inner, f, actions),
        Tab::Science => science_tab(c, inner, f, actions),
        Tab::Comms => comms(c, inner, ui, f, actions),
        Tab::Map => map(c, inner, ui, f, actions),
        Tab::Helm => {}
    }
}

fn row(r: Rect, i: usize, c: &Ctx) -> f32 {
    r.y + i as f32 * (c.lh() + 8.0)
}

fn freq_row(
    c: &Ctx,
    r: Rect,
    y: f32,
    label: &str,
    val: u8,
    set: impl Fn(u8) -> Command,
    a: &mut Vec<Action>,
) {
    c.text(label, r.x, y, BONE);
    let bw = c.lh() + 6.0;
    let bx = r.x + 17.0 * 6.0 * c.fs;
    if c.button(Rect::new(bx, y - 4.0, bw, bw), "<", false) {
        a.push(Action::Cmd(set(val.saturating_sub(1))));
    }
    c.text_center(&val.to_string(), bx + bw + 14.0, y, EMBER);
    if c.button(Rect::new(bx + bw + 28.0, y - 4.0, bw, bw), ">", false) {
        a.push(Action::Cmd(set((val + 1).min(9))));
    }
}

fn weapons(c: &Ctx, r: Rect, f: &Frame, a: &mut Vec<Action>) {
    let Some(o) = ops(f) else { return };
    let u = c.fs;
    // --- shield diagram
    let (cx, cy) = (r.x + r.w * 0.2, r.y + r.h * 0.42);
    let (iw, ih) = (14.0 * u, 26.0 * u);
    draw_rectangle(
        cx - iw / 2.0,
        cy - ih / 2.0,
        iw,
        ih,
        Color::new(0.4, 0.45, 0.5, 0.8),
    );
    draw_triangle(
        vec2(cx - iw / 2.0, cy - ih / 2.0),
        vec2(cx + iw / 2.0, cy - ih / 2.0),
        vec2(cx, cy - ih / 2.0 - 8.0 * u),
        Color::new(0.5, 0.55, 0.6, 0.9),
    );
    let frac = |i: usize| o.shields[i] / o.max_shield.max(1.0);
    let (bl, bt) = (34.0 * u, 4.0 * u);
    let hb = |x: f32, y: f32, fr: f32| c.bar(Rect::new(x, y, bl, bt), fr, health_color(fr));
    hb(cx - bl / 2.0, cy - ih / 2.0 - 8.0 * u - bt - 3.0, frac(0)); // fore
    hb(cx - bl / 2.0, cy + ih / 2.0 + 4.0 * u, frac(1)); // aft
    for (i, x) in [
        (2usize, cx - iw / 2.0 - 12.0 * u),
        (3usize, cx + iw / 2.0 + 8.0 * u),
    ] {
        let (y0, hgt) = (cy - 13.0 * u, 26.0 * u);
        draw_rectangle(x, y0, bt, hgt, Color::new(0.0, 0.0, 0.0, 0.55));
        let fr = frac(i).clamp(0.0, 1.0);
        draw_rectangle(x, y0 + hgt * (1.0 - fr), bt, hgt * fr, health_color(fr));
    }
    c.text_center(
        "SHIELDS",
        cx,
        cy - ih / 2.0 - 8.0 * u - bt - 3.0 - c.lh() - 2.0,
        CYAN,
    );
    let sfac = o.sys.eff(Sys::Shields);
    c.text_center(
        &format!("FORE {:.0}  AFT {:.0}", o.shields[0], o.shields[1]),
        cx,
        cy + ih / 2.0 + 4.0 * u + bt + 6.0,
        BONE,
    );
    c.text_center(
        &format!(
            "PORT {:.0}  STBD {:.0}  GEN {:.0}%",
            o.shields[2],
            o.shields[3],
            sfac * 100.0
        ),
        cx,
        cy + ih / 2.0 + 4.0 * u + bt + 6.0 + c.lh(),
        BONE,
    );
    // --- controls
    let rr = Rect::new(r.x + r.w * 0.42, r.y, r.w * 0.58, r.h);
    let bw = 40.0 * 6.0 * u * 0.5;
    let hullf = o.hull / o.max_hull;
    c.text("HULL", rr.x, row(rr, 0, c), BONE);
    c.bar(
        Rect::new(rr.x + 8.0 * 6.0 * u, row(rr, 0, c), bw, c.lh() - 2.0),
        hullf,
        health_color(hullf),
    );
    c.text(
        &format!("{:.0}/{:.0}", o.hull, o.max_hull),
        rr.x + 8.0 * 6.0 * u + bw + 8.0,
        row(rr, 0, c),
        BONE,
    );
    freq_row(
        c,
        rr,
        row(rr, 1, c),
        "SHIELD FREQUENCY",
        o.shield_freq,
        Command::SetShieldFreq,
        a,
    );
    freq_row(
        c,
        rr,
        row(rr, 2, c),
        "LANCE FREQUENCY",
        o.lance_freq,
        Command::SetLanceFreq,
        a,
    );
    let heat = o.sys.heat[Sys::Lances.idx()];
    c.text("LANCE HEAT", rr.x, row(rr, 3, c), BONE);
    c.bar(
        Rect::new(rr.x + 17.0 * 6.0 * u, row(rr, 3, c), bw * 0.8, c.lh() - 2.0),
        heat / 100.0,
        health_color(1.0 - heat / 100.0),
    );
    let fire = |label: &str, y: f32, cmd: Command, ok: bool, a: &mut Vec<Action>| {
        let b = Rect::new(
            rr.x + 17.0 * 6.0 * u,
            y - 4.0,
            26.0 * 6.0 * u * 0.5,
            c.lh() + 6.0,
        );
        if c.button(b, label, false) && ok {
            a.push(Action::Cmd(cmd));
        }
    };
    c.text("RUNE LANCES", rr.x, row(rr, 4, c), BONE);
    fire("FIRE [ENTER]", row(rr, 4, c), Command::FireLance, true, a);
    c.text(
        &format!("TORPEDOES {}/{}", o.torpedoes, o.max_torpedoes()),
        rr.x,
        row(rr, 5, c),
        BONE,
    );
    fire("FIRE [Z]", row(rr, 5, c), Command::FireTorpedo, true, a);
    c.text(
        &format!("MISSILES  {}/{}", o.missiles, o.max_missiles()),
        rr.x,
        row(rr, 6, c),
        BONE,
    );
    fire("FIRE [V]", row(rr, 6, c), Command::FireMissile, true, a);
    // --- target block
    let y = row(rr, 8, c);
    c.text("TARGET", rr.x, y, CYAN);
    match f.target.and_then(|t| f.world.get(t).map(|e| (t, e))) {
        None => c.text("NONE - PRESS T", rr.x + 8.0 * 6.0 * u, y, BONE),
        Some((t, e)) => {
            let me = f.world.get(f.me).map_or(Vec3::ZERO, |m| m.pos);
            c.text(
                &format!("{}  RNG {:.0}", e.label, e.pos.distance(me)),
                rr.x + 8.0 * 6.0 * u,
                y,
                if e.faction.hostile_to(Faction::Mars) {
                    BAD
                } else {
                    BONE
                },
            );
            match science::report(f.world, f.me, t) {
                Some(rep) => {
                    c.text(
                        &format!("ENEMY SHIELD FREQ {}  ->  TUNE LANCES", rep.shield_freq),
                        rr.x,
                        row(rr, 9, c),
                        WARN,
                    );
                    let b = Rect::new(
                        rr.x,
                        row(rr, 10, c) - 4.0,
                        30.0 * 6.0 * u * 0.5,
                        c.lh() + 6.0,
                    );
                    if c.button(
                        b,
                        "MATCH LANCES",
                        o.lance_freq.abs_diff(rep.shield_freq) <= 1,
                    ) {
                        a.push(Action::Cmd(Command::SetLanceFreq(rep.shield_freq)));
                    }
                }
                None if e.kind == Kind::Ship => c.text(
                    "SCAN TARGET AT SCIENCE (F5) TO LEARN ITS SHIELD FREQUENCY",
                    rr.x,
                    row(rr, 9, c),
                    BONE,
                ),
                None => {}
            }
        }
    }
}

fn engineering(c: &Ctx, r: Rect, f: &Frame, a: &mut Vec<Action>) {
    let Some(o) = ops(f) else { return };
    let u = c.fs;
    let s = &o.sys;
    let (out, dem) = (s.output(), s.demand());
    c.text(
        &format!(
            "REACTOR OUTPUT {out:.1}   DEMAND {dem:.1}   {}",
            if dem > out {
                "OVERLOAD - POWER CUT"
            } else {
                "OK"
            }
        ),
        r.x,
        r.y,
        if dem > out { BAD } else { GOOD },
    );
    c.bar(
        Rect::new(r.x, r.y + c.lh() + 2.0, r.w * 0.5, c.lh() - 2.0),
        (dem / out.max(0.1)).min(1.5) / 1.5,
        if dem > out { BAD } else { CYAN },
    );
    // presets
    for (i, name) in ["1 BALANCED", "2 BATTLE", "3 RUN"].iter().enumerate() {
        let b = Rect::new(
            r.x + r.w * 0.55 + i as f32 * 22.0 * 6.0 * u * 0.5,
            r.y,
            21.0 * 6.0 * u * 0.5,
            c.lh() + 6.0,
        );
        if c.button(b, name, false) {
            a.push(Action::Cmd(Command::PowerPreset(i as u8 + 1)));
        }
    }
    let y0 = r.y + 2.0 * c.lh() + 20.0;
    let (cx_name, cx_pow, cx_cool, cx_heat, cx_eff) = (
        r.x,
        r.x + 14.0 * 6.0 * u,
        r.x + r.w * 0.40,
        r.x + r.w * 0.68,
        r.x + r.w * 0.88,
    );
    let sw = r.w * 0.22;
    for (t, x) in [
        ("SYSTEM", cx_name),
        ("POWER", cx_pow),
        ("COOLANT", cx_cool),
        ("HEAT", cx_heat),
        ("OUTPUT", cx_eff),
    ] {
        c.text(t, x, y0 - c.lh() - 4.0, CYAN);
    }
    for (i, sys) in Sys::ALL.iter().take(POWERED).enumerate() {
        let y = y0 + i as f32 * (c.lh() + 10.0);
        c.text(sys.name(), cx_name, y, BONE);
        let pr = Rect::new(cx_pow, y, sw, c.lh());
        if let Some(v) = c.slider(pr, s.power[i], 2.0, CYAN) {
            a.push(Action::Cmd(Command::SetPower(
                *sys,
                (v * 20.0).round() / 20.0,
            )));
        }
        c.text(
            &format!("{:.0}%", s.power[i] * 100.0),
            pr.x + 4.0,
            y + 1.0,
            WHITE,
        );
        let cr = Rect::new(cx_cool, y, sw * 0.8, c.lh());
        if let Some(v) = c.slider(cr, s.coolant[i], 1.0, Color::new(0.4, 0.6, 1.0, 1.0)) {
            a.push(Action::Cmd(Command::SetCoolant(*sys, v)));
        }
        let hf = s.heat[i] / 100.0;
        c.bar(
            Rect::new(cx_heat, y, r.w * 0.16, c.lh()),
            hf,
            if hf > 0.8 {
                BAD
            } else if hf > 0.5 {
                WARN
            } else {
                GOOD
            },
        );
        c.text(
            &format!("{:.0}%", s.eff(*sys) * 100.0),
            cx_eff,
            y,
            health_color(s.eff(*sys).min(1.0)),
        );
    }
    let y = y0 + POWERED as f32 * (c.lh() + 10.0) + 6.0;
    c.text(
        &format!(
            "REACTOR HEALTH {:.0}%   LEVEL {}",
            s.health[Sys::Reactor.idx()],
            s.reactor_level
        ),
        r.x,
        y,
        BONE,
    );
    c.text(
        "DRAG THE BARS WITH THE MOUSE. OVERHEATED SYSTEMS BURN OUT - GIVE THEM COOLANT.",
        r.x,
        y + c.lh() + 4.0,
        Color::new(0.6, 0.65, 0.7, 1.0),
    );
}

fn damage(c: &Ctx, r: Rect, f: &Frame, a: &mut Vec<Action>) {
    let Some(o) = ops(f) else { return };
    let u = c.fs;
    let hullf = o.hull / o.max_hull;
    c.text("HULL", r.x, r.y, BONE);
    c.bar(
        Rect::new(r.x + 8.0 * 6.0 * u, r.y, r.w * 0.3, c.lh() - 2.0),
        hullf,
        health_color(hullf),
    );
    c.text(
        &format!("{:.0}/{:.0}", o.hull, o.max_hull),
        r.x + 8.0 * 6.0 * u + r.w * 0.3 + 8.0,
        r.y,
        BONE,
    );
    c.text(
        &format!("CREW {}/{}", o.sys.crew, o.sys.crew_max),
        r.x + r.w * 0.62,
        r.y,
        if o.sys.crew * 2 < o.sys.crew_max {
            BAD
        } else {
            BONE
        },
    );
    c.text(
        "CLICK A SYSTEM TO ASSIGN / ADD REPAIR TEAMS (3 TEAMS, 1.5%/S EACH)",
        r.x,
        r.y + c.lh() + 4.0,
        Color::new(0.6, 0.65, 0.7, 1.0),
    );
    let y0 = r.y + 2.0 * c.lh() + 22.0;
    for i in 0..SYS_COUNT {
        let sys = Sys::ALL[i];
        let y = y0 + i as f32 * (c.lh() + 10.0);
        let hp = o.sys.health[i];
        let rowr = Rect::new(r.x, y - 4.0, r.w * 0.8, c.lh() + 8.0);
        let teams_here = o.sys.teams.iter().filter(|t| **t == Some(sys)).count();
        if c.hover(rowr) {
            draw_rectangle(
                rowr.x,
                rowr.y,
                rowr.w,
                rowr.h,
                Color::new(1.0, 1.0, 1.0, 0.07),
            );
        }
        if c.hover(rowr) && (c.clicked || c.right_clicked) {
            // cycle 0 -> 1 -> 2 -> 3 -> 0 teams (right click goes down)
            let want = if c.right_clicked {
                teams_here.saturating_sub(1)
            } else {
                (teams_here + 1) % 4
            };
            let mut count = 0;
            for t in 0..3u8 {
                let here = o.sys.teams[t as usize] == Some(sys);
                let assign = if count < want
                    && (here || o.sys.teams[t as usize].is_none() || want > teams_here)
                {
                    count += 1;
                    Some(sys)
                } else if here {
                    None
                } else {
                    o.sys.teams[t as usize]
                };
                a.push(Action::Cmd(Command::AssignRepair {
                    team: t,
                    sys: assign,
                }));
            }
        }
        c.text(sys.name(), r.x, y, BONE);
        c.bar(
            Rect::new(r.x + 14.0 * 6.0 * u, y, r.w * 0.3, c.lh()),
            hp / 100.0,
            health_color(hp / 100.0),
        );
        c.text(
            &format!("{hp:.0}%"),
            r.x + 14.0 * 6.0 * u + r.w * 0.3 + 8.0,
            y,
            BONE,
        );
        let state = if hp <= 0.0 {
            "OFFLINE"
        } else if hp < 50.0 {
            "DEGRADED"
        } else {
            ""
        };
        c.text(
            state,
            r.x + r.w * 0.62,
            y,
            if hp <= 0.0 { BAD } else { WARN },
        );
        c.text(&"#".repeat(teams_here), r.x + r.w * 0.76, y, GOOD);
    }
    let y = y0 + SYS_COUNT as f32 * (c.lh() + 10.0) + 8.0;
    let teams: Vec<String> = o
        .sys
        .teams
        .iter()
        .enumerate()
        .map(|(i, t)| format!("TEAM {}: {}", i + 1, t.map_or("IDLE", |s| s.name())))
        .collect();
    c.text(&teams.join("   "), r.x, y, CYAN);
}

fn contact_color(e: &crate::sim::Entity) -> Color {
    if e.faction.hostile_to(Faction::Mars) {
        BAD
    } else {
        match e.kind {
            Kind::Station => CYAN,
            Kind::Planet => EMBER,
            _ if e.faction == Faction::Mars => GOOD,
            _ => BONE,
        }
    }
}

fn science_tab(c: &Ctx, r: Rect, f: &Frame, a: &mut Vec<Action>) {
    let Some(o) = ops(f) else { return };
    let u = c.fs;
    let me = f.world.get(f.me).map_or(Vec3::ZERO, |m| m.pos);
    let range = sensor_range(o);
    // contact list (left)
    c.text(
        &format!("CONTACTS  (SENSOR RANGE {range:.0})"),
        r.x,
        r.y,
        CYAN,
    );
    let mut list: Vec<(EntityId, f32)> = f
        .world
        .iter()
        .filter(|(id, e)| {
            *id != f.me
                && matches!(
                    e.kind,
                    Kind::Ship | Kind::Station | Kind::Moon | Kind::Planet
                )
                && e.pos.distance(me) < range
        })
        .map(|(id, e)| (id, e.pos.distance(me)))
        .collect();
    list.sort_by(|x, y| x.1.total_cmp(&y.1));
    for (i, (id, d)) in list.iter().take(12).enumerate() {
        let Some(e) = f.world.get(*id) else { continue };
        let y = r.y + (i as f32 + 1.0) * (c.lh() + 4.0) + 6.0;
        let rowr = Rect::new(r.x, y - 2.0, r.w * 0.38, c.lh() + 2.0);
        let sel = f.target == Some(*id);
        if sel {
            draw_rectangle(
                rowr.x,
                rowr.y,
                rowr.w,
                rowr.h,
                Color::new(1.0, 0.85, 0.2, 0.18),
            );
        } else if c.hover(rowr) {
            draw_rectangle(
                rowr.x,
                rowr.y,
                rowr.w,
                rowr.h,
                Color::new(1.0, 1.0, 1.0, 0.08),
            );
        }
        if c.hover(rowr) && c.clicked {
            a.push(Action::SelectTarget(*id));
        }
        let done = o.scan.done.contains(id);
        c.text(
            &format!("{}{}", e.label, if done { " *" } else { "" }),
            r.x + 4.0,
            y,
            contact_color(e),
        );
        c.text_right(&format!("{d:.0}"), rowr.x + rowr.w - 4.0, y, BONE);
    }
    // detail (right)
    let dx = r.x + r.w * 0.42;
    let btn = |x: f32, label: &str, active: bool| {
        c.button(
            Rect::new(x, r.y - 4.0, 24.0 * 6.0 * u * 0.5, c.lh() + 6.0),
            label,
            active,
        )
    };
    if btn(
        dx,
        "SCAN TARGET [N]",
        o.scan.target.is_some() && o.scan.target == f.target,
    ) && let Some(t) = f.target
    {
        a.push(Action::Cmd(Command::Scan(Some(t))));
    }
    let sweep_label = if o.scan.sweep_cd > 0.0 {
        format!("SWEEP {:.0}S [U]", o.scan.sweep_cd)
    } else {
        "SWEEP [U]".to_owned()
    };
    if btn(dx + 26.0 * 6.0 * u * 0.5, &sweep_label, o.scan.sweep > 0.0) {
        a.push(Action::Cmd(Command::Sweep));
    }
    let Some((tid, t)) = f.target.and_then(|t| f.world.get(t).map(|e| (t, e))) else {
        c.text(
            "SELECT A CONTACT (CLICK OR T) THEN SCAN IT.",
            dx,
            r.y + 3.0 * c.lh(),
            BONE,
        );
        return;
    };
    let mut y = r.y + 2.0 * c.lh() + 8.0;
    c.text(t.label, dx, y, contact_color(t));
    y += c.lh() + 6.0;
    let kind = match t.kind {
        Kind::Ship => "SHIP",
        Kind::Station => "STATION",
        Kind::Planet => "PLANET",
        Kind::Moon => "MOON",
        _ => "ROCK",
    };
    c.text(
        &format!("TYPE {kind}   RANGE {:.0}", t.pos.distance(me)),
        dx,
        y,
        BONE,
    );
    y += c.lh() + 6.0;
    if o.scan.target == Some(tid) && !o.scan.done.contains(&tid) {
        c.text("SCANNING", dx, y, WARN);
        c.bar(
            Rect::new(dx + 10.0 * 6.0 * u, y, r.w * 0.3, c.lh() - 2.0),
            o.scan.progress,
            WARN,
        );
        y += c.lh() + 6.0;
    }
    match science::report(f.world, f.me, tid) {
        None if t.kind == Kind::Ship => {
            c.text("UNSCANNED - CLASS, SHIELDS AND CARGO UNKNOWN", dx, y, BONE)
        }
        None => c.text(&format!("FACTION {}", t.faction.name()), dx, y, BONE),
        Some(rep) => {
            c.text(
                &format!(
                    "CLASS {}   FACTION {}",
                    rep.class.to_uppercase(),
                    rep.faction.name()
                ),
                dx,
                y,
                BONE,
            );
            y += c.lh() + 6.0;
            let hf = rep.hull / rep.max_hull;
            c.text("HULL", dx, y, BONE);
            c.bar(
                Rect::new(dx + 8.0 * 6.0 * u, y, r.w * 0.25, c.lh() - 2.0),
                hf,
                health_color(hf),
            );
            c.text(
                &format!("{:.0}/{:.0}", rep.hull, rep.max_hull),
                dx + 8.0 * 6.0 * u + r.w * 0.25 + 8.0,
                y,
                BONE,
            );
            y += c.lh() + 6.0;
            for (i, name) in FACING_NAMES.iter().enumerate() {
                let sf = rep.shields[i] / rep.max_shield.max(1.0);
                c.text(
                    &format!("SHLD {name}"),
                    dx,
                    y,
                    if i == rep.weakest { WARN } else { BONE },
                );
                c.bar(
                    Rect::new(dx + 12.0 * 6.0 * u, y, r.w * 0.2, c.lh() - 2.0),
                    sf,
                    health_color(sf),
                );
                y += c.lh() + 3.0;
            }
            y += 4.0;
            c.text(
                &format!(
                    "SHIELD FREQ {}   LANCE FREQ {}",
                    rep.shield_freq, rep.lance_freq
                ),
                dx,
                y,
                EMBER,
            );
            y += c.lh() + 6.0;
            c.text(
                &format!(
                    "CARGO {}   BOARDING DIFFICULTY {}/5",
                    rep.cargo, rep.boarding
                ),
                dx,
                y,
                BONE,
            );
            y += c.lh() + 6.0;
            for line in ui::wrap(&rep.hint, ((r.w * 0.55) / (6.0 * u)) as usize) {
                c.text(&line, dx, y, GOOD);
                y += c.lh();
            }
        }
    }
}

fn comms(c: &Ctx, r: Rect, ui: &mut UiState, f: &Frame, a: &mut Vec<Action>) {
    let u = c.fs;
    let target = f.target.and_then(|t| f.world.get(t).map(|e| (t, e)));
    let hb = Rect::new(r.x, r.y - 4.0, 26.0 * 6.0 * u * 0.5, c.lh() + 6.0);
    c.text(
        &format!("TARGET: {}", target.map_or("NONE (PRESS T)", |t| t.1.label)),
        r.x + hb.w + 12.0,
        r.y,
        BONE,
    );
    if c.button(hb, "HAIL [H]", false) {
        a.push(Action::Hail);
    }
    let cols = ((r.w * 0.5) / (6.0 * u)) as usize;
    let mut y = r.y + c.lh() + 20.0;
    if let Some((_, h)) = &ui.dialog {
        for line in ui::wrap(&h.greeting, cols) {
            c.text(&line, r.x, y, CYAN);
            y += c.lh();
        }
        y += 8.0;
        for (i, (label, _)) in h.options.iter().enumerate() {
            let b = Rect::new(r.x, y, r.w * 0.45, c.lh() + 8.0);
            if c.button(b, &format!("{}  {}", i + 1, label), false) {
                a.push(Action::Reply(i));
            }
            y += c.lh() + 12.0;
        }
    } else {
        c.text(
            "HAIL A CONTACT TO TALK. KEYS 1-9 PICK AN ANSWER.",
            r.x,
            y,
            Color::new(0.6, 0.65, 0.7, 1.0),
        );
    }
    // message log (right)
    let lx = r.x + r.w * 0.52;
    c.text("MESSAGE LOG", lx, r.y, CYAN);
    let lines: Vec<String> = ui
        .log
        .iter()
        .flat_map(|l| ui::wrap(l, ((r.w * 0.46) / (6.0 * u)) as usize))
        .collect();
    let rows = ((r.h - 3.0 * c.lh()) / c.lh()) as usize;
    for (i, l) in lines
        .iter()
        .rev()
        .take(rows)
        .collect::<Vec<_>>()
        .iter()
        .rev()
        .enumerate()
    {
        let col = if l.contains("BOUNTY") {
            GOOD
        } else if l.contains("DESTROYED") || l.contains("HELP") {
            BAD
        } else {
            BONE
        };
        c.text(l, lx, r.y + (i as f32 + 1.5) * c.lh(), col);
    }
}

fn map(c: &Ctx, r: Rect, ui: &mut UiState, f: &Frame, a: &mut Vec<Action>) {
    let Some(me) = f.world.get(f.me) else { return };
    let Some(o) = ops(f) else { return };
    let u = c.fs;
    let mr = Rect::new(r.x, r.y, r.w * 0.64, r.h);
    draw_rectangle(mr.x, mr.y, mr.w, mr.h, Color::new(0.0, 0.02, 0.04, 0.9));
    draw_rectangle_lines(
        mr.x,
        mr.y,
        mr.w,
        mr.h,
        1.0,
        Color::new(0.21, 0.78, 0.88, 0.5),
    );
    let centre = vec2(mr.x + mr.w / 2.0, mr.y + mr.h / 2.0);
    if c.hover(mr) {
        let (_, wy) = mouse_wheel();
        if wy != 0.0 {
            ui.map_range =
                (ui.map_range * if wy > 0.0 { 0.8 } else { 1.25 }).clamp(1500.0, 90_000.0);
        }
    }
    let scale = (mr.h / 2.0) / ui.map_range;
    let to_map = |p: Vec3| centre + vec2(p.x - me.pos.x, p.z - me.pos.z) * scale;
    let clip = |p: Vec2| mr.contains(p);
    for k in 1..=4 {
        draw_circle_lines(
            centre.x,
            centre.y,
            mr.h / 2.0 * k as f32 / 4.0,
            1.0,
            Color::new(0.21, 0.78, 0.88, 0.18),
        );
    }
    c.text(
        &format!(
            "RANGE {:.0}/RING {:.0}  (WHEEL = ZOOM)",
            ui.map_range,
            ui.map_range / 4.0
        ),
        mr.x + 6.0,
        mr.y + 4.0,
        Color::new(0.5, 0.6, 0.7, 1.0),
    );
    let sense = sensor_range(o);
    let mut best: Option<(f32, EntityId, Vec2)> = None;
    for (id, e) in f.world.iter() {
        if id == f.me || matches!(e.kind, Kind::Projectile | Kind::Debris) {
            continue;
        }
        let d = e.pos.distance(me.pos);
        let hidden_ship = e.kind == Kind::Ship && e.faction != Faction::Mars && d > sense;
        let small_rock = e.kind == Kind::Asteroid && e.radius < 100.0 && ui.map_range > 6000.0;
        if hidden_ship || small_rock {
            continue;
        }
        let p = to_map(e.pos);
        if !clip(p) {
            continue;
        }
        let col = contact_color(e);
        let rad = (e.radius * scale).max(2.0 * u);
        match e.kind {
            Kind::Planet | Kind::Moon => draw_circle(
                p.x,
                p.y,
                rad.min(mr.h * 0.3),
                Color::new(col.r, col.g, col.b, 0.5),
            ),
            Kind::Station => draw_rectangle(p.x - 3.0 * u, p.y - 3.0 * u, 6.0 * u, 6.0 * u, col),
            Kind::Ship => {
                let fwd = e.rot * FORWARD;
                let dir = vec2(fwd.x, fwd.z).normalize_or_zero();
                let n = vec2(-dir.y, dir.x);
                draw_triangle(
                    p + dir * 5.0 * u,
                    p - dir * 3.0 * u + n * 3.0 * u,
                    p - dir * 3.0 * u - n * 3.0 * u,
                    col,
                );
            }
            _ => draw_circle(
                p.x,
                p.y,
                rad.max(1.5 * u),
                Color::new(col.r, col.g, col.b, 0.6),
            ),
        }
        if f.target == Some(id) {
            draw_circle_lines(p.x, p.y, 7.0 * u, 2.0, Color::new(1.0, 0.85, 0.2, 1.0));
        }
        if !matches!(e.kind, Kind::Asteroid) {
            c.text(
                e.label,
                p.x + 5.0 * u,
                p.y - 3.0 * u,
                Color::new(col.r, col.g, col.b, 0.9),
            );
        }
        let md = c.mouse.distance(p);
        if md < 14.0 * u && best.is_none_or(|b| md < b.0) {
            best = Some((md, id, p));
        }
    }
    // route
    let mut prev = to_map(me.pos);
    let dest_pt = match o.nav.dest {
        Dest::Point(p) => Some(p),
        Dest::Body(id) => f.world.get(id).map(|e| e.pos),
        Dest::None => None,
    };
    for p in dest_pt.into_iter().chain(o.nav.route.iter().copied()) {
        let q = to_map(p);
        draw_line(
            prev.x,
            prev.y,
            q.x,
            q.y,
            1.0,
            Color::new(0.4, 1.0, 0.5, 0.8),
        );
        draw_poly_lines(q.x, q.y, 4, 5.0 * u, 45.0, 2.0, GOOD);
        prev = q;
    }
    // player
    let fwd = me.rot * FORWARD;
    let dir = vec2(fwd.x, fwd.z).normalize_or_zero();
    let n = vec2(-dir.y, dir.x);
    draw_triangle(
        centre + dir * 6.0 * u,
        centre - dir * 4.0 * u + n * 3.5 * u,
        centre - dir * 4.0 * u - n * 3.5 * u,
        Color::new(0.5, 1.0, 0.8, 1.0),
    );
    // clicks on the map
    if c.hover(mr) {
        let shift = is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift);
        if c.right_clicked {
            a.push(Action::Cmd(Command::SetDest(Dest::None)));
        } else if c.clicked {
            if let Some((_, id, _)) = best {
                a.push(Action::SelectTarget(id));
                let solid = f.world.get(id).is_some_and(|e| {
                    matches!(
                        e.kind,
                        Kind::Station | Kind::Moon | Kind::Planet | Kind::Asteroid
                    )
                });
                if solid && !shift {
                    a.push(Action::Cmd(Command::SetDest(Dest::Body(id))));
                }
            } else {
                let off = (c.mouse - centre) / scale;
                let p = Vec3::new(me.pos.x + off.x, me.pos.y, me.pos.z + off.y);
                a.push(Action::Cmd(if shift {
                    Command::AddWaypoint(p)
                } else {
                    Command::SetDest(Dest::Point(p))
                }));
            }
        }
    }
    // right column: autopilot + warp
    let rx = r.x + r.w * 0.67;
    let bw = r.w * 0.32;
    c.text("NAVIGATION", rx, r.y, CYAN);
    let ap = o.nav.autopilot;
    if c.button(
        Rect::new(rx, r.y + c.lh() + 6.0, bw * 0.48, c.lh() + 8.0),
        if ap {
            "AUTOPILOT ON [P]"
        } else {
            "AUTOPILOT [P]"
        },
        ap,
    ) {
        a.push(Action::Cmd(Command::Autopilot(!ap)));
    }
    if c.button(
        Rect::new(rx + bw * 0.52, r.y + c.lh() + 6.0, bw * 0.48, c.lh() + 8.0),
        "CLEAR COURSE",
        false,
    ) {
        a.push(Action::Cmd(Command::SetDest(Dest::None)));
    }
    c.text(
        "CLICK = GO TO  SHIFT = ADD WAYPOINT",
        rx,
        r.y + 2.0 * c.lh() + 20.0,
        Color::new(0.6, 0.65, 0.7, 1.0),
    );
    c.text(
        "RIGHT CLICK = CLEAR",
        rx,
        r.y + 3.0 * c.lh() + 22.0,
        Color::new(0.6, 0.65, 0.7, 1.0),
    );
    let mut y = r.y + 5.0 * c.lh() + 30.0;
    c.text("WARP DRIVE", rx, y, CYAN);
    y += c.lh() + 6.0;
    c.text(
        &format!("FUEL {:.0}/{:.0}", o.nav.fuel, o.nav.max_fuel),
        rx,
        y,
        BONE,
    );
    c.bar(
        Rect::new(rx + 12.0 * 6.0 * u, y, bw * 0.5, c.lh() - 2.0),
        o.nav.fuel / o.nav.max_fuel,
        health_color(o.nav.fuel / o.nav.max_fuel),
    );
    y += c.lh() + 8.0;
    let here = f.world.sector;
    for sec in SectorId::ALL {
        if sec == here {
            continue;
        }
        let cost = here.warp_cost(sec);
        let charging = matches!(o.nav.warp, Some((s, _)) if s == sec);
        let label = format!("{}  ({cost:.0})", sec.name());
        if c.button(Rect::new(rx, y, bw, c.lh() + 8.0), &label, charging) {
            a.push(Action::Cmd(Command::Warp(if charging {
                None
            } else {
                Some(sec)
            })));
        }
        y += c.lh() + 12.0;
    }
    if let Some((s, ch)) = o.nav.warp {
        c.text(
            &format!("CHARGING {}", s.name().to_uppercase()),
            rx,
            y,
            WARN,
        );
        c.bar(Rect::new(rx, y + c.lh() + 2.0, bw, c.lh() - 2.0), ch, WARN);
    }
}
