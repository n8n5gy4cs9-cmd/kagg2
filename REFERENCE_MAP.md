# Reference → Kagga2 map

"Clone into Rust" = re-implement the logic/data in idiomatic Rust, not transliterate C.
References live in `REFERENCE_tobe_DELETED/` (read-only; deleted at T31 on request).
Licences: both GPL-2.0-or-later → Kagga2 is GPL-2.0-or-later (see `CREDITS.md`).

## Space-Nerds-in-Space (`space-nerds-in-space-master/`)

| Reference | Kagga2 module | Take | Skip |
|---|---|---|---|
| `share/snis/ship_types.txt`, `snis_ship_type.c` | `sim/space/ship_class.rs` | class table: toughness, max speed, weapons, cargo, crew | 40 classes → ~14 |
| `power-model.c/.h` | `sim/space/power.rs` | power/coolant/temperature per system, overheat damage | exact constants (rebalanced) |
| `shield_strength.c`, phaser wavelength | `sim/space/shields.rs` | freq-vs-weakness damage multiplier | shield colour-wheel UI |
| `snis_server.c` (movement, damage, AI loops) | `sim/space/{flight,weapons,ai}.rs` | thrust/turn integrators, damage application, NPC states | 33 k-line monolith, network code |
| `snis_client.c` (screens) | `gfx/stations/*.rs` | station layouts: nav, weapons, eng, damcon, science, comms | 26 k lines of UI widgets |
| `help_text_*.txt` | `gfx/help.rs` | wording/structure of per-station help | – |
| `commodities.c`, `share/snis/commodities.txt` | `sim/space/economy.rs` | price = base × volatility, categories | 100+ items → 12 |
| `snis_faction.c`, `share/snis/factions.txt` | `sim/space/faction.rs` | faction hostility matrix | – |
| `docking_port.c`, `starbase-comms.c` | `sim/space/docking.rs`, `comms.rs` | dock handshake, station chat | – |
| `infinite-taunt.c` | `sim/space/comms.rs` | taunt grammar (shrunk to ~60 lines of templates) | – |
| `elastic_collision.c`, `arbitrary_spin.c`, `quat.c` | `sim/space/flight.rs` | collision bounce, quaternion rotation | – (glam has `Quat`) |
| `planetary_properties.c`, `earthlike.c`, `crater.c`, `nebula_noise.c`, `open-simplex-noise.c` | `texgen/` | noise + crater + planet recipes (reimplemented) | their PNG assets (we generate our own) |
| `share/snis/models/*.scad` | `meshgen/` | silhouettes of classic hulls (cruiser, destroyer, freighter, starbase) as *inspiration only* | the SCAD files / STL |
| `share/snis/sounds/*` | `audio/synth.rs` | list of needed effects | the OGG files (CC) – we synthesise |
| `snis_server.c` mining bot, asteroids | `sim/space/mining.rs` | bot deploy → ore | – |
| `doc/lua-api.txt` | `missions.rs` | idea of mission API (spawn, trigger, message) | Lua |
| multiverse, voice chat, DMX, arduino, lobby, RTS | – | – | all cut (voice/lobby maybe in T32) |

## Quake 2 (`Quake-2-master/`, code only — no maps/art in the repo)

| Reference | Kagga2 module | Take | Skip |
|---|---|---|---|
| `qcommon/pmove.c` (movement physics) | `sim/board/pmove.rs` | accelerate, friction, stopspeed, air accel, jump, crouch, slide-move | BSP trace → grid AABB |
| `game/g_ai.c`, `g_monster.c`, `m_move.c` | `sim/board/monster.rs` | `ai_stand/walk/run/charge`, visibility, `M_walkmove`, step-up, `FindTarget`, pain/die flow | animation frame tables |
| `game/m_soldier.c`, `m_infantry.c`, `m_gunner.c`, `m_medic.c`, `m_berserk.c`, `m_tank.c`, `m_supertank.c`, `m_flyer.c`, `m_hover.c` | `sim/board/monsters/*.rs` | health, mass, attack logic, timing, damage numbers | models/frames/sounds |
| `game/p_weapon.c`, `g_weapon.c` | `sim/board/weapons.rs` | fire functions, damage, spread, ammo use, rocket splash, rail pierce | animation states |
| `game/g_items.c` | `sim/board/items.rs` | health/armor/ammo/keys, respawn off | powerups (quad etc.) except maybe Quad (optional) |
| `game/g_combat.c` | `sim/board/combat.rs` | `T_Damage`, armor save, knockback, radius damage | teams/CTF |
| `game/g_func.c`, `g_trigger.c`, `g_target.c`, `g_misc.c` | `sim/board/logic.rs` | doors, buttons, triggers, targets, explosions, lights | platforms, trains, rotating |
| `game/g_spawn.c`, `g_save.c` | `sim/board/spawn.rs`, `save.rs` | spawn table by name; serialization idea | field-offset reflection |
| `client/cl_view.c`, `cl_fx.c`, `ref_gl/*` | `gfx/board_view.rs` | view bob, damage flash, muzzle flash ideas | whole renderer |
| `ctf/`, `win32/`, `linux/`, `irix/`, `solaris/`, `rhapsody/`, `ref_soft/`, `server/` | – | – | cut |

## Rule for porting
1. Read the reference function; write the Rust version with the *same observable behaviour*
   (numbers, timings), documenting `// ref: file.c fn_name` above it.
2. Prefer data tables over per-monster copy-paste.
3. Add a unit test pinning one behaviour (e.g. rocket splash falloff, friction at 0 speed).
