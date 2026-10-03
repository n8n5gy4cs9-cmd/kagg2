//! Kagga2 — Mars Vikings vs. Earth Minions.

pub mod app;
pub mod assets;
pub mod gfx;
pub mod meshgen;
pub mod sim;
pub mod story;
pub mod texgen;

/// Game version from `Cargo.toml`, e.g. `0.1.0`.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Label shown on the main menu, e.g. `Kagga2 0.1.0`.
pub fn version_label() -> String {
    format!("Kagga2 {VERSION}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_label_contains_cargo_version() {
        assert_eq!(
            version_label(),
            format!("Kagga2 {}", env!("CARGO_PKG_VERSION"))
        );
    }
}
