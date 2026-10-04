//! Every tunable number the T30 playtest bot adjusts, in one place. Change a value here, run
//! `cargo run --release --bin playtest` (and `... playtest 10 boarding`) and compare win rates.

/// Enemy scale per difficulty (`World::difficulty` / `BoardWorld::difficulty`): Thrall, Karl, Jarl.
pub const DIFFICULTY_SCALE: [f32; 3] = [0.7, 1.0, 1.4];

/// Boarding party's starting armour points.
pub const BOARD_START_ARMOR: u32 = 40;
/// Shells the party starts with for the scatter-gun it carries from the longship's locker.
pub const BOARD_START_SHELLS: u32 = 20;
/// Chance per room, per size step above 1, of one extra monster (bigger ships are fuller).
pub const BOARD_DENSITY_STEP: f32 = 0.2;
/// Multiplier on all monster damage to the boarding party.
pub const BOARD_MONSTER_DAMAGE: f32 = 0.25;
/// Extra pickups per room on top of the generator's `1..=2`.
pub const BOARD_EXTRA_ITEMS: u32 = 1;
/// Shortest sabotage fuse in seconds (the real fuse grows with the way back to the airlock).
pub const BOARD_FUSE_MIN: f32 = 45.0;
/// Metres at which a monster hears gunfire and wakes up.
pub const BOARD_HEARING: f32 = 22.0;
/// How strongly the difficulty scale changes a monster's hit chance (`1.0` = fully, `0.0` = not).
pub const BOARD_HIT_SCALE: f32 = 0.5;

/// Hit chance multiplier for monsters at difficulty `d` (damage scales with `d` itself).
pub fn hit_scale(d: f32) -> f32 {
    1.0 + (d - 1.0) * BOARD_HIT_SCALE
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn karl_is_the_neutral_difficulty() {
        assert_eq!(DIFFICULTY_SCALE[1], 1.0);
        assert_eq!(hit_scale(1.0), 1.0);
        assert!(hit_scale(1.4) > 1.0 && hit_scale(1.4) < 1.4);
    }
}
