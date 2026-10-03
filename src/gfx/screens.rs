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
        h * 0.18,
        28.0,
        DIM,
    );
    match story::briefing(mission) {
        Some(b) => {
            centered(b.title, h * 0.18 + 60.0, 52.0, RUST);
            centered(b.place, h * 0.18 + 92.0, 24.0, DIM);
            for (i, line) in b.lines.iter().enumerate() {
                centered(line, h * 0.18 + 150.0 + i as f32 * 32.0, 26.0, BONE);
            }
        }
        None => centered("Briefing not written yet.", h * 0.18 + 60.0, 32.0, BONE),
    }
    centered(
        "Press Enter to launch  -  Esc for the menu",
        h * 0.88,
        24.0,
        DIM,
    );
}

/// Debrief: result, saga verse and (after a loss) why.
pub fn draw_debrief(mission: u8, won: bool, reason: &str) {
    let h = screen_height();
    let (head, color) = if won {
        ("Victory", RUST)
    } else {
        ("Defeat", DIM)
    };
    centered(&format!("Mission {mission}: {head}"), h * 0.34, 56.0, color);
    if !won && !reason.is_empty() {
        centered(reason, h * 0.34 + 44.0, 24.0, BONE);
    }
    centered(story::verse(mission, won), h * 0.34 + 110.0, 28.0, BONE);
    centered("Press Enter to continue", h * 0.80, 24.0, DIM);
}

/// Ending choice (`chosen == 0`) or epilogue for the chosen ending.
pub fn draw_ending(chosen: u8) {
    let h = screen_height();
    match story::Ending::from_code(chosen) {
        None => {
            centered(
                "Prefect Voss is dead. The fleet codes are yours.",
                h * 0.28,
                34.0,
                RUST,
            );
            centered(
                "1  Break the Armada - destroy the codes, make an uneasy peace",
                h * 0.45,
                28.0,
                BONE,
            );
            centered(
                "2  Take the Throne - use the codes, Mars rules",
                h * 0.45 + 50.0,
                28.0,
                BONE,
            );
            centered("Press 1 or 2", h * 0.85, 24.0, DIM);
        }
        Some(e) => {
            centered("EPILOGUE", h * 0.22, 48.0, RUST);
            for (i, line) in e.epilogue().iter().enumerate() {
                centered(line, h * 0.36 + i as f32 * 34.0, 26.0, BONE);
            }
            centered(
                "Kagg2 - generated art, procedural sound. Thank you for playing.",
                h * 0.78,
                22.0,
                DIM,
            );
            centered("Press Enter for the menu", h * 0.86, 24.0, DIM);
        }
    }
}

/// Main-menu Help screen: one page of `help::pages()`.
pub fn draw_help(page: usize) {
    let pages = super::help::pages();
    let h = screen_height();
    let p = &pages[page.min(pages.len() - 1)];
    centered("HELP", h * 0.12, 44.0, RUST);
    centered(p.title, h * 0.12 + 50.0, 30.0, BONE);
    for (i, line) in p.lines.iter().enumerate() {
        centered(line, h * 0.34 + i as f32 * 30.0, 24.0, BONE);
    }
    centered(
        &format!(
            "Page {}/{}  -  Left/Right to turn, Esc to return",
            page + 1,
            pages.len()
        ),
        h * 0.9,
        22.0,
        DIM,
    );
}
