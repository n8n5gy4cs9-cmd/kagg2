//! Docked starbase screen (T16): services, market and shipyard.

use macroquad::prelude::*;

use super::ui::{BAD, BONE, CYAN, Ctx, EMBER, GOOD, WARN, health_color};
use crate::sim::command::Command;
use crate::sim::econ::{self, NAMES, Upgrade};
use crate::sim::ops::Ops;
use crate::sim::{EntityId, World};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DockTab {
    #[default]
    Services,
    Market,
    Shipyard,
}

#[derive(Default)]
pub struct DockState {
    pub tab: DockTab,
    pub notice: String,
}

fn ops_mut(w: &mut World, me: EntityId) -> Option<&mut Ops> {
    w.get_mut(me).and_then(|e| e.ops.as_deref_mut())
}

/// Draw the docked screen and apply purchases. Returns a command to send (undock).
pub fn draw(
    c: &Ctx,
    st: &mut DockState,
    w: &mut World,
    me: EntityId,
    station: EntityId,
) -> Option<Command> {
    let (sw, sh) = (screen_width(), screen_height());
    let (label, faction) = w
        .get(station)
        .map_or(("STARBASE", crate::sim::ops::Faction::Neutral), |e| {
            (e.label, e.faction)
        });
    let mult = econ::service_mult(label, faction);
    let t = w.time;
    let r = Rect::new(sw * 0.08, sh * 0.08, sw * 0.84, sh * 0.8);
    c.panel(r, &format!("{} - DOCKED", label.to_uppercase()));
    let inner = Rect::new(
        r.x + 12.0,
        r.y + c.lh() + 14.0,
        r.w - 24.0,
        r.h - c.lh() - 26.0,
    );
    let mut undock = None;
    for (i, (tab, name)) in [
        (DockTab::Services, "1 SERVICES"),
        (DockTab::Market, "2 MARKET"),
        (DockTab::Shipyard, "3 SHIPYARD"),
    ]
    .iter()
    .enumerate()
    {
        let b = Rect::new(
            inner.x + i as f32 * 22.0 * 6.0 * c.fs * 0.5,
            inner.y,
            21.0 * 6.0 * c.fs * 0.5,
            c.lh() + 8.0,
        );
        if c.button(b, name, st.tab == *tab) {
            st.tab = *tab;
        }
    }
    if is_key_pressed(KeyCode::Key1) {
        st.tab = DockTab::Services;
    } else if is_key_pressed(KeyCode::Key2) {
        st.tab = DockTab::Market;
    } else if is_key_pressed(KeyCode::Key3) {
        st.tab = DockTab::Shipyard;
    }
    let ub = Rect::new(
        inner.x + inner.w - 26.0 * 6.0 * c.fs * 0.5,
        inner.y,
        26.0 * 6.0 * c.fs * 0.5,
        c.lh() + 8.0,
    );
    if c.button(ub, "UNDOCK [L]", false) {
        undock = Some(Command::Dock(None));
    }
    let Some(o) = ops_mut(w, me) else {
        return undock;
    };
    c.text(
        &format!("CREDITS {}", o.cargo.credits),
        inner.x + inner.w * 0.5 - 60.0,
        inner.y + 4.0,
        EMBER,
    );
    let body = Rect::new(
        inner.x,
        inner.y + c.lh() + 24.0,
        inner.w,
        inner.h - c.lh() - 40.0,
    );
    let rowh = c.lh() + 12.0;
    let mut note: Option<String> = None;
    let mut result = |res: Result<(), &'static str>, ok: &str| {
        note = Some(match res {
            Ok(()) => ok.to_owned(),
            Err(e) => e.to_owned(),
        });
    };
    match st.tab {
        DockTab::Services => {
            let bw = 26.0 * 6.0 * c.fs * 0.5;
            let rows = [
                ("REPAIR HULL + SYSTEMS + CREW", econ::repair_cost(o, mult)),
                ("REARM TORPEDOES + MISSILES", econ::rearm_cost(o, mult)),
                ("REFUEL WARP DRIVE", econ::refuel_cost(o, mult)),
            ];
            for (i, (name, cost)) in rows.iter().enumerate() {
                let y = body.y + i as f32 * rowh;
                c.text(name, body.x, y, BONE);
                c.text_right(
                    &format!("{cost} CR"),
                    body.x + body.w - bw - 16.0,
                    y,
                    if *cost > 0 { EMBER } else { GOOD },
                );
                if c.button(
                    Rect::new(body.x + body.w - bw, y - 4.0, bw, c.lh() + 6.0),
                    "BUY",
                    false,
                ) {
                    match i {
                        0 => result(econ::repair(o, mult), "REPAIRS COMPLETE"),
                        1 => result(econ::rearm(o, mult), "ORDNANCE LOADED"),
                        _ => result(econ::refuel(o, mult), "TANKS FULL"),
                    }
                }
            }
            let y = body.y + 4.0 * rowh;
            let hf = o.hull / o.max_hull;
            c.text(
                &format!(
                    "HULL {:.0}/{:.0}   TORP {}/{}   MISS {}/{}   FUEL {:.0}/{:.0}   CREW {}/{}",
                    o.hull,
                    o.max_hull,
                    o.torpedoes,
                    o.max_torpedoes(),
                    o.missiles,
                    o.max_missiles(),
                    o.nav.fuel,
                    o.nav.max_fuel,
                    o.sys.crew,
                    o.sys.crew_max
                ),
                body.x,
                y,
                health_color(hf),
            );
            c.text(
                &format!(
                    "PRICE LEVEL x{mult:.1}{}",
                    if label == "Forge" {
                        "  (HOME BASE DISCOUNT)"
                    } else {
                        ""
                    }
                ),
                body.x,
                y + c.lh() + 6.0,
                CYAN,
            );
            c.text(
                "SHIELDS RECHARGE FREE WHILE DOCKED.",
                body.x,
                y + 2.0 * c.lh() + 12.0,
                Color::new(0.6, 0.65, 0.7, 1.0),
            );
        }
        DockTab::Market => {
            c.text("COMMODITY", body.x, body.y, CYAN);
            c.text("BUY", body.x + body.w * 0.30, body.y, CYAN);
            c.text("SELL", body.x + body.w * 0.40, body.y, CYAN);
            c.text("HELD", body.x + body.w * 0.50, body.y, CYAN);
            c.text(
                &format!("HOLD {}/{}", o.cargo.used(), o.cargo.capacity),
                body.x + body.w * 0.84,
                body.y,
                BONE,
            );
            let sm = c.lh() + 6.0;
            let bw = 8.0 * 6.0 * c.fs * 0.5 + 10.0;
            for (i, name) in NAMES.iter().enumerate() {
                let y = body.y + (i as f32 + 1.0) * (sm + 4.0) + 4.0;
                let (buy, sell) = (econ::price(label, i, t), econ::sell_price(label, i, t));
                c.text(name, body.x, y, if i == 0 { GOOD } else { BONE });
                c.text(&buy.to_string(), body.x + body.w * 0.30, y, EMBER);
                c.text(&sell.to_string(), body.x + body.w * 0.40, y, BONE);
                c.text(
                    &o.cargo.hold[i].to_string(),
                    body.x + body.w * 0.50,
                    y,
                    BONE,
                );
                let bx = body.x + body.w * 0.58;
                for (k, (lbl, qty, is_buy)) in [
                    ("BUY1", 1u16, true),
                    ("BUY5", 5, true),
                    ("SELL1", 1, false),
                    ("ALL", o.cargo.hold[i], false),
                ]
                .iter()
                .enumerate()
                {
                    let b = Rect::new(bx + k as f32 * (bw + 6.0), y - 4.0, bw, sm);
                    if c.button(b, lbl, false) {
                        if *is_buy {
                            result(econ::buy(o, label, i, *qty, t), "PURCHASED");
                        } else {
                            result(econ::sell(o, label, i, *qty, t), "SOLD");
                        }
                    }
                }
            }
            c.text("PRICES DRIFT EVERY 90 SECONDS AND DIFFER BETWEEN STATIONS. MINE ORE WITH K, SELL IT HERE.", body.x, body.y + 14.0 * (sm + 4.0) + 8.0, Color::new(0.6, 0.65, 0.7, 1.0));
        }
        DockTab::Shipyard => {
            let bw = 20.0 * 6.0 * c.fs * 0.5;
            for (i, u) in Upgrade::ALL.iter().enumerate() {
                let y = body.y + i as f32 * (rowh + c.lh() + 4.0);
                let lv = u.level(o);
                c.text(&format!("{}  LV {}/3", u.name(), lv), body.x, y, BONE);
                c.text(
                    u.blurb(),
                    body.x,
                    y + c.lh() + 2.0,
                    Color::new(0.6, 0.65, 0.7, 1.0),
                );
                match u.cost(o) {
                    Some(cost) => {
                        c.text_right(
                            &format!("{cost} CR"),
                            body.x + body.w - bw - 16.0,
                            y,
                            if o.cargo.credits >= cost { EMBER } else { BAD },
                        );
                        if c.button(
                            Rect::new(body.x + body.w - bw, y - 4.0, bw, c.lh() + 6.0),
                            "UPGRADE",
                            false,
                        ) {
                            result(econ::upgrade(o, *u), "UPGRADE INSTALLED");
                        }
                    }
                    None => c.text_right("MAX", body.x + body.w - 8.0, y, GOOD),
                }
            }
        }
    }
    if let Some(n) = note {
        st.notice = n;
    }
    if !st.notice.is_empty() {
        c.text(&st.notice, inner.x, inner.y + inner.h - c.lh(), WARN);
    }
    undock
}
