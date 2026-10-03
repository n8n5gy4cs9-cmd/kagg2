# Kagg2

**Mars Vikings vs. Earth Minions.** Captain the longship *Huginn* through a simplified
bridge-sim space war (inspired by *Space Nerds In Space*), then storm disabled enemy ships and
bases in first-person boarding actions (inspired by *Quake 2*). Written in Rust. Single player
first; multiplayer is an optional future goal.

> Status: T01–T29 done (M0 foundation, M1 space, M2 boarding, M3 campaign: 10 missions, save/settings, tutorial/help); building task by task — see [tasks.md](tasks.md).

## Run
```sh
cargo run --release              # the game (textures are generated on first run)
cargo run --bin texgen --release # regenerate all textures into assets/textures/
cargo run --release -- --viewer  # model viewer (left/right = next model)
cargo run --release -- --space mars   # jump straight into a sector
cargo test --release
./run.sh [fight|dock|board|tab N|sector NAME|viewer|test|shot]   # quick launcher, see header of run.sh
```

Flight controls: **W/S** throttle, **A/D** yaw, **↑/↓** pitch, **Q/E** roll, **←/→** strafe,
**R/F** up/down, **Space** brake, **X** zero throttle, **C** camera, **T** target, **M** flight
assist, **H** help, **Esc** menu.
Combat/stations: **Enter**/click lances, **Z** torpedo, **V** missile (lock with **T**), **[ ]** lance frequency,
**, .** shield frequency, **F1–F7** station tabs, **Tab** crew AUTO, **N** scan, **P** autopilot, **L** dock, **K** mine, **-/=** volume.

## Docs
| File | What |
|---|---|
| [tasks.md](tasks.md) | Done / Undone checklist (the only to-do list) |
| [PLAN.md](PLAN.md) | Detailed plan, tech decisions, task acceptance |
| [PRD.json](PRD.json) | Machine-readable product requirements |
| [GAME_DESIGN.md](GAME_DESIGN.md) | Systems, numbers, controls |
| [STORY.md](STORY.md) | Setting, factions, cast, 10 missions |
| [ARCHITECTURE.md](ARCHITECTURE.md) | Module layout and rules |
| [REFERENCE_MAP.md](REFERENCE_MAP.md) | What is ported from which reference file |
| [ASSETS.md](ASSETS.md) | Generated texture/mesh/sound lists |
| [HANDOFF.md](HANDOFF.md) | Session notes: what's done, next, how to resume |
| [CREDITS.md](CREDITS.md) | Licences and credits |

## Licence
GPL-2.0-or-later (see [CREDITS.md](CREDITS.md)).
