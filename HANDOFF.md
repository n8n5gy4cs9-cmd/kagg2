# HANDOFF

## State (v0.37.0)
- **Boarding maps (T36):** `src/board/maps.rs` — 3 authored ASCII maps (Outer Base 2–3, Cargo Deck 3–4, Citadel 5), two layers (layout + things), mirrored by seed; `level::generate` uses them for size ≥ 2 and falls back to `generate_random`. Bots still win missions 4/5/8/9/10 (3 seeds each); solo bot survey: size 3 ≈ 3–5/6, size 5 hard (as before; capital ships are meant for partners). Not Quake maps (no map data exists; commercial) — original layouts. Replacement list in `ASSETS.md`. Next: height (stairs/lifts), more maps, an editor-format loader.
- Fuse test now compares sums over seeds (authored maps make path length seed-dependent).

## Console (v0.36.0)
- **Dev console (T35):** `` ` `` or F12 opens it anywhere (game pauses). `src/cheats.rs` parses (`Cheat`), `SpaceScene::cheat` / `BoardScene::cheat` apply, `main.rs` handles `level N` / `board [N]` jumps (fresh scene of that mission; `board` uses the mission's real forced objective with a dummy target id, so winning it does not advance the mission stage). Commands: `help god heal ammo credits kill win lose level board menu quit`. Not played by a human yet (headless tests only: parser, `boarding_stage`, `kill_all`).

- **T01–T34 done = everything in `PLAN.md` (M0–M5).** `./run.sh` launches (see its header). New binaries: `playtest` (bot reports), `server` + `netclient` (LAN), plus `texgen`.
- **Space sim** (headless, `src/sim/`): `ops.rs`, `combat.rs`, `ai.rs` (fleeing ships now jump to warp when 9 km clear), `crew.rs`, `systems.rs`, `science.rs`, `nav.rs`, `comms.rs`, `econ.rs`, `sector.rs`, `mission.rs`,
  plus **`bot.rs`** (bridge bot), **`boarding.rs`** (space-side boarding aftermath, shared by the scene and the harness), **`script.rs`** (`.k2m` mission scripts).
  Step order in `World::step`: controllers (AI, crew, autopilot) → apply commands → orbits → missile steering → movement/docked hold → projectile collision → upkeep → solid collisions.
- **Boarding sim** (headless, `src/board/`): `level.rs`, `pmove.rs`, `weapons.rs`, `monster.rs`, `world.rs` (`step` = single player, `step_team` = party of up to 4, `add_mates`, `with_player`), **`bot.rs`**, `theme_for` in `mod.rs`.
- **`src/balance.rs`** holds every tunable the bots adjust; **`src/playtest.rs`** is the harness (`run_mission`, `run_campaign`, `survey`, `board_survey`, `bench`); **`src/net/`** is the LAN layer (`wire`, `proto`, `server`, `client`).
- **GFX**: unchanged structure; `board_scene.rs` gained AI partners (`P` on the objective screen, `partners`, `bots`), `main.rs` gained `--script FILE`, `--coop N`, `$KAGG2_HOME/missions/*.k2m` loading, and prints average/worst frame time with `--shot`.
- Dev flags: `--space <sector> [--fight] [--fire] [--dock] [--tab N] [--chase] [--target]`, `--board [1-4] [--size 1-5] [--coop N]`, `--script FILE`, `--viewer [model]`, `--shot out.png [--frames N]`.

## Session v0.31.0–v0.35.0 (T30–T34, one session)
- **T30**: bots play all ten missions; results in `HTML/manual.html#playtest`. Bots fixed real problems (endless chases, mission 10 alpha strike, sabotage fuse too short on deep levels, boarding too deadly beyond size 2: density cliff replaced by a per-room chance, monster damage ×0.25, start kit). Capital ships: Frost Command size 3, Bulwark 4, Aegis Prime 5 (`board::theme_for`).
  Latest tables (4 seeds, typical ship per mission): every mission won on every seed at Thrall/Karl/Jarl except mission 4 Jarl (3/4) and mission 8 Thrall (3/4: the bot sank the prey). Chained campaigns: Thrall and Karl win all ten; Jarl stops at mission 4 (boarding the Courier).
  Solo boarding bot on Karl: size 1 12/12 (corvette-type) and 7/12 (bridge-type); size 3 9/12 and 6/12; size 5 1/12 and 0/12 — capital ships are boarded with 1–2 AI partners in the harness.
- **T31**: thin LTO release profile, `tools/release.sh`, `.github/workflows/release.yml`, `tools/licence_check.py` (all 48 packages permissive; writes `LICENCES.md`), `LICENSE` (GPL v2 text, copied from the reference repo's COPYING), `license` in `Cargo.toml`. Sim step 2–20 µs; game ≈10 ms/frame on this Mac.
- **T32**: LAN server (`bin/server`), text client (`bin/netclient`), integration tests over loopback TCP. **T33**: co-op `BoardWorld` + AI partners. **T34**: `.k2m` scripts, example `missions/wolf-pack.k2m`.
- Verified: `cargo clippy --all-targets -- -D warnings` clean, `cargo test --release` passes (see the final run in the session summary), `cargo fmt`, `python3 tools/licence_check.py`, `tools/make_html.py`, screenshots of a script mission and a co-op boarding start, a real server + client run over localhost.
  **Not verified:** the Linux/Windows release builds (CI workflow never ran), a human playing anything new (partners' look/feel, `P` toggle, script missions in the full menu flow), network play between two machines, any graphical networking.

## Known gaps / debt (one line each)
- Dock screen mutates `Ops` directly via `econ` (multiplayer seam would need `Command::Buy…`). Warp gates not built (single system). Enemy squads respawn on re-entering a sector.
- Minion stations (Concord forts, Geneva Dock) are inert targets — no turrets, not boardable.
- Missions use stock sectors; warp gates are not built (mission 6 is a plain blockade fight). Balance was tuned with bots only; a human should replay missions 8–10 and boarding sizes 4–5.
- Multiplayer: no graphical client, no boarding/dock screen over the network (so missions with a boarding goal cannot be finished online), no encryption/auth, no reconnect. Co-op partners are AI only in the game.
- Mission scripts: no briefing/ending text, script data is leaked (`Box::leak`) by design; the `Board` goal in a script needs a ship label that exists.
- Bots: the bridge bot never uses cover or comms, the boarding bot never retreats; they under-estimate human skill. `STUCK` in a table means the bot gave up (three failed boardings or the 1200 s cap).
- Boarding: monsters path by BFS ignoring locks (they open unlocked doors only); no gibs/ragdoll; pickups are billboards; no flashlight; prisoners do not follow; level music/ambience not separate.
- Map is a flat X/Z projection (no altitude).

## Next (ideas, none scheduled)
- A graphical network client (render a replica `World` from `Snapshot`s, send `Command`s) and boarding over the network on top of `BoardWorld::step_team`.
- Human playtest pass on missions 8–10 and boarding sizes 4–5 using `balance.rs`; bots say the numbers are reachable, not that they are fun.
- Run `.github/workflows/release.yml` once and fix whatever Linux/Windows break; remove `REFERENCE_tobe_DELETED/` only when the maintainer says so (the licence check uses it).

## Resume
1. Read `tasks.md` → first undone task → its section in `PLAN.md`.
2. `cargo check && cargo clippy --all-targets -- -D warnings && cargo test --release && cargo fmt`.
3. Run in `--release` (debug builds are slow for the CPU-lit meshes).

## Notes
- Quake 2 reference has no art/maps; everything visual is generated. Licence: GPL-2.0-or-later (see `CREDITS.md`).
- Edition 2024 reserves `gen`; let-chains are used (clippy collapsible_if). `is_multiple_of` is preferred over `% n == 0`.
- macroquad draw-call capacity is raised in `window_conf` (60k verts). Mesh UVs stay inside 0..1 (no wrap mode), so level quads are one tile each.
- Discrete commands wait in `SpaceScene::queue` until a fixed step runs (frames with 0 steps used to drop key presses).
