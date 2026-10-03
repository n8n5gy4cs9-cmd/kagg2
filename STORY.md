# Kagg2 — Story

## Setting

**Year 2347 of the Concord Calendar — "Ragnarök Year Three" to Martians.**

Two centuries ago the first Mars colonists were Nordic miners and shipwrights who refused the
Earth Concord's Tithe-Clause. Over generations *Marsgard* grew into a hard, proud clan-realm:
iron-and-rune starships called **longships**, a council of **Jarls**, and sagas sung over the
reactor hum. Earth, a tidy single state run by the **Concord**, calls them "the Rust Raiders".

Three years ago the Concord declared the Martian clans a "rogue market" and sent its fleets.
Its soldiers are the **Minions**: mass-produced, loyalty-conditioned conscripts in white-and-cyan
armour, led by career **Prefects**. Minions do not retreat, do not loot, and do not stop.

The war is old. The Jarls have one plan left: the *Spear of Odin* — hit the Concord's chain of
forward bases in the Belt, Jupiter and Luna, steal the fleet codes, and take the flagship
*Aegis Prime* over Earth before the Concord's Great Armada finishes building.

## Factions

| Faction | Colour | Notes |
|---|---|---|
| **Marsgard Clans** (player) | rust red / bone / iron | longships, skiffs, drakkars, the Forge (home base) |
| **Earth Concord Minions** (enemy) | white / cyan | interceptors, cutters, frigates, dreadnoughts, barges, forts |
| **Belt Free Miners** (neutral) | ochre | trade, hints, side contracts; hostile only if you attack them |
| **Jotun Yards** (neutral, Jupiter) | blue-grey | shipyard, upgrades for credits |

## Cast (all voices are text lines; names are placeholders you can rename)

- **You** — *the Captain* (player picks the name; default "Rurik"). Newly sworn Hersir of the *Huginn*.
- **Jarl Ingrid Stormhand** — Marsgard war leader, gives briefings.
- **Sigrun Far-Sight** — navigator / helm. Dry humour. Runs Helm + Nav crew-AI.
- **Bjorn Gearbeard** — chief engineer. Speaks to the reactor. Runs Engineering + Damage crew-AI.
- **Astrid the Völva** — science / seer. Reads scans as omens. Runs Science crew-AI.
- **Skald Eirik** — comms officer and the ship's poet; writes taunts.
- **Prefect Aurelian Voss** — Concord fleet commander, final boss. Polite, cold, believes he is saving everyone.
- **Admiral-Prefect Halden** — mid-campaign villain at Luna.

## Campaign — 10 missions (Mars → Earth)

Each mission: Briefing (Jarl Ingrid) → Play → Debrief with saga verse. Space unless marked **[BOARD]**.

| # | Mission | Place | Gist | New toys |
|---|---|---|---|---|
| 1 | **First Raven** | Mars Orbit | Shakedown. Fly the *Huginn*, shoot target drones, dock at the Forge. Minion scouts jump in at the end. | flight, lances, docking, tutorial |
| 2 | **Phobos Watch** | Phobos | Defend a mining station from Minion raiders. Power management matters. | Engineering, shields |
| 3 | **Iron Tithe** | Belt | Escort ore haulers; mine ore; scan for ambushers. | Science, mining, trade |
| 4 | **Boarding Party** | Belt | Disable a Minion cutter and storm it; steal the *Codebook*. | **[BOARD]** first boarding, torpedoes |
| 5 | **The Silent Moon** | Deimos | Hide in asteroid shadow, wait for the Minion convoy, free Martian prisoners from the barge. | missiles, **[BOARD]** rescue |
| 6 | **Jovian Toll** | Ganymede | Pay (or fight) at a Concord toll gate; upgrade at Jotun Yards; warp gate to the inner system. | warp, shipyard, upgrades |
| 7 | **Wolves of the Belt** | Belt | Fleet battle with 3 skiff wingmen; give orders by comms. | wingmen, comms orders |
| 8 | **Gate of Frost** | Concord forward base | Destroy shield generators, then **[BOARD]** to sabotage the reactor before the self-destruct timer. | timer objective, Mech enemy |
| 9 | **Luna Siege** | Luna | Break the fortress ring; **[BOARD]** the dreadnought *Bulwark*, kill Admiral-Prefect Halden. | big fleets, Medic/Gunner mix |
| 10 | **Ragnarök Over Earth** | Earth Orbit | Final fleet clash; board *Aegis Prime*; fight to the bridge; duel Prefect Voss (boss). | boss fight, ending |

### Endings (one choice, no branching campaign)
After Voss falls the Captain either **Break the Armada** (destroy fleet codes: Earth and Mars
equal, uneasy peace) or **Take the Throne** (use the codes: Mars rules). Only a text epilogue +
credits difference — keeps scope small.

## Saga verses (flavour pool, shown on debrief)

Short alliterative lines, 2–3 per mission, e.g.:
*"Iron wakes on the red hill; ravens ride the reactor's breath."*
Written in `data/saga.rs` (T25) — each mission has a win verse and a loss verse.

## Tone rules

- Grim humour, never grimdark gore (boarding is arcade-violent; no realistic blood — sparks and
  robotic-looking Minion armour; "gibs" are armour plates).
- Martians speak in short clipped sentences, kennings ("whale-road" for space, "iron-bird" for
  ship). Concord speaks in bureaucratic politeness.
- Every briefing ≤ 6 lines; skippable with Enter.
