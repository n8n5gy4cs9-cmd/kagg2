//! Boarding actions (M2): level generator, player movement, weapons, monsters, level logic.
//! Headless — never imports `macroquad` (rendering lives in `gfx::board_scene`).

pub mod bot;
pub mod level;
pub mod monster;
pub mod pmove;
pub mod weapons;
pub mod world;

pub use level::{Level, Objective, Theme, generate};
pub use pmove::Input;
pub use weapons::Weapon;
pub use world::{BEvent, BoardWorld, Outcome};

/// Theme and size (`1..=5`) of the level generated when boarding the ship `label` of `class_name`.
pub fn theme_for(label: &str, class_name: &str, seed: u64) -> (Theme, u32) {
    match (label, class_name) {
        (l, _) if l.starts_with("Prison Barge") => (Theme::PrisonBarge, 3),
        (l, _) if l.starts_with("Courier") => (Theme::Corvette, 2),
        // the campaign's capital ships grow from mission 8 to 10 (T30 balance)
        (l, _) if l.starts_with("Frost Command") => (Theme::Station, 3),
        (l, _) if l.starts_with("Bulwark") => (Theme::DreadnoughtBridge, 4),
        (_, "Interceptor") => (Theme::Corvette, 1),
        (_, "Cutter") if seed.is_multiple_of(2) => (Theme::PrisonBarge, 2),
        (_, "Cutter") => (Theme::Corvette, 2),
        (_, "Frigate") => (Theme::Station, 3),
        (_, "Destroyer") => (Theme::Station, 4),
        _ => (Theme::DreadnoughtBridge, 5),
    }
}
