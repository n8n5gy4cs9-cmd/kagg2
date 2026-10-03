//! `cargo run --bin texgen [dir]` — writes every generated texture to `assets/textures/`.

use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    let dir = std::env::args()
        .nth(1)
        .map_or_else(|| PathBuf::from("assets/textures"), PathBuf::from);
    match kagga2::texgen::write_all(&dir) {
        Ok(rows) => {
            println!("{:<24} {:>9}  mean rgb", "name", "size");
            for r in &rows {
                let m = r.mean;
                println!(
                    "{:<24} {:>9}  {},{},{}",
                    r.name,
                    format!("{}x{}", r.w, r.h),
                    m[0],
                    m[1],
                    m[2]
                );
            }
            println!("{} textures -> {}", rows.len(), dir.display());
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("texgen failed: {e}");
            ExitCode::FAILURE
        }
    }
}
