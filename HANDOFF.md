# HANDOFF

## State (v0.30.0)
- **T01–T29 done = M0–M3 (campaign).** T25–T29 added in v0.30.0 (see `GAME_DESIGN.md` §9). `./run.sh` launches (see its header: `fight`, `dock`, `board`, `tab N`, `sector NAME`, `viewer`, `test`, `shot`).
- **Space sim** (headless, `src/sim/`): `ops.rs` (per-ship Ops + Faction), `combat.rs` (lance/torpedo/missile, damage by facing + shield frequency, destroy/debris/bounty),
  `ai.rs` (Minion patrol/intercept/attack/flee/reinforce, wingmen + Orders), `crew.rs` (per-station AUTO), `systems.rs` (reactor budget, power/coolant, heat, repair),
  `science.rs`, `nav.rs` (autopilot, warp, dock, mining, command router), `comms.rs`, `econ.rs`, `sector.rs` (`new_game`, encounters, `transfer` for warp).
  Step order in `World::step`: controllers (AI, crew, autopilot) → apply commands → orbits → missile steering → movement/docked hold → projectile collision → upkeep → solid collisions.
- **Boarding sim** (headless, `src/board/`): `level.rs` (tile grid, generator, key/lock solver), `pmove.rs` (Quake 2 pmove port + grid collision), `weapons.rs` (6 weapons, armour),
  `monster.rs` (state machine, sight/hearing, medic), `world.rs` (`BoardWorld::step`: items, doors, alarm, objectives, rockets, outcome).
- **Audio** `src/audio/synth.rs` (headless synth) + `gfx/sound.rs` (macroquad `audio` feature, enabled in Cargo.toml).
- **GFX**: `space_scene.rs` (input → queued Commands, events → effects/sound, `boardable()` / `apply_boarding()`), `stations.rs` + `ui.rs` (F1–F7 panels → `Action`s),
  `dock_ui.rs`, `board_scene.rs` (objective screen, FPS renderer with baked vertex light, HUD), `hud.rs`. `main.rs` owns the `BoardScene`, mouse grab, volume keys, forced end-of-mission events.
- Verified this session: `cargo clippy --all-targets -- -D warnings` clean, `cargo test --release` 92 pass, `cargo fmt`; screenshots of Helm/Weapons/Engineering/Map/Dock/beam and a boarding level.
  **Not verified (needs a human at the keyboard):** sound output, mouse clicks on panels, mouse-look feel/sensitivity in boarding (0.0028 rad/px), real-time balance of space and boarding combat.
- Dev flags: `--space <sector> [--fight] [--fire] [--dock] [--tab N] [--chase] [--target]`, `--board [1-4] [--size 1-5]`, `--viewer [model]`, `--shot out.png [--frames N]`.

## Session v0.30.0 (T25-T29)
- New: `sim/mission.rs`, `story.rs` (briefings, verses, endings), `save.rs`, `gfx/keys.rs`, `gfx/help.rs`, `gfx/settings_ui.rs`; AppState gained Settings/Help/Ending; `run.sh` fixed (`default-run`, `.cargo/config.toml` targets apple-m1).
- Verified: clippy clean, `cargo test --release` 111 pass, launch screenshot. **Not verified by a human:** mission flow end-to-end, settings screen mouse/keys, key rebinding in play, HUD caption layout, boarding-forced objectives.
- HTML docs: `HTML/` (manual, player guide, developer guide, AI guide) built by `python3 tools/make_html.py`.

## Known gaps / debt (one line each)
- Dock screen mutates `Ops` directly via `econ` (multiplayer seam would need `Command::Buy…`). Warp gates not built (single system). Enemy squads respawn on re-entering a sector.
- Minion stations (Concord forts, Geneva Dock) are inert targets — no turrets, not boardable.
- Missions use stock sectors; warp gates are not built (mission 6 is a plain blockade fight). Mission balance is untested by a human: T30 bot should check solvability and tune `sim/mission.rs` numbers.
- Boarding: monsters path by BFS ignoring locks (they open unlocked doors only); no gibs/ragdoll; pickups are billboards; no flashlight; prisoners do not follow; level music/ambience not separate.
- Map is a flat X/Z projection (no altitude).

## Next
- **T30** balance pass + headless bot playtest harness (`PLAN.md` M4). Play missions 1–10 by hand first: the spawn offsets, time limit of mission 8 (420 s) and wave sizes are guesses.

## Resume
1. Read `tasks.md` → first undone task → its section in `PLAN.md`.
2. `cargo check && cargo clippy --all-targets -- -D warnings && cargo test --release && cargo fmt`.
3. Run in `--release` (debug builds are slow for the CPU-lit meshes).

## Notes
- Quake 2 reference has no art/maps; everything visual is generated. Licence: GPL-2.0-or-later (see `CREDITS.md`).
- Edition 2024 reserves `gen`; let-chains are used (clippy collapsible_if). `is_multiple_of` is preferred over `% n == 0`.
- macroquad draw-call capacity is raised in `window_conf` (60k verts). Mesh UVs stay inside 0..1 (no wrap mode), so level quads are one tile each.
- Discrete commands wait in `SpaceScene::queue` until a fixed step runs (frames with 0 steps used to drop key presses).
