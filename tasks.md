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

## ⬜ Undone

### M0 — Foundation
_(all done)_

### M1 — Space
- [ ] **T07** Weapons: rune lances, thunder torpedoes, Gungnir missiles, projectiles, explosions
- [ ] **T08** Shields, hull, subsystem damage, destruction, shield-frequency mechanic
- [ ] **T09** Minion ship AI (patrol, intercept, attack, flee, reinforcements)
- [ ] **T10** Station-tab UI (Helm/Weapons/Engineering/Damage/Science/Comms/Map) + crew-AI auto-operation
- [ ] **T11** Engineering: power + coolant sliders, heat, reactor
- [ ] **T12** Damage control: system health, repair priorities, repair crew
- [ ] **T13** Science: scanner, target info, shield-frequency reveal
- [ ] **T14** Nav & map: sector map, course plot, autopilot, warp drive/gates, fuel
- [ ] **T15** Comms: hail, wingman orders, station chat, taunts, mission messages
- [ ] **T16** Starbases: docking, repair/resupply, shipyard upgrades, trade + mining
- [ ] **T17** Audio: procedural SFX + music loops

### M2 — Boarding (Quake 2 style)
- [ ] **T18** Boarding level format + procedural generator (ships/bases)
- [ ] **T19** First-person renderer (textured geometry, lights, sprites, viewmodel, HUD)
- [ ] **T20** Player controller + collision (Quake 2 pmove port)
- [ ] **T21** Boarding weapons (axe, blaster, shotgun, machinegun, rocket, rail) + items/armor
- [ ] **T22** Minion monsters: Quake 2 AI state machine (grunt, guard, gunner, medic, enforcer, mech)
- [ ] **T23** Level logic: doors, buttons, keycards, triggers, objectives, alarms
- [ ] **T24** Space ↔ boarding transition (disable, clamp, board, loot, capture/destroy)

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
