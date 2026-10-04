# Kagg2 — Game Design

## 1. Loop

```
Briefing → Space (fly / fight / scan / trade) ─┬─ objective done → Debrief → next mission
                                               └─ enemy disabled → [B] Board → FPS level → back to Space
```

## 2. Space

### 2.1 The ship (*Huginn*, class **Longship**)
| Stat | Start | Upgrade cap |
|---|---|---|
| Hull | 400 | 800 |
| Shields (4 facings, each) | 100 | 250 |
| Reactor output | 100 units | 200 |
| Rune lance | 1 mount | 3 mounts |
| Torpedoes | 4 | 8 |
| Missiles | 6 | 12 |
| Warp fuel | 100 | 200 |
| Cargo bays | 4 | 10 |
| Crew (repair teams 3, marines for boarding) | 12 | 24 |

### 2.2 The six systems
Each system has **health** (0–100 %) and **power** (0–200 % slider) and **heat**.

| System | Affects |
|---|---|
| Engines | thrust, turn rate, top speed |
| Warp | charge time, jump range |
| Shields | recharge rate, max strength |
| Lances | damage, fire rate (heat) |
| Sensors | scan speed, detection range, lock time |
| Life Support | crew oxygen, repair-team survivability |

Effective output = `power% × health% × (1 − overheat penalty)`.
Reactor gives a fixed budget; sliders must sum ≤ budget. Coolant (a second pool, 100 units)
lowers heat per system. Over 100 % heat deals damage to the system.

### 2.3 Stations (F-keys) and crew AI

| Key | Station | What you do manually | What crew AI does when AUTO |
|---|---|---|---|
| F1 | Helm | fly, throttle, brake, warp | keeps course on autopilot, avoids collisions |
| F2 | Weapons | aim, fire lance/torp/missile, set lance freq | fires at locked target with skill-based aim error |
| F3 | Engineering | power + coolant sliders, presets | rebalances for threat (Battle/Run/Balanced) |
| F4 | Damage Ctrl | send repair teams | repairs lowest-health critical system first |
| F5 | Science | scan target, sweep | scans the nearest hostile, reports freq |
| F6 | Comms | hail, wing orders, dock request | auto-accepts friendlies, announces threats |
| F7 | Map | plot course, waypoints, warp target | – |

Crew skill 1–5 (grows via missions). AUTO is never as good as a focused player — e.g. AUTO
weapons aim error ±6°, MANUAL 0°; AUTO engineering reacts after 3 s lag.
**Tab = tactical pause** (single player only) lets you issue orders without time pressure.

### 2.4 Combat
- **Shields** absorb first, by facing; hit → shield value drops, recharges after 4 s no-hit.
- **Frequency:** shields freq 0–9. Lance freq set on Weapons screen. `Δ = |lance − shield|`:
  Δ≤1 → ×2 damage to shields, Δ 2–3 → ×1, Δ≥4 → ×0.5. Science scan shows enemy freq.
  Torpedoes ignore freq but are slow; missiles home but can be shot down.
- **Hull hit** (shields down): hull damage + 30 % chance to damage a random system.
- **Destroyed**: hull 0 → explosion, debris. **Disabled**: engines+weapons 0 % but hull > 0
  → can be **boarded**.
- **Weapons**
  | Weapon | Damage | Range | Rate | Notes |
  |---|---|---|---|---|
  | Rune lance | 8 / shot (×mounts) | 2 500 | 4/s | heat builds, freq-sensitive |
  | Thunder torpedo | 120 | 4 000 | 1 / 5 s | slow, big, 8 max |
  | Gungnir missile | 60 | 6 000 | 1 / 3 s | homing, needs lock (3 s), 12 max |

### 2.5 Enemy ships (Concord Minions)
| Class | Hull | Shield | Speed | Weapons | Role |
|---|---|---|---|---|---|
| Interceptor | 40 | 30 | 1 400 | lance | fast raider (SNIS "scrambler/vanquisher") |
| Cutter | 120 | 80 | 1 000 | lance + missile | boardable, common |
| Frigate | 300 | 150 | 800 | lance, torp | squad leader |
| Destroyer | 500 | 250 | 700 | lance, torp, missile | escorts |
| Dreadnought | 1 500 | 600 | 400 | everything | boardable boss ship |
| Prison barge | 200 | 40 | 500 | – | rescue boarding |
| Orbital fort | 2 500 | 800 | 0 | turrets | base |
Class rows reflect `share/snis/ship_types.txt` toughness/speed, rescaled.

### 2.6 Nav & Warp
- Sector = a 3D volume ≈ 20 000 units across, with 5–25 objects.
- **Warp** between sectors (needs fuel, 5 s charge, can't charge under fire at <50 % Warp health).
- **Warp gates** between systems (Mars↔Belt↔Jupiter↔Luna↔Earth). Starts unlocked by campaign.
- Autopilot: "Fly to" any contact/waypoint; stops at 500 units.

### 2.7 Economy (simple)
Credits (¤). 12 commodities (ore, ice, fuel cells, iron, rune-steel, mead, rations, medicine,
machine parts, tech-chips, contraband, luxury) with base price + station volatility ±30 %
(`commodities.c` model reduced). Starbases: **Refuel**, **Repair**, **Rearm** (cost ¤),
**Shipyard** (upgrades), **Trade**. Mining: deploy bot to asteroid, 60 s → ore, sell for ¤.
Credits also come from mission rewards and boarding loot.

## 3. Boarding (Quake 2 style)

### 3.1 Flow
Disabled ship in range → **B** → choose objective → FPS level generated from seed
(ship class → size and theme) → complete or flee through the airlock → results.

### 3.2 Player
Quake-2 feel: 320 units/s run, jump, crouch, strafe, mouse look. Health 100 (max 100, +mega
200), armor types (shard +2, jacket 25, plate 50 = ~66 % absorption). Optional zero-g sections
in cargo bays (low gravity multiplier).

### 3.3 Weapons (damage from `p_weapon.c`)
| # | Name | Q2 origin | Dmg | Ammo | Notes |
|---|---|---|---|---|---|
| 1 | Axe | – (Viking melee) | 40 | – | fast, short |
| 2 | Blaster "Sparkbolt" | Blaster | 15 | ∞ | slow projectile |
| 3 | Scatter-gun "Thunder" | Shotgun | 6×12 pellets | shells | wide spread |
| 4 | Rune Rattler | Machinegun | 8 | bullets | hitscan, spread grows |
| 5 | Mjolnir launcher | Rocket | 100–120 + splash 120 | rockets | self-damage |
| 6 | Valkyrie Lance | Railgun | 150 | slugs | pierces |

### 3.4 Enemies (Quake 2 stats)
| Minion | Q2 | HP | Behaviour |
|---|---|---|---|
| Grunt | soldier | 20/30/40 | blaster/shotgun/machinegun variants, dodge |
| Guard | infantry | 100 | machinegun volley, melee fallback |
| Gunner | gunner | 175 | chaingun + grenade |
| Medic | medic | 300 | revives dead Minions, blaster |
| Enforcer | berserk | 240 | melee, leaping slam |
| Mech | tank | 750 | rocket + machinegun, mini-boss |
| Prefect Voss | supertank | 1 500 | boss, rockets + chain gun, 2 phases |
| Drone | flyer | 50 | flying, ram + blaster |

### 3.5 Objectives
Sabotage reactor (plant charge, then run), Capture bridge (kill all in room), Rescue prisoners
(open cells, escort to airlock), Steal data core (item → airlock), Kill boss. Alarm: after N
shots heard, reinforcements spawn at the airlock. Keycards Bronze → Silver → Gold.

## 4. Controls (defaults)

| Context | Keys |
|---|---|
| Global | F1–F7 stations, Tab pause, Esc menu, F5/F9 quicksave/load (T28), `` ` `` / F12 developer console (v0.36: god, heal, ammo, credits, kill, win, lose, `level N`, `board [N]`, menu, quit) |
| Helm | W/S throttle, A/D yaw, Q/E roll, ↑/↓ pitch (↓ = nose up), ←/→ strafe, R/F up/down, Space brake, X zero, LShift full, M assist, C camera, T/G target/clear, H help, B board, J warp |
| Weapons | Enter/LMB (Helm tab) lance, Z torpedo, V missile (needs T lock), [ ] lance frequency, , . shield frequency |
| Stations | F1–F7 tabs, Tab = toggle crew AUTO for the open station, N scan, U sweep, P autopilot, O go to target, L dock/undock, K mine, 1–3 power presets (Engineering), H hail (Comms) |
| Audio | -/= master volume, F11 music on/off |
| Boarding | WASD move, mouse look, Space jump, C crouch, LMB/Enter fire, 1–6 weapon, E use (doors auto-open; E at consoles, prisoners, alarm, airlock), Esc abort to ship |

## 5. Difficulty (T28)
**Thrall** (easy): enemy damage ×0.6, aim error ×2. **Karl** (normal). **Jarl** (hard): enemy
damage ×1.4, enemy shield regen ×1.5, crew AI lag +1 s.

## 6. Multiplayer-friendliness (future)
Everything player-driven goes through `Command` values applied to the sim at fixed ticks.
A seat = (station, controller) where controller is `CrewAi` or `Human(player_id)`.

## 7. As built in M1 (v0.18)
- **Damage:** lance 14 × level × lance power; torpedo 140 (+ 90-unit splash), missile 55 (homing, 2.4 rad/s).
  Shield facing is chosen from the attacker's bearing. Lance vs shield frequency: ±1 → ×2, ±2–3 → ×1, ≥4 → ×0.5.
  Hull hits damage a random system (0.6 % of the overflow); hull < 12 % makes an AI ship surrender (boardable).
- **Power:** reactor budget 6.0 (+20 %/level); six sliders 0–200 %; over-budget requests are scaled down.
  Heat = power² × 4/s minus coolant share; ≥ 100 burns the system. < 50 % health degrades, 0 % = offline.
- **Crew AI:** each of Helm/Weapons/Engineering/Damage/Science can run on AUTO (skill 0.7).
- **Economy:** 12 commodities, per-station bias ±30 % and drift ±15 % every 90 s; Forge services at half price.
- **Warp:** charge 6 s at nominal power; fuel cost 5 (same host) to ~40 (Mars → Jupiter). Warp gates deferred
  (single system). Enemies respawn when you re-enter a sector.

## 8. As built in M2 (v0.25)
- **Trigger:** target a hostile ship that is *disabled* (hull < 12 %) or crippled (hull < 60 % and shields < 15 %), get within 450 units of its hull and press **B**.
  Class picks the level: Interceptor → corvette (size 1), Cutter → corvette or prison barge (2), Frigate/Destroyer → station (3/4), Dreadnought → bridge (5).
- **Levels:** tile grid, 4 m tiles; rooms chained left→right from the airlock; doors at room entrances; up to 3 doors on the main path are locked (bronze → silver → gold)
  and each key lies in a region reachable without it. A flood-fill solver (keys picked up on the way) must reach the goal or the level is regenerated.
- **Objectives:** Sabotage (set charges at the reactor, escape in 45 s), Capture bridge (clear the room, E at the console), Rescue 3 prisoners, Steal the data core, Kill Prefect Voss.
  Finish → return to the airlock and press E. Leaving early (E in the airlock, or Esc) = *Fled*; dying ends the mission.
- **Outcome in space:** win → +credits (15 per kill, +300 for an objective), +2 torpedoes/+3 missiles, boarded ship removed (captured or sabotaged); crew lost = damage taken / 25.
- **Combat numbers:** blaster 15, scatter-gun 12×4, rattler 8 @10/s, Mjolnir 120 + 5 m splash, Valkyrie 150 piercing, axe 25; armour absorbs 30/60/80 % by amount held.
  Monsters (hp): grunt 30, guard 100, gunner 175, medic 300 (revives corpses in 3 s), enforcer 240 (melee), mech 750, Voss 1500.

## 9. As built in M3 (v0.30)
- **Mission engine** (`sim/mission.rs`, headless): a mission is a list of *stages*; each has a goal, timed *beats* (dialogue lines and ship spawns), an optional "must not be destroyed" label prefix and an optional time limit.
  Goals: throttle, target, reach a body, dock, scan N, hold ore, buy an upgrade, wait, clear all hostiles (optionally except a label), label gone, label crippled, complete a boarding with a given objective, escort haulers.
  `SpaceScene` steps it after every sim step; notes become HUD captions; a decided mission ends after a 3 s banner. Stock sector enemies are removed so missions spawn their own.
- **Campaign:** 1 First Raven (tutorial) · 2 Phobos Watch · 3 Iron Tithe · 4 Boarding Party (steal data) · 5 The Silent Moon (rescue) · 6 Jovian Toll (upgrade) · 7 Wolves of the Belt (wingmen) · 8 Gate of Frost (3 s generators on a timer, sabotage) ·
  9 Luna Siege (sabotage the Bulwark) · 10 Ragnarök Over Earth (kill Voss on Aegis Prime). Win → next mission, loss → retry. After mission 10 the player picks an ending.
- **Carry-over:** credits and upgrade levels (`Carry`); each mission refits hull, shields and ammo.
- **Settings** (`~/.kagg2/settings.txt`, or `$KAGG2_HOME`): difficulty scale Thrall 0.7 / Karl 1.0 / Jarl 1.4 (enemy aim and boarding damage), volume, music, mouse sensitivity, invert-Y, 26 flight keys.
  **Save** (`save.txt`): next mission, credits, upgrades, ending; written after each won mission. Dev skip: `KAGGA_DEV=1` enables F9.
- **Help:** F8 = overlay for the open station tab, boarding card on the objective screen, menu Help screen (flight, boarding, each station, tips).

## 10. As built in M4–M5 (v0.35)
- **Balance (T30)** — all knobs in `src/balance.rs`: difficulty scale 0.7 / 1.0 / 1.4; boarding party starts with armour 40 and a scatter-gun (20 shells);
  monster damage to the party ×0.25 (hit chance scales by half of the difficulty step); monsters hear gunfire at 22 m; one extra pickup per room;
  extra-monster chance 20 % per size step per room (sizes 1–5 average 7 / 14 / 19 / 27 / 37 monsters); sabotage fuse = max(45 s, walk back × 1.6 + 20 s).
  Campaign capital ships: Frost Command = station size 3, Bulwark = bridge size 4, Aegis Prime = bridge size 5.
  Bots found and fixed: fleeing ships now jump to warp when 9 km clear of every enemy (a chase could never end); mission 10's second squadron arrives after 45 s
  (the opening was an alpha strike that killed a stock ship in 12 s); sabotage fuse scaled to the level (45 s was unwinnable on deep levels).
  `cargo run --release --bin playtest` prints the tables; `playtest N boarding` the boarding table; `playtest N MISSION` one mission; `playtest bench` timings.
- **Release (T31)** — `[profile.release]` thin LTO, one codegen unit, stripped. Sim cost per 60 Hz step: about 2–5 µs in a fight, about 20 µs in a size-5 boarding level
  with a party of three (budget 16 667 µs), so frame time is all rendering; on an Apple-silicon Mac the game averages about 10 ms per frame in space and boarding scenes.
  `tools/release.sh` runs checks and builds `dist/kagg2-<ver>-<os>-<arch>.tar.gz` (zip on Windows); `.github/workflows/release.yml` runs it on macOS, Linux and Windows runners.
  `tools/licence_check.py` audits dependency licences, the GPL text and "no reference file shipped" and writes `LICENCES.md`.
- **Multiplayer (T32)** — see `ARCHITECTURE.md`: seats per station, crew AI keeps the empty ones, server-authoritative 60 Hz sim, 20 Hz snapshots, std TCP. Text client only.
- **Co-op boarding (T33)** — parties of up to 4 (host + 3 mates); the objective screen's **P** adds 0–2 AI shield-brothers who walk the same plan as the playtest bot.
- **Mission scripts (T34)** — `.k2m` text files; format in the manual (`HTML/manual.html#mission-scripts`) and `src/sim/script.rs`.

## 11. Console and cheats (v0.36)
`` ` `` or F12 opens a pausing console on any screen (Up/Down = history). Commands: `help`, `god [on|off]`, `heal`, `ammo`/`keys`, `credits N`, `kill`, `win`, `lose`, `level N` (space part of mission 1–10), `board [N]` (boarding level of mission N: 4, 5, 8, 9, 10; none = current mission), `menu`, `quit`. God mode refills hull/shields (space) or health (boarding) every frame. Jumps build a fresh scene with the saved campaign ship and do not touch the save; a `board` jump uses a dummy target, so winning it returns to space without advancing the mission stage.
