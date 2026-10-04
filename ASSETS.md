# Kagg2 — Assets (all generated)

Nothing is copied from the references. Textures: `cargo run --bin texgen` (T02) → `assets/textures/`.
Meshes: built in code (T03). Sounds: synthesised at start-up (T17).
All generators are **seeded and deterministic**.

## Art direction
- **Marsgard:** rust red `#b5482a`, iron `#4a4d52`, bone `#d8cdb4`, ember orange `#ff8a2b`;
  riveted plates, rune carvings, wood-and-iron, hammered metal.
- **Concord Minions:** white `#e8eef2`, cyan `#35c8e0`, graphite `#2a2f36`; clean panels, hex
  vents, hazard stripes, glowing cyan strips.
- Texture sizes: 256² for tiles, 512² for planets/hulls, 1024×512 for sky/nebula, 64² for pickups.
- Format PNG RGBA8, tileable where marked **T**.

## Texture list (116 generated, T02 ✅)

### Space — environment
| Name | Size | Recipe |
|---|---|---|
| `sky_stars` | 1024×512 | 3 star layers (density, brightness, slight colour), equirect |
| `sky_nebula` | 1024×512 | fbm noise → violet/teal palette, alpha blend |
| `sun` | 512² | turbulence + granulation, emissive |
| `mercury`, `venus`, `jupiter`, `saturn`, `uranus`, `neptune` | 512² | scenery bands/craters |
| `mars` | 512² | rust fbm + canyon + polar caps |
| `earth` | 512² | ocean/continent noise + clouds |
| `luna` | 512² | grey + craters (Voronoi rims) |
| `phobos`, `deimos` | 256² | lumpy grey/brown, craters |
| `asteroid_a/b/c` | 256² **T** | rock fbm + ore flecks (c: ice) |
| `ring_saturn` | 512×64 | radial banding |
| `glow_soft`, `glow_engine` | 128² | radial gradient sprites |
| `explosion_sheet` | 512² (4×4) | 16 frames fireball |
| `beam_lance`, `beam_core` | 64×16 | beam end-caps |
| `torpedo`, `missile_trail`, `spark`, `shield_hit` | 64² | sprites |
| `shield_bubble` | 256² | hex grid, additive |

### Space — ships & stations
| Name | Size | Notes |
|---|---|---|
| `hull_viking_a/b` | 512² **T** | red-iron plates, rivets, rune strips |
| `hull_viking_trim` | 256² **T** | bone + brass |
| `sail_viking` | 256² | solar-sail banner with raven rune |
| `hull_minion_a/b` | 512² **T** | white panels, cyan lines, hex vents |
| `hull_minion_dark` | 256² **T** | graphite, hazard stripes |
| `engine_glow` | 64² | additive |
| `station_forge` | 512² **T** | industrial iron, furnace glow windows |
| `station_fort` | 512² **T** | minion armour + turret decals |
| `cargo_container_a/b` | 128² | stencilled crates (red / ochre) |
| `window_lights` | 128² **T** | tiny lit windows |

### Boarding — world
| Name | Size | Notes |
|---|---|---|
| `wall_minion_a/b/c` | 256² **T** | clean panel, conduit panel, vent panel |
| `wall_viking_a/b` | 256² **T** | wood + iron, prison cell iron |
| `wall_reactor` | 256² **T** | glowing tubes |
| `floor_grate`, `floor_tile`, `floor_hazard` | 256² **T** | metal grate, white tile, stripes |
| `ceiling_a/b` | 256² **T** | panels with light slits |
| `door_minion`, `door_blast`, `door_cell` | 128×256 | sliding doors |
| `door_lock_bronze/silver/gold` | 128×256 | door with coloured keycard light |
| `console_a/b`, `terminal_screen` | 128² | consoles, scrolling text screen |
| `reactor_core` | 256² | pulsing cyan core |
| `crate_a/b`, `barrel` | 128² | props |
| `light_panel`, `light_red` | 64² | emissive lights |
| `decal_scorch`, `decal_sparks`, `decal_oil` | 64² alpha | wall damage |
| `glass_cracked` | 128² alpha | window |
| `bridge_viewscreen` | 256×128 | planet-over-space picture |

### Boarding — characters / items / weapons
| Name | Size | Notes |
|---|---|---|
| `minion_armor`, `minion_visor` | 64² | white/cyan, one texture per body part |
| `minion_heavy`, `minion_medic`, `minion_enforcer`, `minion_mech`, `prefect_voss` | 128² | colour variants |
| `drone_body` | 64² | flyer |
| `marine_*` | 64² | allied marines (T33 co-op / cutscenes) |
| `pickup_health_s/l`, `pickup_armor_*`, `pickup_ammo_*`, `pickup_key_*`, `pickup_chip` | 64² | billboards |
| `weapon_axe`, `_blaster`, `_shotgun`, `_rattler`, `_mjolnir`, `_valkyrie` | 256² | viewmodel textures |
| `muzzle_flash`, `bolt_blue`, `rocket`, `rail_trail` | 64² | effects |
| `hud_frame`, `crosshair_*` | 128² | HUD bits |

### UI
`ui_panel`, `ui_button`, `ui_slider`, `title_logo` (runic "KAQQ2"), `font_atlas` (5×7 pixel font
generated from a bitmap table — no TTF files needed).

### As built (T02)
Look: palette ramps + 4×4 Bayer dithering, optional 2× "big pixel" blocks, nearest filtering.
`font_atlas` = 16×6 cells of 8×8 (ASCII 32–126). Pickups: `pickup_ammo_shells/bullets/rockets/cells/slugs`,
`pickup_armor_shard/jacket/plate`, `marine_armor/helmet/cloth`. Tests: count, sizes, determinism hash,
seam ratio of every **T** texture. `cargo run --bin texgen` also writes `assets/contact_sheet.png`.

## Meshes (T03 ✅, code-built, `src/meshgen/`)
38 models, flat-shaded boxes/prisms with per-part textures, `glows` anchors for engine sprites. Lit at
draw time (CPU, sun-direction cel bands). `cargo run --release -- --viewer` shows them all.
Names: longship skiff drakkar hauler forge fort interceptor cutter frigate destroyer dreadnought
prison_barge torpedo missile mining_bot escape_pod cargo_crate sphere sphere_lo asteroid_a/b/c ring quad
minion_grunt/guard/gunner/medic/enforcer prefect_voss marine mech drone door_frame console crate barrel reactor.

### Original plan
Longship (hull + prow dragon + sail), Skiff, Drakkar, Hauler, Forge station; Minion
interceptor, cutter, frigate, destroyer, dreadnought, prison barge, orbital fort; torpedo,
missile, mining bot, escape pod, cargo crate; sphere (planets), asteroid (3 variants), ring;
boarding: box-person (6 parts), mech, drone, door frame, console, crate, barrel, pickups.

## Sounds (T17 ✅ — `src/audio/synth.rs`, rendered at startup, nothing on disk)
Space set done (lance, torpedo, missile, explosion, shield hit, hull creak, alarm, dock clamp, beep, warp, scan + footstep/gunshot/door for boarding, 32 s music loop). Original plan:
lance, torpedo, missile, explosion (S/L), shield hit, hull hit, alarm, warp charge/jump,
dock clamp, UI click/confirm/deny, engine hum (loop), ambient drone (loop), music loop;
boarding: footsteps, door, blaster, shotgun, rattler, rocket, rail, axe, pain, die, pickup,
alarm, reactor hum.

## Level layouts (v0.38) — `src/board/maps.rs`
Ships of size 2+ are boarded in **hand-authored, Quake-2-style base maps** (hub rooms, pillared halls,
loops, key-locked shortcuts, alcoves) mirrored by seed; size 1 still uses the random room chain
(`level::generate_random`). They are **original layouts, not copies of Quake 2 maps** (the reference
repo ships no map data and the Quake 2 maps are commercial content). Maps (v0.37 list): *Outer Base* (size 2–3,
44×26), *Cargo Deck* (3–4, 60×34), *Citadel* (5, 66×38). Format: two ASCII layers (layout + things),
legend in the file header; tests check rectangular layers, solvability for every objective and mirror,
and that every locked door has its key.
v0.38 adds **height and light**: a third layer gives every tile a floor height (half-metre steps, so
stairs, daises and ramps), ceilings follow the room kind, risers and ceiling drops are drawn, and
`*` `^` `&` things are lamps (baked into vertex light plus a ceiling fixture). Rooms are no longer
boxes: octagons, circles, chamfered halls, pillar rings. Maps: *Landing Party* (size 1), *Outer Base*
(2–3), *Temple Yard* (2–3, derived from the GPL Quake 2 map spirit2dm9, see `CREDITS.md`),
*Cargo Deck* (3–4), *Citadel* (5). Still a heightfield: no overlapping floors, lifts or jump pads.

## What to replace later (everything below is a generated stand-in)
Priority 1 = most visible to a player, 3 = least. Keep the **names** (and sizes) and the game picks
the new files up; textures live in `assets/textures/*.png` (regenerate with `texgen` or overwrite).

| Area | Now | Replace with | Prio |
|---|---|---|---|
| Boarding enemies (`minion_*`, `prefect_voss`, `drone_body`) | box-person meshes (`src/meshgen/board.rs`) + 64²/128² palette textures; Quake-2-style roster and behaviour (soldier→Grunt, infantry→Guard, gunner, medic, berserk→Enforcer, tank→Mech, supertank→Voss) | real skinned/animated models + skins | 1 |
| Boarding walls/floors/doors (`wall_*`, `floor_*`, `ceiling_*`, `door_*`) | 256² dithered tiles | hand-painted 256²–512² tiling sets per theme (corvette, station, prison, bridge) | 1 |
| Weapon viewmodels (`weapon_*`) | flat 256² sprites/boxes | modelled first-person weapons + muzzle/impact effects | 1 |
| Level maps (`src/board/maps.rs`) | 3 ASCII maps | more/bigger maps, ideally made in a level editor with height (stairs, lifts, windows, secrets); a loader for an editor format would replace `Template` | 1 |
| Ship meshes (`longship`, `minion_*`, `dreadnought`, …) | code-built prisms | modelled ships with hull textures (`hull_*`) | 2 |
| Space textures (planets, sun, sky, nebula) | procedural noise | painted/NASA-style maps (licence permitting) | 2 |
| Pickups (`pickup_*`) and props (`crate_*`, `barrel`, `console_*`) | 64²/128² billboards and boxes | 3D props | 2 |
| Sounds (`src/audio/synth.rs`) | synthesised at start-up | recorded/mastered SFX, voices, music | 2 |
| HUD, font, logo (`hud_frame`, `font_atlas`, `title_logo`) | generated | designed UI skin and font | 3 |

## Validation
`texgen` prints a table (name, size, mean colour) and writes `assets/contact_sheet.png`;
tests verify count, dimensions, tileability (edge wrap diff below threshold) and determinism.
