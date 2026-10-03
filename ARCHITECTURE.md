# Kagga2 — Architecture

## Crate layout (single crate, lib + bins)

```
Kagga2/
├── Cargo.toml                 # name kagga2; version = single source of truth
├── src/
│   ├── lib.rs / main.rs       # main = macroquad loop, CLI dev flags (--viewer/--space/--shot)
│   ├── app.rs                 # AppState machine (pure; testable)                     [T01]
│   ├── assets.rs              # texture registry + fallback + model lookup             [T03]
│   ├── texgen/                # noise, image, space/ships/board/chars/ui recipes, font [T02]
│   ├── meshgen/               # Builder + ships/bodies/board model catalogue           [T03]
│   ├── sim/                   # HEADLESS — no macroquad                                [T04-T06]
│   │   ├── world.rs           # generational entity arena, step(cmds) -> events, orbits, collisions
│   │   ├── rng.rs clock.rs command.rs event.rs
│   │   ├── flight.rs          # ShipClass table, throttle/assist/Newtonian integrate
│   │   └── sector.rs          # Sol sectors, sky bodies, new_game()
│   ├── gfx/                   # macroquad-only
│   │   ├── render3d.rs        # pixel render target, post shader, lit models, billboards
│   │   ├── space_scene.rs     # input -> Commands, camera, sky/world/effects draw
│   │   ├── hud.rs cockpit.rs pixfont.rs viewer.rs screens.rs starfield.rs
│   └── bin/texgen.rs          # writes assets/textures/*.png
├── assets/textures/*.png      # generated, committed (+ manifest.json, ../contact_sheet.png)
└── docs…
```

## Principles

1. **Sim never touches the GPU.** `sim::*` takes plain data (`glam::Vec3`, structs) and is unit
   tested. `gfx::*` reads the world and draws.
2. **Fixed timestep (60 Hz)** accumulator in `main.rs`; render interpolates.
3. **Input → `Command`.** The UI translates keys/mouse/crew-AI decisions into `Command`s each
   tick. Replays, bots (T30) and netcode (T32) use the same path.
4. **Determinism.** One seeded `Rng` in `World`; no `HashMap` iteration order dependence in the
   sim (use `Vec`/`BTreeMap`); no wall-clock in the sim.
5. **Two worlds, one trait.** `SpaceWorld` and `BoardWorld` both implement
   `trait Scene { fn step(&mut self, dt: f32, cmds: &[Command]) -> Vec<Event>; }`. `App` owns the
   active scene and swaps it on `Event::BeginBoarding` / `Event::EndBoarding`.
6. **Data in code first** (`const` tables for ships, weapons, commodities, missions). External
   file formats only in T34.
7. **Errors:** typed enums (`AssetError`, `SaveError`), `?` everywhere, no `unwrap` outside tests.

## App state machine (T01)

```
Boot ─▶ MainMenu ─▶ Briefing(mission) ─▶ Space(mission) ⇄ Boarding(level)
            ▲                                │
            └────────── Debrief ◀────────────┘
Any ─ Esc ─▶ PauseMenu ─▶ MainMenu / Resume
```
`app::AppState` + `app::Transition` are pure data so transitions are unit tested.

## Rendering (as built, T05)
The 3D scene renders into a ~270 px-high **render target** (integer upscale of the window), drawn with
nearest filtering through a post shader (bloom, Bayer dither to 15 levels, vignette) → pixel-art
look; HUD/text are drawn afterwards at full resolution. Models are lit on the CPU (warm sun,
cool ambient, cel bands); glows/dust use an additive material. Sky = nebula sphere + star quads
+ distant sun/planets at 250 000 units (camera far plane 400 000).

## Rendering plan (macroquad, original)
- Space: `set_camera(&Camera3D)`, `draw_mesh` for ships/planets, skybox as a big inverted cube
  with generated textures, lasers as camera-facing quads, HUD in screen space.
- Boarding: level is baked into one `Mesh` per texture (vertex colours hold baked light),
  monsters/props drawn as small boxy meshes, pickups as billboards, viewmodel as 2D overlay
  quads or a small mesh with its own camera.

## Multiplayer seam (T32, not now)
`Seat { station, controller: Controller::{CrewAi, Human(PlayerId)} }`. Server runs `step` with
commands from all humans + crew AI; clients render snapshots. Nothing in M0–M4 may assume a
single human except the `Tab` pause.

## Testing strategy
- Unit tests per module (state machine, flight integration, damage math, level solvability).
- Determinism test: same seed + same command stream → identical world hash.
- Bot playtest (T30): headless runs of every mission.
- `cargo test` must pass without a display.
