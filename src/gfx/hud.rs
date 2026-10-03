//! Flight HUD (screen space, crisp pixel font): speed/throttle, heading tape, crosshair,
//! prograde marker, target brackets, nav labels, radar scope and message log.

use macroquad::prelude::*;

use super::pixfont;
use crate::assets::Assets;
use crate::sim::flight::{FORWARD, RIGHT, UP};
use crate::sim::{EntityId, Kind, World};

const CYAN: Color = Color::new(0.21, 0.78, 0.88, 1.0);
const BONE: Color = Color::new(0.85, 0.80, 0.70, 1.0);
const EMBER: Color = Color::new(1.0, 0.54, 0.17, 1.0);
const PANEL: Color = Color::new(0.04, 0.07, 0.10, 0.62);

pub struct HudData<'a> {
    pub world: &'a World,
    pub player: EntityId,
    pub target: Option<EntityId>,
    pub view_proj: Mat4,
    pub cockpit: bool,
    pub messages: &'a [(String, f32)],
    pub show_help: bool,
    pub fov_deg: f32,
    pub ui_scale: f32,
}

fn kind_color(k: Kind) -> Color {
    match k {
        Kind::Station => CYAN,
        Kind::Planet => EMBER,
        Kind::Moon => Color::new(0.7, 0.7, 0.75, 1.0),
        Kind::Ship => Color::new(1.0, 0.8, 0.2, 1.0),
        Kind::Asteroid => Color::new(0.5, 0.45, 0.4, 1.0),
    }
}

fn kind_name(k: Kind) -> &'static str {
    match k {
        Kind::Station => "STATION",
        Kind::Planet => "PLANET",
        Kind::Moon => "MOON",
        Kind::Ship => "SHIP",
        Kind::Asteroid => "ASTEROID",
    }
}

fn project(m: &Mat4, p: Vec3, sw: f32, sh: f32) -> Option<(Vec2, f32)> {
    let c = *m * Vec4::new(p.x, p.y, p.z, 1.0);
    if c.w <= 0.0 {
        return None;
    }
    let (nx, ny) = (c.x / c.w, c.y / c.w);
    Some((
        vec2((nx * 0.5 + 0.5) * sw, (1.0 - (ny * 0.5 + 0.5)) * sh),
        c.w,
    ))
}

fn brackets(c: Vec2, half: f32, color: Color, t: f32) {
    let l = (half * 0.45).max(5.0);
    for (sx, sy) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
        let (x, y) = (c.x + sx * half, c.y + sy * half);
        draw_line(x, y, x - sx * l, y, t, color);
        draw_line(x, y, x, y - sy * l, t, color);
    }
}

fn fmt_dist(d: f32) -> String {
    if d >= 10_000.0 {
        format!("{:.1}K", d / 1000.0)
    } else {
        format!("{:.0}", d)
    }
}

pub fn draw_space(assets: &Assets, d: &HudData) {
    let (sw, sh) = (screen_width(), screen_height());
    let fs = d.ui_scale;
    let t = (fs * 0.5).round().max(1.0);
    let Some(me) = d.world.get(d.player) else {
        return;
    };
    let Some(ship) = me.ship else { return };
    let fwd = me.rot * FORWARD;
    let speed = me.vel.length();

    // ---- crosshair + prograde marker
    let c = vec2(sw / 2.0, sh / 2.0);
    let g = 10.0 * t;
    for (dx, dy) in [(-1.0, 0.0), (1.0, 0.0), (0.0, -1.0), (0.0, 1.0)] {
        draw_line(
            c.x + dx * g,
            c.y + dy * g,
            c.x + dx * (g + 9.0 * t),
            c.y + dy * (g + 9.0 * t),
            t,
            Color::new(0.5, 1.0, 0.8, 0.9),
        );
    }
    draw_rectangle(
        c.x - t / 2.0,
        c.y - t / 2.0,
        t,
        t,
        Color::new(0.5, 1.0, 0.8, 0.9),
    );
    if speed > 4.0
        && let Some((p, _)) = project(&d.view_proj, me.pos + me.vel.normalize() * 1000.0, sw, sh)
    {
        let col = Color::new(0.4, 1.0, 0.5, 0.95);
        draw_circle_lines(p.x, p.y, 7.0 * t, t, col);
        draw_line(p.x - 12.0 * t, p.y, p.x - 7.0 * t, p.y, t, col);
        draw_line(p.x + 7.0 * t, p.y, p.x + 12.0 * t, p.y, t, col);
        draw_line(p.x, p.y - 7.0 * t, p.x, p.y - 12.0 * t, t, col);
    }

    // ---- heading tape (top centre)
    let heading = fwd.x.atan2(-fwd.z).to_degrees().rem_euclid(360.0);
    let pitch = fwd.y.clamp(-1.0, 1.0).asin().to_degrees();
    let ty = sh * 0.085;
    let span = 8.0 * 6.0 * fs; // pixels per 10 degrees
    let px_per_deg = span / 10.0 * 0.6;
    for k in -5i32..=5 {
        let deg = ((heading / 10.0).round() as i32 + k) * 10;
        let x = c.x + (deg as f32 - heading) * px_per_deg;
        let major = deg.rem_euclid(30) == 0;
        draw_line(
            x,
            ty + 14.0 * fs,
            x,
            ty + (if major { 6.0 } else { 9.0 }) * fs + 8.0,
            t,
            Color::new(0.21, 0.78, 0.88, 0.8),
        );
        if major {
            let label = match deg.rem_euclid(360) {
                0 => "N".to_owned(),
                90 => "E".to_owned(),
                180 => "S".to_owned(),
                270 => "W".to_owned(),
                v => format!("{v}"),
            };
            pixfont::draw_centered(assets, &label, x, ty - 4.0 * fs, fs, BONE);
        }
    }
    draw_triangle(
        vec2(c.x - 4.0 * t, ty + 22.0 * fs),
        vec2(c.x + 4.0 * t, ty + 22.0 * fs),
        vec2(c.x, ty + 16.0 * fs),
        EMBER,
    );
    pixfont::draw_centered(
        assets,
        &format!("HDG {:03.0}  PIT {:+03.0}", heading, pitch),
        c.x,
        ty + 24.0 * fs,
        fs,
        CYAN,
    );

    // ---- throttle / speed panel (left)
    let (px, py, pw, ph) = (sw * 0.095, sh * 0.30, 60.0 * fs, sh * 0.36);
    draw_rectangle(px, py, pw, ph, PANEL);
    draw_rectangle_lines(px, py, pw, ph, t, Color::new(0.21, 0.78, 0.88, 0.55));
    let bar_h = ph - 40.0 * fs;
    let (bx, by) = (px + 10.0 * fs, py + 10.0 * fs);
    draw_rectangle(bx, by, 14.0 * fs, bar_h, Color::new(0.0, 0.0, 0.0, 0.5));
    let thr = ship.throttle.clamp(-0.3, 1.0);
    let zero_y = by + bar_h * (1.0 / 1.3);
    let thr_y = by + bar_h * ((1.0 - thr) / 1.3);
    let (top, bot) = if thr >= 0.0 {
        (thr_y, zero_y)
    } else {
        (zero_y, thr_y)
    };
    draw_rectangle(bx, top, 14.0 * fs, (bot - top).max(1.0), EMBER);
    draw_line(bx - 4.0, zero_y, bx + 14.0 * fs + 4.0, zero_y, t, BONE);
    let actual = (speed / ship.class.max_speed).clamp(0.0, 1.0);
    let ay = by + bar_h * ((1.0 - actual) / 1.3);
    draw_triangle(
        vec2(bx + 14.0 * fs + 3.0, ay),
        vec2(bx + 14.0 * fs + 3.0 + 6.0 * t, ay - 3.0 * t),
        vec2(bx + 14.0 * fs + 3.0 + 6.0 * t, ay + 3.0 * t),
        CYAN,
    );
    pixfont::draw(assets, "THR", px + 8.0 * fs, py + ph - 28.0 * fs, fs, BONE);
    pixfont::draw(
        assets,
        &format!("{:.0}%", thr * 100.0),
        px + 8.0 * fs,
        py + ph - 16.0 * fs,
        fs,
        EMBER,
    );
    pixfont::draw(
        assets,
        &format!("SPD {:>4.0}", speed),
        px,
        py - 10.0 * fs - 4.0,
        fs,
        CYAN,
    );

    // ---- targeting
    let range_lim = 90_000.0;
    for (id, e) in d.world.iter() {
        if id == d.player || e.pos.distance(me.pos) > range_lim {
            continue;
        }
        let dist = e.pos.distance(me.pos);
        let is_target = d.target == Some(id);
        let big = matches!(
            e.kind,
            Kind::Station | Kind::Planet | Kind::Moon | Kind::Ship
        ) || e.radius > 100.0;
        if !is_target && !big {
            continue;
        }
        if let Some((p, w)) = project(&d.view_proj, e.pos, sw, sh) {
            if p.x < -50.0 || p.x > sw + 50.0 || p.y < -50.0 || p.y > sh + 50.0 || w <= 0.0 {
                continue;
            }
            let col = kind_color(e.kind);
            let focal = sh / (2.0 * (d.fov_deg.to_radians() / 2.0).tan());
            let half = (e.radius / w * focal).clamp(10.0 * t, sh * 0.45);
            if is_target {
                brackets(p, half + 4.0 * t, Color::new(1.0, 0.85, 0.2, 1.0), t);
                pixfont::draw_centered(
                    assets,
                    &format!("{} {}", e.label, fmt_dist(dist)),
                    p.x,
                    p.y + half + 8.0 * t,
                    fs,
                    Color::new(1.0, 0.85, 0.2, 1.0),
                );
            } else if half < sh * 0.2 || dist > 3000.0 {
                // small diamond + label for navigation
                let r = 4.0 * t;
                draw_poly_lines(p.x, p.y, 4, r, 45.0, t, col);
                if !matches!(e.kind, Kind::Asteroid) {
                    pixfont::draw(
                        assets,
                        e.label,
                        p.x + r + 4.0,
                        p.y - 3.5 * fs,
                        fs,
                        Color::new(col.r, col.g, col.b, 0.85),
                    );
                }
            }
        }
    }

    // ---- target panel (top right)
    if let Some(tg) = d.target.and_then(|id| d.world.get(id)) {
        let (w, h) = (210.0 * fs / 1.5, 72.0 * fs / 1.5);
        let (x, y) = (sw - w - sw * 0.10, sh * 0.2);
        draw_rectangle(x, y, w, h, PANEL);
        draw_rectangle_lines(x, y, w, h, t, Color::new(1.0, 0.85, 0.2, 0.6));
        let rel = tg.pos - me.pos;
        let closing = -(tg.vel - me.vel).dot(rel.normalize_or_zero());
        let col = kind_color(tg.kind);
        pixfont::draw(
            assets,
            "TARGET",
            x + 6.0 * fs,
            y + 5.0 * fs,
            fs,
            Color::new(1.0, 0.85, 0.2, 1.0),
        );
        pixfont::draw(assets, tg.label, x + 6.0 * fs, y + 17.0 * fs, fs, col);
        pixfont::draw(
            assets,
            kind_name(tg.kind),
            x + 6.0 * fs,
            y + 29.0 * fs,
            fs,
            BONE,
        );
        pixfont::draw(
            assets,
            &format!("RNG {}", fmt_dist(rel.length())),
            x + 6.0 * fs,
            y + 41.0 * fs,
            fs,
            CYAN,
        );
        pixfont::draw(
            assets,
            &format!("CLS {:+.0}", closing),
            x + 6.0 * fs,
            y + 53.0 * fs,
            fs,
            CYAN,
        );
        // off-screen pointer
        let off = project(&d.view_proj, tg.pos, sw, sh)
            .is_none_or(|(p, _)| p.x < 0.0 || p.x > sw || p.y < 0.0 || p.y > sh);
        if off {
            let local = me.rot.inverse() * rel;
            let dir = vec2(local.dot(RIGHT), -local.dot(UP)).normalize_or_zero();
            let p = c + dir * (sh * 0.36);
            let n = vec2(-dir.y, dir.x);
            draw_triangle(
                p + dir * 10.0 * t,
                p - dir * 6.0 * t + n * 7.0 * t,
                p - dir * 6.0 * t - n * 7.0 * t,
                Color::new(1.0, 0.85, 0.2, 0.9),
            );
        }
    }

    // ---- radar scope (bottom right)
    let rr = 62.0 * fs / 1.5;
    let rc = vec2(sw - rr - sw * 0.10, sh * 0.80 - rr);
    draw_circle(rc.x, rc.y, rr, PANEL);
    for k in 1..=3 {
        draw_circle_lines(
            rc.x,
            rc.y,
            rr * k as f32 / 3.0,
            t,
            Color::new(0.21, 0.78, 0.88, 0.3),
        );
    }
    draw_line(
        rc.x - rr,
        rc.y,
        rc.x + rr,
        rc.y,
        t,
        Color::new(0.21, 0.78, 0.88, 0.2),
    );
    draw_line(
        rc.x,
        rc.y - rr,
        rc.x,
        rc.y + rr,
        t,
        Color::new(0.21, 0.78, 0.88, 0.2),
    );
    let range = 20_000.0f32;
    let inv = me.rot.inverse();
    for (id, e) in d.world.iter() {
        if id == d.player {
            continue;
        }
        let local = inv * (e.pos - me.pos);
        let flat = vec2(local.dot(RIGHT), local.dot(FORWARD)); // +y ahead
        let dist = flat.length().max(0.001);
        let rn = (dist / range).min(1.0).sqrt();
        let p = rc + vec2(flat.x / dist * rn * rr, -flat.y / dist * rn * rr);
        let col = if d.target == Some(id) {
            Color::new(1.0, 0.85, 0.2, 1.0)
        } else {
            kind_color(e.kind)
        };
        if e.kind == Kind::Asteroid && e.radius < 100.0 {
            draw_rectangle(p.x, p.y, t, t, Color::new(col.r, col.g, col.b, 0.5));
            continue;
        }
        let dy = (local.dot(UP) / range * rr * 3.0).clamp(-rr * 0.4, rr * 0.4);
        draw_line(
            p.x,
            p.y,
            p.x,
            p.y - dy,
            t,
            Color::new(col.r, col.g, col.b, 0.6),
        );
        let s = if e.kind == Kind::Planet {
            3.0 * t
        } else {
            2.0 * t
        };
        draw_rectangle(p.x - s / 2.0, p.y - dy - s / 2.0, s, s, col);
    }
    draw_triangle(
        vec2(rc.x, rc.y - 5.0 * t),
        vec2(rc.x - 3.5 * t, rc.y + 3.0 * t),
        vec2(rc.x + 3.5 * t, rc.y + 3.0 * t),
        Color::new(0.5, 1.0, 0.8, 1.0),
    );
    pixfont::draw_centered(assets, "SCOPE", rc.x, rc.y + rr + 4.0, fs, BONE);

    // ---- status line + messages + help
    let status = format!(
        "{}  |  {}  |  ASSIST {}  |  {}",
        d.world.sector.name().to_uppercase(),
        if d.cockpit { "COCKPIT" } else { "CHASE" },
        if ship.assist { "ON" } else { "OFF" },
        if ship.brake { "BRAKING" } else { "" },
    );
    pixfont::draw(
        assets,
        &status,
        24.0 * fs,
        sh - 18.0 * fs,
        fs,
        Color::new(0.8, 0.85, 0.9, 0.9),
    );
    for (i, (m, ttl)) in d.messages.iter().rev().take(4).enumerate() {
        let a = (ttl / 1.0).clamp(0.0, 1.0);
        pixfont::draw_centered(
            assets,
            m,
            c.x,
            sh * 0.70 - i as f32 * 14.0 * fs,
            fs,
            Color::new(1.0, 0.5, 0.3, a),
        );
    }
    if d.show_help {
        let lines = [
            "W/S THROTTLE  A/D YAW  UP/DOWN PITCH  Q/E ROLL",
            "LEFT/RIGHT STRAFE  R/F UP/DOWN  SPACE BRAKE  X STOP",
            "C CAMERA  T TARGET  G CLEAR  M FLIGHT ASSIST  H HELP",
        ];
        for (i, l) in lines.iter().enumerate() {
            pixfont::draw_centered(
                assets,
                l,
                c.x,
                sh * 0.78 + i as f32 * 12.0 * fs,
                fs,
                Color::new(0.85, 0.8, 0.7, 0.9),
            );
        }
    }
}
