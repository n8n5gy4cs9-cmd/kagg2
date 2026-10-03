# HANDOFF

## State (v0.7.0)
- **T01–T06 done.** Foundation + first playable space flight:
  - `src/texgen/` (+ `bin/texgen`): 116 generated PNGs in `assets/textures/` (+ `manifest.json`, `assets/contact_sheet.png`). Palette-ramp + Bayer-dither "pixel art" look. `font.rs` = shared 5×7 bitmap font.
  - `src/meshgen/`: 38 headless low-poly models (ships, stations, planets/asteroids/ring, boarding people/mech/drone/props). `src/assets.rs` loads textures (regenerates them if the folder is missing; magenta checkerboard fallback).
  - `src/sim/` (headless, no macroquad): `World` (generational ids), `Rng` (xoshiro), `Stepper` (60 Hz), `Command`/`Event`, `flight.rs` (throttle = target speed, flight assist, Newtonian mode), `sector.rs` (6 sectors: Mars Orbit, Phobos, Belt, Jupiter, Luna, Earth; orbits, asteroid fields, sky bodies).
  - `src/gfx/`: `render3d.rs` (320×~270 pixel render target → post shader: bloom, 15-level Bayer dither, vignette; vertex-lit models; additive glows), `space_scene.rs` (input → Commands, camera, sky, effects), `hud.rs`, `cockpit.rs`, `pixfont.rs`, `viewer.rs`.
- Verified: `cargo clippy --all-targets -- -D warnings` clean, `cargo test --release` 25 pass, `cargo fmt`, screenshots of all 6 sectors, viewer, cockpit + chase.
- Flight keys: W/S throttle, A/D yaw, ↑/↓ pitch (↓ = nose up), Q/E roll, ←/→ strafe, R/F up/down, Space brake, X zero throttle, LShift full, M assist, C camera, T target, G clear, H help, B board (placeholder), F9 win (placeholder), Esc menu.
- Dev flags: `cargo run --release -- --viewer [model]`, `--space <mars|phobos|belt|jupiter|luna|earth> [--chase] [--pos x,y,z] [--look x,y,z] [--throttle 0.8] [--target] --shot out.png [--frames 45]` (saves a screenshot and exits), env `KAGGA_FPS=1`.
- **Next: T07** weapons (lances, torpedoes, missiles, projectiles, explosions — `explosion_sheet`, `beam_*`, `torpedo`, `missile` models/textures already exist).

## Resume
1. Read `tasks.md` → first undone task → its section in `PLAN.md`.
2. `cargo check && cargo clippy --all-targets -- -D warnings && cargo test --release && cargo fmt`.
3. Run in `--release` (debug builds are slow for the CPU-lit meshes).

## Notes
- Quake 2 reference has no art/maps; everything visual is generated.
- Licence implication: GPL-2.0-or-later (see `CREDITS.md`).
- Edition 2024 reserves `gen` (entity id field is `generation`); let-chains are used (clippy collapsible_if).
- macroquad draw-call capacity is raised in `window_conf` (60k verts) — big meshes need it.
- Textures have no wrap mode in safe macroquad, so mesh UVs stay inside 0..1 (quads pick a random sub-window of tileable textures).
- Out-of-scope ideas noted: mouse flight, projectile/beam rendering (T07), crew-AI auto-throttle (T10).
