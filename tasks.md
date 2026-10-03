# Kagga2 — Tasks

Only two lists: **Done** and **Undone**. When a task is finished, move its line to Done (same ID).
Details/acceptance for every task: `PLAN.md`. Do tasks in order unless told otherwise.

## ✅ Done

- [x] **T01** Project skeleton: Cargo crate, window, app state machine, main menu, version label, tests, lint clean _(v0.2.0)_
- [x] **T02** Procedural texture generator (`texgen`) — 116 PNGs + manifest + contact sheet _(v0.3.0)_
- [x] **T03** Asset loader + procedural mesh library (38 models: ships, stations, bodies, boarding characters/props) + `--viewer` _(v0.4.0)_
- [x] **T04** Sim core: generational entity store, 60 Hz stepper, seeded RNG, event bus, `Command` inputs, state hash _(v0.5.0)_
- [x] **T05** Flight model + cockpit/chase camera + HUD (pixel-art render target + post shader) _(v0.6.0)_
- [x] **T06** Sol sector world: 6 sectors, orbiting moons/stations, asteroid fields, sky bodies, skybox _(v0.7.0)_
- [x] **T07** Weapons: rune lances (hitscan, heat), thunder torpedoes, Gungnir missiles (homing), projectiles, explosions _(v0.8.0)_
- [x] **T08** Shields (4 facings), hull, subsystem damage, destruction + debris + bounty, shield-frequency mechanic, surrender _(v0.9.0)_
- [x] **T09** Minion ship AI: patrol → intercept → attack (circle-strafe) → flee → reinforcements; wingmen _(v0.10.0)_
- [x] **T10** Station tabs F1–F7 (Helm/Weapons/Engineering/Damage/Science/Comms/Map) + per-station crew-AI AUTO _(v0.11.0)_
- [x] **T11** Engineering: reactor budget, power + coolant sliders, heat/burnout, presets _(v0.12.0)_
- [x] **T12** Damage control: system health, degradation, 3 repair teams, crew loss without life support _(v0.13.0)_
- [x] **T13** Science: scan (hold time), shield-frequency reveal, long-range sweep, scan report _(v0.14.0)_
- [x] **T14** Nav & map: sector map, course plot, autopilot (avoids bodies), warp drive + fuel (warp gates deferred: single system) _(v0.15.0)_
- [x] **T15** Comms: hail dialogs, wingman orders, demand surrender, taunt grammar, message log _(v0.16.0)_
- [x] **T16** Starbases: docking, repair/rearm/refuel, shipyard upgrades, 12-commodity market, mining bot _(v0.17.0)_
- [x] **T17** Audio: procedural SFX (14) + 32 s seamless music loop, volume keys (-/=, F11) _(v0.18.0)_
- [x] **T18** Boarding level format + procedural generator (4 themes, 5 sizes, locks with reachable keys, solvability check) _(v0.19.0)_
- [x] **T19** First-person renderer: batched textured level, baked room lights, monsters, pickups, viewmodel, HUD (same pixel-art pipeline) _(v0.20.0)_
- [x] **T20** Player controller: Quake 2 pmove port (friction/accelerate/air), grid collision + sliding, crouch, low-g option _(v0.21.0)_
- [x] **T21** Boarding weapons (axe, blaster, scatter-gun, rattler, Mjolnir rockets, Valkyrie rail) + items/armour/keys _(v0.22.0)_
- [x] **T22** Minion monsters: Quake 2 style state machine (grunt, guard, gunner, medic revive, enforcer, mech, Voss) _(v0.23.0)_
- [x] **T23** Level logic: doors + key locks, alarm, objectives (sabotage, capture, rescue, steal, boss), airlock exit _(v0.24.0)_
- [x] **T24** Space ↔ boarding transition: B on a crippled/disabled hostile → objective choice → boarding → loot/capture/casualties _(v0.25.0)_

## ⬜ Undone

### M0 — Foundation
_(all done)_

### M1 — Space
_(all done)_

### M2 — Boarding (Quake 2 style)
_(all done)_

### M3 — Campaign
- [ ] **T25** Mission + story engine (objectives, triggers, briefings, saga dialogue)
- [ ] **T26** Campaign missions 1–5
- [ ] **T27** Campaign missions 6–10, bosses, ending
- [ ] **T28** Save/load, settings, difficulty, key bindings
- [ ] **T29** Tutorial + in-game help screens

### M4 — Polish
- [ ] **T30** Balance pass + headless bot playtest harness
- [ ] **T31** Performance, release builds (macOS/Linux/Windows), final docs, licence check

### M5 — Optional / future
- [ ] **T32** Multiplayer: server-authoritative sim, networking, lobby, station assignment
- [ ] **T33** Co-op boarding
- [ ] **T34** Mission scripting file format / modding hooks
