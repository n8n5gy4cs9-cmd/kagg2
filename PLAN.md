# Kagga2 — Detailed Plan

> **Pitch:** Mars Vikings vs. the Earth Concord's Minions. You captain the longship *Huginn*:
> fly and fight in a simplified Space-Nerds-in-Space bridge sim, then storm the enemy ships and
> bases in Quake 2 style first-person boarding actions. Single player first; multiplayer later.

## 1. Pillars (what "fun but simple" means here)

1. **One captain, a whole bridge.** SNIS needs 6+ humans. Kagga2 gives you every station in one
   UI (tabs F1–F7) and an AI **crew** that runs the stations you are not on. Take over any
   station for better results. (This is also the multiplayer seam: later a human takes the seat.)
2. **Few systems, each with a lever.** 6 ship systems, 3 weapons, 1 shield mechanic, 1 power
   budget. Every lever visibly changes the fight.
3. **Space → board → space.** Wear an enemy down, clamp on, fight through the hull in first
   person, loot/capture/destroy, return. Two games feeding each other.
4. **Short missions, saga tone.** 10 missions, 10–20 min each, skippable briefings, Norse voice.
5. **Everything generated.** All textures, meshes and sounds are made by our own Rust code.
   No external art, no licence baggage from assets.

## 2. Tech decisions

| Topic | Decision | Why |
|---|---|---|
| Language | Rust, edition 2021/2024 per `cargo new`, stable toolchain | requested |
| Engine | `macroquad` (window, 2D/3D draw, input, audio, textures) | tiny, no ECS boilerplate, quick compile, 3D meshes + textures supported |
| Math | `glam` (same version macroquad re-exports) | headless sim without a window |
| Texture output | `png` crate in the `texgen` bin | deterministic PNG writing |
| Layout | one crate: `lib` (sim, headless, tested) + `bin` (game) + `bin/texgen` | sim testable without GPU; multiplayer-ready |
| Sim | fixed 60 Hz step, seeded RNG, input = `Command` structs | determinism, saves, replays, netcode later |
| Errors | `thiserror`-style typed errors by hand (no unwrap in game paths) | repo rules in `AGENTS.md` |
| Licence | **GPL-2.0-or-later** | we port logic from GPL v2 sources (SNIS, Quake 2) — see `CREDITS.md` |

Dependencies are added only when a task needs them (repo rule): T01 adds `macroquad`;
T02 adds `png`; nothing else is planned before T32 (networking: std `TcpStream`/`UdpSocket` first).

## 3. What we take from each reference (details: `REFERENCE_MAP.md`)

**Space-Nerds-in-Space (bridge, ships, economy)** — port the *ideas and data*, not the C code:
ship classes (`ship_types.txt`), power/coolant model (`power-model.c`), shields + weapon
wavelength weakness (`shield_strength.c`), commodities & trade (`commodities.c`), factions
(`snis_faction.c`), warp gates / solar systems, docking ports, damage control, comms/taunts,
mining. Cut: voice chat, DMX/Arduino, lobby/multiverse, Lua, RTS mode, 14 other solar systems.

**Quake 2 (boarding)** — port *gameplay logic*: player movement (`pmove`), weapons
(`p_weapon.c`), monster state machine (`g_ai.c`, `m_*.c`), items (`g_items.c`), doors/buttons/
triggers (`g_func.c`, `g_trigger.c`), health/armor/damage (`g_combat.c`). The repo has **no
art or maps** (only `baseq2/config.cfg`), so levels are procedural and textures are generated.
Cut: BSP, software renderer, network protocol, CTF, 20 of 25 monster types.

## 4. Milestones & task detail

Acceptance = what must be true to tick the task. Every task also needs: `cargo check`,
`cargo clippy -- -D warnings`, `cargo test`, `cargo fmt` clean (see root `AGENTS.md`).

### M0 — Foundation
- **T01 Project skeleton.** `Cargo.toml` (name `kagga2`, version `0.1.0`), `src/lib.rs` (+
  `version_label()`), `src/main.rs`, app state machine (`Boot → MainMenu → Briefing → Space |
  Boarding → Debrief`), main menu (New Game / Continue(disabled) / Settings(disabled) / Quit),
  animated starfield, title + story blurb, window 1280×720 resizable. *Accept:* game window
  opens, menu navigable by keyboard/mouse, Esc quits, unit tests for state transitions + version.
- **T02 Texture generator.** `src/bin/texgen.rs` + `src/texgen/` (value/fbm noise, voronoi,
  panels, rust, runes, stripes). Writes every texture in `ASSETS.md` to `assets/textures/` with a
  `manifest.json`. Seeded → byte-identical reruns. *Accept:* `cargo run --bin texgen` produces all
  files; test checks count, sizes, determinism hash; contact-sheet PNG for eyeballing.
- **T03 Assets + meshes.** `assets.rs` loader (textures by name, fallback checkerboard);
  `meshgen/` builds low-poly meshes in code: longship, skiff, drakkar, hauler, forge station,
  Minion interceptor/cutter/frigate/dreadnought/barge, planets (UV spheres), asteroids
  (displaced icosphere), boarding characters (boxy), props. *Accept:* a "model viewer" debug
  state shows every mesh textured; tests check non-degenerate meshes.
- **T04 Sim core.** `sim/` : `World` (entity arena with generational ids), components as plain
  structs, `step(dt, &[Command])`, `Rng` (xoshiro in-house), `Event` queue, `Command` enum
  (Throttle, Turn, FireLance, …). *Accept:* headless test runs 10 000 steps deterministically
  (same seed → same hash).

### M1 — Space (SNIS-lite)
- **T05 Flight.** Newtonian-lite: thrust, strafe, pitch/yaw/roll with damping, speed cap, "match
  velocity" brake, cockpit + chase cameras, HUD (speed, heading, target brackets, crosshair).
  Keys: W/S throttle, A/D yaw, Q/E roll, ↑/↓ pitch, Space brake, mouse optional.
- **T06 Sol sector.** Scale-model sector (units compressed): Sun, Mercury…Neptune as scenery,
  playable sectors: *Mars Orbit, Phobos/Deimos, Belt, Jupiter/Ganymede, Luna, Earth Orbit*.
  Skybox (stars+nebula), planet textures, asteroid fields, stations. Sector loader from Rust data.
- **T07 Weapons.** Rune lance (hitscan beam, power-gated, cooldown by heat), Thunder Torpedo
  (slow, big damage, 8 max), Gungnir missile (homing, needs lock, 12 max). Projectile entities,
  collision, explosion particles/sprites, ammo.
- **T08 Shields & damage.** Shield ring per facing (fore/aft/port/starboard), hull points,
  per-system damage on hull hits, destruction + debris. **Frequency mechanic:** each shield has
  freq 0–9; lance set to the same freq (±1) does ×2 shield damage, off by ≥4 does ×0.5.
- **T09 Minion AI.** Behaviour tree-lite: Patrol → Intercept → Attack (circle-strafe / stand-off
  by class) → Flee at <25 % hull → Call reinforcements. Squads/fleets with a leader. Difficulty
  scales accuracy/reaction. Wingmen (Mars skiffs) obey orders from T15.
- **T10 Station UI.** Tab bar F1 Helm, F2 Weapons, F3 Engineering, F4 Damage Control, F5 Science,
  F6 Comms, F7 Map. Each station has a **crew AI** (`crew.rs`) with a skill level, performing
  its default behaviour (auto-aim assist off by default, auto-distribute power, auto-repair,
  auto-scan nearest threat). Player toggles AUTO/MANUAL per station.
- **T11 Engineering.** Reactor output budget; sliders for power to Engines, Warp, Shields,
  Lances, Sensors, Life Support; coolant allocation; heat → overheating damages the system.
  Port the idea of `power-model.c`. Presets 1–3 (Balanced / Battle / Run).
- **T12 Damage control.** Each system has health 0–100 %; <50 % degrades performance, 0 % off.
  Repair crew (3 teams) assigned to systems by priority; crew can die if life support fails.
- **T13 Science.** Scan target (hold scan N seconds) → class, hull, shield freq, weakness hint,
  cargo, faction, boarding difficulty. Long-range sensor sweep shows contacts on map.
- **T14 Nav/Map.** 2D sector map with contacts, waypoints, course plot, autopilot ("fly to X"),
  warp drive (fuel, charge-up, jump between sectors), warp gates between systems.
- **T15 Comms.** Hail dialog trees (friend/neutral/enemy), orders to wingmen (attack/guard/
  follow/retreat), docking requests, taunt generator (small grammar from `infinite-taunt.c`),
  mission message log.
- **T16 Starbases & economy.** Docking sequence at a port, refuel/repair/rearm, shipyard
  upgrades (lance, shields, reactor, hull, torpedo bays), buy/sell ~12 commodities with price
  volatility per station, mine asteroids with a mining bot, sell ore. Mars **Forge** station
  is the home base.
- **T17 Audio.** `audio/synth.rs` renders WAV in memory: laser, torpedo, missile, explosion,
  shield hit, hull creak, alarm, docking clamp, UI beeps, footsteps, gunshots, doors; ambient
  drone + a simple generated music loop (minor-key lur/drone). Volume sliders.

### M2 — Boarding (Quake-2-lite)
- **T18 Level generator.** Tile/room graph on a grid, extruded to 3D (walls, floors, ceilings).
  Themes: *Minion corvette*, *Minion station/base*, *Prison barge*, *Dreadnought bridge*.
  Rooms: airlock (spawn), corridors, barracks, armoury, reactor, bridge/objective room, vents.
  Seeded; guarantees solvable key/door order. Level size scales with enemy ship class.
- **T19 FPS renderer.** Batch wall/floor/ceiling quads per texture, simple per-room light level,
  flicker lights, fog, billboard sprites for pickups/effects, boxy 3D monsters, first-person
  viewmodels, HUD (health, armor, ammo, keys, objective, compass), damage flash.
- **T20 Player controller.** Port `pmove`: ground accel/friction/stop speed, air control,
  jump, crouch, low-gravity option for zero-g section, grid-AABB collision with sliding.
- **T21 Weapons & items.** Axe (melee), Blaster (infinite), Scatter-gun (shells ×12 pellets),
  Rune Rattler (machinegun), Mjolnir launcher (rocket, splash), Valkyrie Lance (rail). Damage
  numbers from `p_weapon.c`. Items: health (small/large), armor shard/jacket/plate, ammo types,
  keycards (Bronze/Silver/Gold), data chips. Armor absorption from `g_combat.c`.
- **T22 Minions (monsters).** Port the Quake 2 AI loop (stand → walk → run → attack → pain →
  die, sight/hearing, melee vs ranged choice, dodge). Roster with Quake 2 health:
  Grunt (20–40), Guard (100), Gunner (175), Medic (300, revives), Enforcer (240, melee),
  Mech (750 mini-boss), Prefect Voss (1500 boss, from supertank). Gibs/ragdoll-lite.
- **T23 Level logic.** Doors (open/close, locked by key), buttons, pressure triggers, alarm
  (spawns reinforcements), objectives: **Sabotage reactor, Capture bridge, Rescue prisoners,
  Steal data core, Kill boss**. Exit via airlock; timer on some objectives (self-destruct).
- **T24 Transition.** From space: target hull <60 % *and* shields down, within clamp range,
  press **B**, choose objective → boarding level; outcome returns to space with loot/captured
  ship/destroyed ship + crew casualties.

### M3 — Campaign
- **T25 Mission engine.** `missions.rs`: mission = briefing + start state + objective list +
  triggers (enter area, kill N, scan X, dock, board) + dialogue lines + success/fail.
  Data in Rust structs first (file format is T34).
- **T26 Missions 1–5**, **T27 Missions 6–10** (see `STORY.md`). Each mission playtested by the
  T30 bot for solvability.
- **T28 Save/load & settings.** Simple versioned text save (campaign progress, ship upgrades,
  credits, mission state at mission start). Settings: volume, mouse sensitivity, invert-Y,
  difficulty (Thrall/Karl/Jarl), key bindings (basic rebinding).
- **T29 Tutorial/help.** Mission 1 doubles as tutorial; F-key help overlays per station
  (modelled on SNIS `help_text_*`); boarding controls card.

### M4 — Polish
- **T30 Balance + bot.** Headless bot that flies/ shoots/ boards on all missions at 3 difficulties;
  report win rates, TTK, resource flow; tune numbers in one `balance.rs`.
- **T31 Release.** Profile perf (target 60 fps on a 2020 laptop), release builds, final docs,
  licence/credits audit, remove `REFERENCE_tobe_DELETED/` (only when user says so).

### M5 — Optional / future (design kept compatible, not scheduled)
- **T32 Multiplayer.** Server-authoritative sim (`step` already takes `Command`s per player),
  snapshots, one human per station (crew-AI yields the seat), LAN lobby. Std sockets first.
- **T33 Co-op boarding**, **T34 Mission scripting format**.

## 5. Risks

| Risk | Mitigation |
|---|---|
| Scope creep (two games in one) | Fixed 10-mission cap; every feature has a "simple version" above; M5 is optional |
| Procedural textures look flat | Layered noise + wear/decals, contact sheet review in T02, art-direction palette in `ASSETS.md` |
| Boarding levels dull/unsolvable | Seeded generator + solvability test + bot (T30) |
| One-person bridge overload | Crew AI per station, pause-able orders (Tab = tactical pause) |
| GPL obligations | Whole project GPL-2.0-or-later, credits in `CREDITS.md` |
| macroquad 3D limits (lighting) | Bake light into vertex colours; fog; fall back to custom mesh batches |

## 6. Definition of done (every task)

1. Code compiles, clippy `-D warnings`, tests pass, `cargo fmt`.
2. No `unwrap`/`expect` in non-test code; no `unsafe`.
3. Matching `.md` docs updated; `tasks.md` line moved to Done; `HANDOFF.md` session note added.
4. One-line commit message offered (`vX.Y.Z: …`), version bumped in `Cargo.toml`
   (minor = feature/task, patch = fix).
