# Design: harbours

Every boat on the island: the harbours, the ships at sea, the boats on the lakes and the ferry, how they float, how
they are fought, and the jobs they bring. The rules are `cb-core::ships` (pure, seeded, tested); `cb-app::boats`
draws them and wires them to the world. Tuning is `data/world/ships.ron`; the gunboat's numbers are a vehicle's, in
`data/world/units.ron`; the shipping jobs are three more kinds in `data/jobs/jobs.ron`.

## 1. Harbours (`ships::harbour`)

Placed once from the world plan, after the cities and bases, so the plan and its golden fingerprint do not change.

- **Which cities.** A city is coastal when water `berth_depth_m` deep lies within `coast_reach_m` (1 800 m) of its
  square's edge. Coastal cities are taken largest first, each kept at least `spacing_m` (3 000 m) from the harbours
  already placed, up to `most` (four: HB2).
- **Where on the shore.** From the city's centre, 48 bearings are walked outward in 20 m steps until the ground
  drops below sea level (the shoreline), then on until it is `berth_depth_m` (10 m) deep (the berth). The bearing
  with the shortest shore-to-berth run within `pier_max_m` (450 m) wins, ties to the lower bearing; a bearing whose
  shore lies within `base_clear_m` (600 m) of an air base's site is skipped.
- **What is built.** Nothing is dug or flattened: the quay stands on piles, so the terrain is the plan's.
  - The **quay**: a concrete deck `deck_m` (2.5 m) above sea level, `quay_m.x` (220 m) along the shore and running
    from `quay_m.y` (40 m) inland of the shoreline out to the berth: walkable, as a fixed platform
    (`world::platform`, the Hermit's deck rule).
  - **Cranes**: `cranes` (3) luffing cranes along the berth edge, 40 m tall on a 12 m portal.
  - **Warehouses**: `warehouses` (3) sheds behind the quay, 50 x 24 x 12 m, footed on the lowest ground under them.
  - The **harbour office**: a 16 x 12 x 9 m block at the landward corner of the quay, with a terminal under its
    awning, 3 m in front of its door.
  - The **roads** (anchorage): a point `anchorage_m` (900 m) out from the berth along the same bearing, where
    `at_anchor` (2) ships ride.
  - The **mouth**: where ships turn in, `approach_m` (1 200 m) out from the berth.
- **The office's terminal** is the air bases' terminal (6.1): `world::bases::Terminal` gains an `office` field
  (the harbour's id) and the same reach and look rule finds it. Its screen is the menu's TERMINAL page with only the
  JOBS tab: the career's one board, so every shipping job is also in the mail.

## 2. Sea lanes (`ships::lanes`)

- A **sea grid** of `cell_m` (200 m) cells over the world. A cell is open water when the ground at its centre and
  at four points `coast_clear_m` (300 m) out is below sea level less `lane_depth_m` (12 m): the coast and every
  islet are kept clear by that margin.
- **Lanes** are A* paths over the grid (8 neighbours, a diagonal costs the square root of two), from each harbour's
  mouth: one to every other harbour's mouth, and one out to the **offing**, the point `offing_m` (17 000 m) from
  the island's centre on the harbour's own bearing outward (past the horizon at sea level, inside the world's edge
  fade). A route that finds no water is left out. Paths are pulled straight where the line between two of their
  points stays on open water (string pulling), so ships run in long legs, not stair steps. A lane runs on from the
  mouth to the berth, the last leg a ship takes slowly.
- **Two-way traffic** keeps to the right: a ship on a lane steers for the point `keep_right_m` (80 m) to the right
  of the lane's line.
- Each lane is a polyline with its length and its two ends, deterministic from the plan.

## 3. Boats (`ships::boat`)

```text
Boat { id, kind, job, pos, heading_rad, speed_m_s, hp, phase, crew, flee_s, flee_from, harmed_by_pilot, ... }
Phase: Underway | Moored { left_s } | Anchored | Fleeing | Sinking { t_s } | Sunk { t_s }
Job:   Lane { lane, s_m, forward } | Fish { ground } | Loop { points, next } | Ferry { lane, forward } | Anchor { at }
```

- **Kinds** (true sizes, `ships.ron`): freighter 120 x 18 m, 26 m to the masthead, 7 m draught, 7 m/s; tanker
  150 x 24 m, 30 m, 9 m, 6.5 m/s; fishing boat 22 x 6.5 m, 10 m, 2.8 m, 4.5 m/s; rowing boat 4.2 x 1.4 m, 1.1 m,
  0.25 m, 1.3 m/s; launch 8 x 2.6 m, 2.8 m, 0.8 m, 8 m/s; ferry 60 x 14 m, 16 m, 3.2 m, 7 m/s. Each has its hit
  points, armour, crew, turn rate and acceleration.
- **Following a route** (pure pursuit): the boat steers for the point `lookahead_m` (its length plus 60 m) ahead of
  it along its route, shifted `keep_right_m` to the right, turning at its kind's turn rate, accelerating at its rate.
- **Keeping clear of the coast**: a boat probes the water `probe_s` (20 s) of travel ahead, and 30 degrees either
  side; ground shallower than its draught plus `keel_clear_m` (2 m) ahead turns it toward the deeper side and slows
  it by half.
- **Keeping clear of each other**: a boat slows to match a boat ahead within `gap_lengths` (4) of its own length in
  a 30 degree cone, and stops if one is within its own length; the higher id yields, so two never both wait.
- **Slowing for harbour**: within `approach_m` of its route's harbour end a boat slows linearly to `berth_m_s`
  (1 m/s) at the berth, then moors there for `moor_s` (120-240 s) and sails the route back. At the offing it is
  gone (past the horizon) and its slot comes round again.
- **Fishing boats** run out to their **ground**, a point 2-4 km off their harbour on open water, work it at 1.5 m/s
  in a 300 m circle for 300-600 s, and run home.
- **Anchored ships** hold their position, heading slowly round to the swell's main heading.
- **Lake boats** row or motor a loop of 5 points inside 70 % of their lake's radius, on water deeper than their
  draught; rowing boats at 1.3 m/s, launches at 6 m/s.
- **The ferry** shuttles between the harbour's berth and the islet's landing on a lane of its own, waiting `moor_s`
  at each end.

## 4. Floating (`ships::float`)

- A hull rides the water surface the world gives (`Waters::water_at`: the sea, or its lake's level), so a boat on a
  lake floats at the lake's level, not the sea's.
- It heaves, pitches and rolls on the swell the water already draws (`water_look.ron`'s three swell trains). The
  swell's height at a point and time is `sum_i h_i sin(k_i (p . d_i) - w_i t)` with `k_i = 2 pi / L_i` and
  `w_i = sqrt(g k_i)` (deep water). The hull samples it at its bow, stern, port and starboard: heave is their mean,
  pitch the bow-stern difference over the length, roll the port-starboard difference over the beam. A long hull
  averages short waves away, so a freighter barely moves and a rowing boat bobs.
- The shader bends only the normals by the swell (the surface stays flat), so a hull's heave of a few tenths of a
  metre reads as the boat riding the waves without cutting through a flat sheet.
- **Wake**: a moving boat within 500 m of the eye throws the existing `skate_spray` effect at its bow every 0.4 s,
  scaled by speed. There is no ripple height field (`water-ripples` is not built), so nothing else.

## 5. The fight

- **Hits**: each boat afloat is one capsule along its hull in the weapons' part list (ids from 20 000), hit by
  `weapons::fly` like a vehicle. A hit takes damage by the vehicles' rule, `units::armour_hit`: damage only when the
  round's penetration beats the hull's armour.
- **Sinking**: at zero hit points a boat sinks over its kind's `sink_s` (freighter 45 s, rowing boat 6 s): it lists
  to `list_deg` (25 degrees), settles by the stern, and goes down until its deck is under at `sink_s`. A tanker's
  sinking also throws a fire. It then lies as a **wreck** on the bottom (its keel on the bed, its masts above the
  water where the bottom is shallow) for `wreck_s` (600 s), and leaves an **oil slick**, a dark disc growing to its
  length times 1.5 and fading over `slick_s` (300 s).
- **Shoving and swamping**: a mech wading or skimming whose body (its hit radius) touches a boat no longer than
  `shove_max_m` (25 m) pushes it out of the way at the mech's speed. When they meet faster than `swamp_m_s`
  (5 m/s) the boat is swamped and sinks. A swamping by the pilot's mech counts as the pilot's harm.
- **Fleeing**, as the cars do (`car-traffic`): an explosion within `flee_blast_m` (600 m), a hostile within
  `flee_hostile_m` (400 m), or a hit sets a boat fleeing for `flee_s` (60 s): it turns away from the threat and
  runs at its best speed, still keeping clear of the coast, then steers back onto its route. Anchored ships slip
  their cable and run; moored ones stay moored.
- **Crabs attack ships**: an enemy mech that is not a defence raider takes a crewed ship within `crab_reach_m`
  (700 m) as its target point when the pilot is more than `crab_pilot_m` (1 500 m) away, and fires on it as it
  shells a city. The nearest ship wins, ties to the lower id.
- **Gunboats** are vehicles (`units`), kind `Gunboat`: they ride the sea's surface, close on their target to their
  standoff, circle it there and fire when nosed on, and turn away from water shallower than their draught. Their
  numbers are in `units.ron`: 30 x 6.5 m, 14 m/s, 260 hit points, 30 mm of armour, a 40 mm gun. A job's gunboats
  fight the job's ward (the escort's freighter) when it has one, else the pilot.

## 6. Standing (`ships::harm`)

The same rule as cars (W37: "Only cars with a driver in them, harmed by your fire, feet or car"), civilians (W42:
"Only your own rounds, blasts and mech feet") and civil aircraft (W34): harm costs standing only when

- the boat is **crewed** (a wreck is not, nor a boat whose crew is zero), and
- the harm is the **pilot's own**: a round or blast the pilot's mech fired, or the pilot's mech shoving or swamping
  it. Enemy fire, crabs and the gunboats cost nothing.

It is reported once when the pilot first harms a boat and once more if the pilot's harm sinks it, as the message
`boats::BoatHarmed { boat, kind, crew, base, at, sunk }`, where `base` is the nearest air base to the boat. The
lead wires it into career standing (`career-progression`); this change only reports it.

## 7. Shipping jobs

Three more kinds in `data/jobs/jobs.ron`, through the one job generator and the one board (`cb-core::jobs`, 6.1):

| Kind | Site | Enemies | Won when | Lost when |
| --- | --- | --- | --- | --- |
| `Escort` | the freighter's start, `escort_out_m` (2 500 m) out along a lane from a harbour's mouth | gunboats, coming for the freighter | the freighter is moored and no gunboat is left | the freighter sinks, or the clock |
| `Raid` (sink the gunboats) | a harbour's fishing ground | gunboats, attacking the fishing boats | every gunboat is sunk | the clock |
| `Blockade` | a harbour's mouth | gunboats lying across the mouth | every gunboat is sunk | the clock |

- A kind marked `at_sea: true` is sited at a harbour, drawn from the harbours as the land kinds are drawn from the
  cities; with no harbour it is never offered. Its `mechs` count is its gunboats.
- The board steps as before: `enemies_left` is the job's gunboats left, plus one while an escort's freighter has
  not moored. A sunk ward fails the job through `Board::fail`.
- Tiers and pay follow the other kinds (tiers 1 to 3; tier 1 is 2-3 gunboats).

## 8. Spawn and despawn round the pilot (`ships::traffic`)

The traffic is a timetable: each lane carries ships `headway_m` (3 000-5 000 m, drawn per lane) apart, each slot's
place at time `t` fixed by the lane's seed. A slot becomes a real boat when its timetabled place comes within
`spawn_m` (6 000 m) of the pilot, and stops being one past `despawn_m` (8 000 m) unless it is sinking, a wreck or a
job's ward. A slot whose boat sank stays empty. The same seed and the same pilot path give the same boats in the
same places. Fishing boats, anchored ships, lake boats and the ferry are slots of their own under the same rule
(lake boats within `lake_spawn_m`, 3 000 m).

## 9. Models (`tools/blender/build_boats.py`)

Block-outs at true size, built the CSG way (`blender-hard-surface`) and exported to `assets/models/boats/<id>.glb`
with the manifest `assets/models/boats.json` (bounds measured off the export, triangles against a budget per LOD).
LOD 0 is the block-out; LOD 1 drops the superstructure detail. Flat PBR colours, labelled stand-ins until the
detail pass. `cb-tools validate` refuses a boat whose model is missing, over budget, or whose measured size is more
than 5 % off its data. The mockup page `docs/mockups/boats.html` (from `tools/mockups/gen_boats.py`) shows them all
orbitable at true scale beside the pilot (1.8 m) and the 24 m Foreman.

## 10. Cost

| Item | Cost |
| --- | --- |
| Harbour placement and lanes | once at load, measured by `cb-tools ships` |
| Simulation | boats real within 6 km: about 30-60, each a pursuit step and three water probes; measured by `cb-tools ships` (section 11) |
| Drawing | one glb scene a boat (block-outs of 100-2 000 triangles), about 60 boats: under 100 000 triangles and about 300 draws; each harbour is two meshes |
| Memory | lanes under 100 KB; boat models under 2 MB |

## 11. Measured

Measured with `cargo run --release -p cb-tools -- ships 42` (seed 42, the shipped data); filled in when built.

## Pre-approved choices

The owner, 2026-10-26: "You also forgot boats, I'll take whatever you recommend for that", then "No survey needed qs
preapprove all boat choices". Every choice below is the recommendation, pre-approved by the owner, 2026-10-26.

| Id | Choice | Pre-approved recommendation |
| --- | --- | --- |
| HB1 | Can the pilot board a ship on foot | Not in this change: ships are scenery and targets; boarding comes with mariners and decks |
| HB2 | How many harbours | One per coastal city, largest first, 3 km apart, at most four |
| HB3 | How a harbour meets the shore | A quay deck on piles out to berth depth; nothing is dug or flattened |
| HB4 | The harbour office's terminal | The air bases' terminal, JOBS tab only, the career's one board |
| HB5 | Sea lanes | A* over 200 m sea cells, 300 m clear of land and 12 m deep, string pulled; harbour to harbour and out to the offing at 17 km |
| HB6 | Which ships at sea | Freighters and tankers on the lanes, three fishing boats a harbour, two ships at anchor a harbour |
| HB7 | Boat sizes | Real 1980s sizes: 120 m freighter, 150 m tanker, 22 m trawler, 4.2 m rowing boat, 8 m launch, 60 m ferry, 30 m gunboat |
| HB8 | Inland water | Rowing boats and launches on the lakes; no river boats while rivers are off; the ferry runs to an islet |
| HB9 | Floating | Ride the water's real surface; heave, pitch and roll on the water's own swell |
| HB10 | Wake | Bow spray from the existing effect; no wake in the water until `water-ripples` is built |
| HB11 | Hits | One capsule a boat, the vehicles' armour rule (`units::armour_hit`) |
| HB12 | Sinking | List 25 degrees and go down over 6-45 s by size; a wreck on the bottom for 10 min and an oil slick for 5 min |
| HB13 | Mech against small boats | A wading or skimming mech shoves boats up to 25 m long, and swamps them meeting faster than 5 m/s |
| HB14 | Fleeing | Blasts within 600 m, hostiles within 400 m or a hit: run from the threat for 60 s, then rejoin the route |
| HB15 | Crabs and ships | Crabs go for a crewed ship within 700 m when the pilot is over 1.5 km away |
| HB16 | Standing | Only crewed boats, only the pilot's own fire, blasts or mech; reported once on first harm and once on sinking, to the nearest base |
| HB17 | Shipping jobs | Escort, sink the gunboats, clear a blockade; tiers 1-3; gunboats are enemy vehicles |
| HB18 | Spawning | A seeded timetable; real boats within 6 km of the pilot, gone past 8 km |
| HB19 | Models | Labelled CSG block-outs at true size, two LODs, flat PBR colours, until a detail pass |

## Open questions

None: every boat choice is pre-approved (above).
