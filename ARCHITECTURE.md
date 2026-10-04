# Kagg2 — Architecture

## Crate layout (single crate, lib + bins)

```
Kagg2/
├── Cargo.toml                 # name kagg2; version = single source of truth
├── src/
│   ├── lib.rs / main.rs       # main = macroquad loop, CLI dev flags (--viewer/--space/--shot)
│   ├── app.rs                 # AppState machine (pure; testable)                     [T01]
│   ├── assets.rs              # texture registry + fallback + model lookup             [T03]
│   ├── texgen/                # noise, image, space/ships/board/chars/ui recipes, font [T02]
│   ├── meshgen/               # Builder + ships/bodies/board model catalogue           [T03]
│   ├── sim/                   # HEADLESS — no macroquad                                [T04-T16]
│   │   ├── world.rs           # entity arena; step(): AI+crew+autopilot cmds -> apply -> move -> combat -> upkeep
│   │   ├── rng.rs clock.rs command.rs event.rs
│   │   ├── flight.rs          # ShipClass table (Viking + Minion classes), integrate (engine perf scales it)
│   │   ├── sector.rs          # Sol sectors, encounters, new_game(), transfer() for warp
│   │   ├── ops.rs             # per-ship Ops: hull/shields/ammo/systems/nav/scan/cargo/upgrades, Faction
│   │   ├── combat.rs          # lances, torpedoes, missiles, damage (facing + frequency), destroy, upkeep  [T07/T08]
│   │   ├── ai.rs              # Minion AI + wingmen, Order                                               [T09]
│   │   ├── crew.rs            # crew-AI per station (AUTO toggles)                                       [T10]
│   │   ├── systems.rs         # reactor budget, power/coolant, heat, health, repair teams                [T11/T12]
│   │   ├── mission.rs         # campaign engine: Mission/Stage/Goal/Beat data, MissionState, Carry                   [T25-T27]
│   │   ├── science.rs nav.rs  # scan/sweep/report; autopilot, warp, docking, mining, command router      [T13/T14/T16]
│   │   ├── comms.rs econ.rs   # hail dialogs + taunts; prices, trade, services, shipyard                 [T15/T16]
│   │   ├── bot.rs boarding.rs # headless bridge bot (flies/fights/docks/mines/scans/boards); space-side boarding aftermath      [T30]
│   │   ├── script.rs          # `.k2m` mission scripts -> Mission data + registry (ids 11+)                              [T34]
│   ├── board/                 # HEADLESS boarding actions: level.rs (generator) pmove.rs weapons.rs monster.rs world.rs  [T18-T24]
│   │   └── bot.rs             # boarding bot (lock-aware pathing, fights, loot, party play); world.rs also runs co-op parties [T30/T33]
│   ├── balance.rs             # every tunable the playtest adjusts (difficulty scale, boarding kit/density/damage/fuse) [T30]
│   ├── playtest.rs            # headless harness: bots play missions, survey tables, bench                               [T30/T31]
│   ├── net/                   # HEADLESS multiplayer: wire.rs proto.rs server.rs client.rs (std TcpStream)                [T32]
│   ├── story.rs save.rs       # story text/verses/endings; settings + save files (text, versioned)        [T25-T28]
│   ├── audio/                 # HEADLESS synth: SFX + music -> PCM/WAV                                   [T17]
│   ├── gfx/                   # macroquad-only
│   │   ├── render3d.rs        # pixel render target, post shader, lit models, billboards
│   │   ├── space_scene.rs     # input -> Commands (queued until a sim step runs), events -> effects/sound, draw
│   │   ├── stations.rs ui.rs  # F1–F7 panels, immediate-mode widgets; panels emit Actions, never touch the world
│   │   ├── board_scene.rs     # first-person renderer + objective screen + HUD; BoardInfo/BoardResult (T19/T24)
│   │   ├── dock_ui.rs sound.rs # docked screen (mutates Ops via econ), playback Mixer
│   │   ├── keys.rs help.rs settings_ui.rs  # rebindable flight keys, help text, settings screen          [T28/T29]
│   │   ├── hud.rs cockpit.rs pixfont.rs viewer.rs screens.rs starfield.rs
│   └── bin/                   # texgen (textures), playtest (bot reports), server + netclient (T32)
├── assets/textures/*.png      # generated, committed (+ manifest.json, ../contact_sheet.png)
└── docs…
```

## Principles

1. **Sim never touches the GPU.** `sim::*` takes plain data (`glam::Vec3`, structs) and is unit
   tested. `gfx::*` reads the world and draws.
2. **Fixed timestep (60 Hz)** accumulator in `main.rs`; render interpolates.
3. **Input → `Command`.** The UI translates keys/mouse/crew-AI decisions into `Command`s each
   tick. Bots (`sim::bot`, T30) and netcode (`net`, T32) use the same path.
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

## Multiplayer (T32, as built)
`net::server::Server` owns a `World` and a `MissionState`. Clients (`net::client::Client`) connect
over TCP (`std::net`, non-blocking, length-prefixed binary frames, protocol version in `Hello`),
take stations (`net::station_of(&Command)` says which station owns a command), press ready, and
send only `Command`s. The server validates the seat, steps the sim at 60 Hz (crew AI keeps every
automatable station nobody holds), and broadcasts a `Snapshot` (nearest 160 entities + the
flagship's vitals + the mission objective) at 20 Hz. Everything read from the network is bounds
checked (`wire.rs`): frames over 256 KiB, NaN floats, unknown tags and trailing bytes drop the
client. No encryption or authentication: LAN only. **Not done:** a graphical client (the game
UI still renders its own local `World`), boarding and the dock screen over the network (they
mutate state outside `Command`s), reconnects.

## Co-op boarding (T33, as built)
`BoardWorld.mates` holds up to 3 extra `Player`s. `step_team(dt, &[Input])` runs each member's
movement/fire/use/pickup by swapping them into `self.player` (`with_player`), so single-player
code paths are unchanged. Doors, keys (shared), objectives and the alarm are shared; each monster
hunts the nearest living member and hears the nearest noise; the airlock needs the whole living
party; defeat is everybody down. In the game the partners are AI (`board::bot::BoardBot`, press
**P** on the objective screen, or `--board N --coop 2`); the same API takes human inputs.

## Mission scripts (T34, as built)
`sim::script::parse` compiles the `.k2m` text format to the campaign's own `Mission` data
(strings are leaked once so the engine's `&'static` types stay unchanged); `register` hands out
ids from 11; `mission::def` and `SectorId::for_mission` look scripts up. The game loads
`$KAGG2_HOME/missions/*.k2m` at start-up and `--script FILE` plays one directly.

## Testing strategy
- Unit tests per module (state machine, flight integration, damage math, level solvability).
- Determinism test: same seed + same command stream → identical world hash.
- Bot playtest (T30): `cargo run --release --bin playtest` flies and boards every mission at three
  difficulties and prints win rates; `playtest bench` times the sim (T31).
- `cargo test` must pass without a display.
