//! Drawing of the non-gameplay screens: boot, main menu, briefing, debrief, placeholders.

use macroquad::prelude::*;

use crate::app::{MISSION_COUNT, Menu};
use crate::{story, version_label};

const RUST: Color = Color::new(0.85, 0.36, 0.20, 1.0);
const BONE: Color = Color::new(0.85, 0.80, 0.70, 1.0);
const DIM: Color = Color::new(0.45, 0.45, 0.48, 1.0);

fn centered(text: &str, y: f32, size: f32, color: Color) {
    let dims = measure_text(text, None, size as u16, 1.0);
    draw_text(text, (screen_width() - dims.width) / 2.0, y, size, color);
}

/// Screen rectangle of main-menu item `index`; shared by drawing and mouse hit-testing.
pub fn menu_item_rect(index: usize) -> Rect {
    let w = 320.0;
    let h = 44.0;
    let top = screen_height() * 0.52;
    Rect::new(
        (screen_width() - w) / 2.0,
        top + index as f32 * (h + 8.0),
        w,
        h,
    )
}

pub fn draw_boot() {
    centered(story::TITLE, screen_height() / 2.0, 96.0, RUST);
}

pub fn draw_main_menu(menu: &Menu) {
    let h = screen_height();
    centered(story::TITLE, h * 0.20, 110.0, RUST);
    centered(story::SUBTITLE, h * 0.20 + 40.0, 30.0, BONE);
    for (i, line) in story::BLURB.iter().enumerate() {
        centered(line, h * 0.20 + 80.0 + i as f32 * 24.0, 22.0, DIM);
    }
    for (i, item) in menu.items().iter().enumerate() {
        let r = menu_item_rect(i);
        let selected = i == menu.selected();
        let (fill, text) = match (item.enabled, selected) {
            (false, _) => (Color::new(0.10, 0.10, 0.12, 0.6), DIM),
            (true, true) => (RUST, WHITE),
            (true, false) => (Color::new(0.16, 0.16, 0.20, 0.8), BONE),
        };
        draw_rectangle(r.x, r.y, r.w, r.h, fill);
        let dims = measure_text(item.label, None, 28, 1.0);
        draw_text(
            item.label,
            r.x + (r.w - dims.width) / 2.0,
            r.y + r.h * 0.68,
            28.0,
            text,
        );
    }
    draw_text(version_label(), 12.0, h - 12.0, 20.0, DIM);
    draw_text(
        "Up/Down + Enter, or mouse. Esc quits.",
        12.0,
        h - 34.0,
        18.0,
        DIM,
    );
}

pub fn draw_briefing(mission: u8) {
    let h = screen_height();
    centered(
        &format!("Mission {mission} of {MISSION_COUNT}"),
        h * 0.25,
        28.0,
        DIM,
    );
    match story::briefing(mission) {
        Some(lines) => {
            centered(lines[0], h * 0.25 + 60.0, 48.0, RUST);
            centered(lines[1], h * 0.25 + 120.0, 24.0, BONE);
            centered(lines[2], h * 0.25 + 150.0, 24.0, BONE);
        }
        None => centered("Briefing not written yet.", h * 0.25 + 60.0, 32.0, BONE),
    }
    centered("Press Enter to launch", h * 0.80, 26.0, DIM);
}

/// Stand-in for the space and boarding scenes until T05 / T18 exist.
pub fn draw_placeholder(title: &str, hint: &str) {
    let h = screen_height();
    centered(title, h * 0.40, 48.0, RUST);
    centered(hint, h * 0.40 + 50.0, 24.0, BONE);
}

pub fn draw_debrief(mission: u8, won: bool) {
    let h = screen_height();
    let (head, color) = if won {
        ("Victory", RUST)
    } else {
        ("Defeat", DIM)
    };
    centered(&format!("Mission {mission}: {head}"), h * 0.40, 56.0, color);
    centered("Press Enter to continue", h * 0.40 + 60.0, 24.0, BONE);
}
