# Design: car-traffic

## 1. The lane graph (`cb-core::world::lanes`, pure)

Built once from the world plan's `StreetPiece`s and `Highway`s:
- **Streets:** each piece becomes two lanes, one each way, 3.5 m from its centre line. Arterials get
  two lanes each way.
- **Highways:** two lanes each way.
- **Nodes:** a junction is wherever two pieces' centre lines cross or meet. Lanes connect through it:
  straight on, left and right.
- **Highways meet cities** at the edge of the city square, joined to the nearest arterial.

The graph is deterministic from the plan (stable ids in plan order), with a test that every lane has
a way out, so a car never dead-ends.

## 2. Cars (`cb-core::traffic`, pure, fixed step)

```text
Car { id: u32, kind: CarKind, lane: u32, s_m: f32 /* along the lane */, speed_m_s: f32,
      state: Driving | Stopping | Stopped | Crushed | Burning, next: u32 /* lane after the node */ }
```

- **Following:** the intelligent driver model, which accelerates to the lane's limit and keeps a
  2 s gap to the car ahead. Its constants are in `data/world/traffic.ron`.
- **Junctions:** right of way by arrival order at the node. A car waits while a crossing lane's car
  is within 15 m of the node, and the order is stable by car id.
- **Turns:** the next lane is chosen at the node by a `SimRng` forked from the car's id and the node,
  so a replay drives the same.
- **Fear:**
  - within 120 m of an explosion, or 60 m of a moving mech, a car brakes at 7 m/s2 and stays
    stopped for 20-40 s;
  - a car in the mech's path that cannot stop is pushed aside and damaged.
- **Crushing:** a mech's foot strike (`foot_strikes`, the one footfall rule) whose foot covers a car
  crushes it. The footprint and dust are as today.
- **Hits:** cars carry a capsule in the weapons' part list, like vehicles (invasion-forces). A hit
  that beats their 5 mm of armour sets them Burning.
- **Spawn and despawn:**
  - Lanes within 1 km of the eye (1.5 km until the owner, 2026-10-26: "car should be 1000m") are kept filled to their density, on lane positions not in the
    view frustum, or beyond 400 m and behind cover.
  - A car is placed only on the stretch of its lane that is in reach. It used to be drawn along the whole
    lane and thrown away when out of reach, with three tries per car wanted; a highway running far past the
    ring then filled to about half its density (4 cars for 8 inside 1 km), which the 1.5 km ring had hidden.
  - Cars beyond 1.2 km are dropped.
  - Wrecks (Crushed, Burning) stay for 10 minutes or until 2 km away.

## 3. Drawing (`cb-app::traffic`)

- **Instancing:** one instanced draw per car kind and LOD (gpu-instancing), with the transform
  written each frame from the interpolated lane position. Wheels turn in the shader from the speed.
- **Models:** `tools/blender/build_cars.py`, CSG hard surface:
  - 7 kinds, 3 paint colours each from a palette;
  - LOD 0 (within 80 m) about 6 000 triangles, LOD 1 (to 400 m) 1 200, LOD 2 (beyond) 200;
  - a crushed variant per kind, about 2 000 triangles;
  - headlights and tail lights as emission, lit at dusk.
- **Materials:** Material Maker car paint (base, clear-coat roughness), chrome, glass, tyre rubber;
  full PBR sets.

## 4. Numbers (survey J6)

| Item | Value |
| --- | --- |
| City lanes | one car per 60 m |
| Highway lanes | one car per 300 m |
| Cars within 1.5 km of downtown | about 150-250 |
| City speed | 40-60 km/h |
| Highway speed | 90-110 km/h |

## 5. Cost

| Item | Cost |
| --- | --- |
| Simulation | 250 cars x a following step and a node check: about 0.05 ms a fixed step (measured when built) |
| Drawing | 7 kinds x 3 LODs, about 21 instanced draws; about 250 x 1 200 = 300 000 triangles worst case downtown |
| Memory | under 1 MB of lane graph; car meshes about 6 MB; paint sets about 40 MB at 2048 px |

Measured downtown at rush density on the 3070 before it is called done.

## 6. As built (2026-10-26)

What landed, where it differs from the plan above, and what it measured.

- **Lanes** (`cb-core::world::lanes`): each city's street lines become a junction grid; every block-long piece
  is a road with lanes each way at the offsets in `traffic.ron` (keep right); a highway is one road from its
  city's nearest junction to the other's. Lanes stop short of each junction by its half width, and connector
  lanes (cubic curves) cross it into every other road's lane in the same slot or the nearest. 9 278 lanes and
  584 junctions on seed 42, built in 14 ms.
- **Junctions:** in place of "wait while a crossing car is within 15 m", two connectors at one junction
  *conflict* when they pass within `conflict_m` (2.5 m) or lead into one lane. A waiting car is given its
  turn, earliest arrival first (ties to the lower id), when its connector meets no connector a car is on or
  has been given, none an earlier car still waits for, and the lane beyond has room for it. A car that cannot
  stop in time stops at the line, never inside the box.
- **Traffic lights on the arterials** (survey W39, answered 2026-10-26: "Lights on the arterials"). A junction
  where an arterial meets at least two other roads is lit (`Node::signal`): 257 of the 584 junctions on seed
  42. Each road lane into it carries a phase (`Lane::phase`): 0 for the roads within 45 degrees of its first
  arterial, 1 for the rest. The two phases take turns on one clock, `traffic::light` (`lights` in
  `traffic.ron`: 24 s green, 3 s amber, 2 s red both ways, a 58 s cycle), offset per junction by its id times
  the golden ratio so neighbours do not all change together. A car whose light is red does not ask for a turn:
  it stops at the line and holds nobody up; on amber it goes only when it could not stop at 2.5 m/s2
  (`idm_brake_m_s2`) short of the line; on green it takes its turn by the rule above, so a turn across oncoming
  traffic still gives way. Every other junction keeps taking turns by arrival. With the lights came one fix to
  the turn rule for every junction: only the front car of a lane asks for its turn (a car behind it given a
  turn held a path the car in front needed, and the two stood for good; the lights' red made it common).
  A second fix (2026-10-27, after the 1 km ring): the room on the lane past a junction is shared out in
  arrival order, and a car held at red keeps its place in that order, so the space it needs is kept for it.
  Before, a red car asked for nothing, and on seed 9 a street car turning onto a crawling arterial stood
  174 s: every gap that opened there went to the arterial's own cars on their green, and on the street's
  green there was never room (`a_car_held_at_red_keeps_its_place_for_the_room_on_the_lane_it_turns_into`).
  The game draws a block-out head (pole, housing, three lenses: `cb-app::traffic_lights`) on the kerb of every
  road into a lit junction, 882 heads on seed 42, shown within 700 m, its lenses lit from the same
  `traffic::light`. Tested: lights only where an arterial meets two other roads, the phases never open
  together, no car given a lit junction on red and cars waiting at the line on red, and ten minutes of the
  town with every minute's crossings and no car standing more than three cycles. Measured again with the
  lights (`traffic-bench 42 3600`, same Mac): p50 0.125 ms, p95 0.203 ms, p99 0.260 ms a fixed step.
- **Fear** is two fields on a driving car (`fear_s`, `hailed`) rather than `Stopping` and `Stopped` states:
  the car brakes at 7 m/s2 while either holds. A mech that walks into a car shoves it off its lane as a free
  body (`car::CarBody`, the driven car's own physics) and costs it 0.4 health.
- **Wrecks** keep their lane position, so cars queue behind them (survey W38).
- **Harm** to any car is reported with `by_player` and `occupied` (`traffic::Harm`); `cb-app` publishes it
  as `CivilianHarmed` and counts it in `CivilianToll`, the hook for the career's standing per base (W34,
  career-loop, survey W37).
- **Drawing:** the block-outs (`tools/blender/build_cars.py`, 428-468 triangles each, approved by the owner on
  2026-10-26: "Approved all of the stand ins", survey W36) are drawn per car from their glb; Bevy batches the
  shared meshes per kind. A crushed car is its
  block-out at 35 % height, a burning one carries the `burning` stream; no LODs, no crushed models and no PBR
  sets yet, and no paint variety on screen (each block-out is its kind's first paint).
- **Density:** survey J6's numbers fill the 1.5 km round the capital's start with about 980 driving and 210
  parked cars on seed 42, not the 150-250 section 4 estimated (survey W35).
- **Measured** (`cargo run --release -p cb-tools -- traffic-bench 42 3600`, Apple M4 Max, CPU only,
  headless, not a frame time): 1 189 cars round the start, the traffic step and its refill p50 0.134 ms, p95
  0.198 ms, p99 0.231 ms a fixed step; the first step reads each lane's road height once (a 9 ms step behind
  the loading screen). Frame time with the cars drawn is not measured: that needs the 3070 (task 3.1).
- **Height and the road** (2026-10-26, owner: "cars are a little too short I think lol"). Measured in the game
  (`CB_STILLS_CARS=1 --stills <dir>`, `cb-app::capture_cars`): the glbs were true to their data (sedan 1.40 m,
  bus 3.10 m, no scale on any node), but every car stood on the bare grade while the road is drawn `road_lift_m`
  (0.15 m) over it, so each sat sunk to its hubs, a sedan showing 1.25 m over the asphalt and its wheels cut off.
  Cars now ride on the drawn road (`world::OnRoads`, `WorldPlan::road_top_at`, a crater in the road still lowering
  it), and every car is raised by `car_height_scale` (1.18, `traffic.ron`): body, roof, wheels, ride height and
  the driver's eye, length and width kept (`CarSpec::raised` on load, the same number in `build_cars.py`;
  `cb-tools validate` refuses a model built at another height). A sedan is now 1.65 m, the bus 3.66 m. Stills:
  `docs/screenshots/cars/before-*.png` and `after-*.png`. The pilot on foot stood on the bare grade too, 0.15 m
  under the drawn road and 0.2 m under the pavement (their eye 1.55 m over the asphalt); the walking rule now stands
  every body on the drawn surface (`WorldPlan::walk_top_at` in `Scene::support_at`, the people's feet by the same
  `City::surface_top`), so the eye is 1.70 m over the road and a sedan's roof 1.65 m.
- **Cost at the capital's density, and stepping far cars less often** (2026-10-28, after the desktop's city run on the
  RTX 3070, `docs/benchmarks/2026-10-28-city/report.md` on `perf-probe`: `step_traffic` about 4.2 ms a fixed step for
  about 1 100 cars on its slow frames, twice that when a slow frame runs two steps). `cb-tools traffic-bench` now steps
  the traffic as the game does (the wheels on the drawn road over the live `Scene`, the refill kept out of the view cone
  and behind cover through the same `traffic::in_cone` and `traffic::sight_blocked` the game calls) from three eyes:
  standing on foot, a mech striding at 8 m/s and skating at 20 m/s along the streets from the start, frightening and
  shoving cars as in the game. `Traffic::cost` (`StepCost`) times each part of a step. Measured on the Mac (Apple M4
  Max, CPU only, headless, release, seed 42, old and new interleaved old, new, new, old, 3 600 steps, means per step):

  | Part | Standing, before | after | Skating, before | after |
  | --- | ---: | ---: | ---: | ---: |
  | Each lane's cars (map, sorted) | 0.017 | 0.012 | 0.017 | 0.013 |
  | Junctions (right of way) | 0.023 | 0.020 | 0.023 | 0.020 |
  | Following (leaders, driver model) | 0.027 | 0.019 | 0.033 | 0.024 |
  | Placing on the road | 0.012 | 0.006 | 0.013 | 0.006 |
  | Loose bodies (shoved cars, wrecks) | 0.001 | 0.000 | 0.180 | 0.025 |
  | Refill | 0.049 | 0.010 | 0.038 | 0.008 |
  | **Step and refill, mean** | **0.146** | **0.073** | **0.324** | **0.103** |
  | p95 | 0.201 | 0.113 | 0.499 | 0.163 |
  | p99 | 0.246 | 0.126 | 0.555 | 0.200 |

  About 1 000 cars round the start. The largest cost was not the car following but the loose bodies: every car a mech
  shoves off its lane stays a free body for ten minutes, and each was stepped every fixed step (its wheels on the
  ground, its hull against the walls), so the cost grew the longer a mech walked the streets. Three minutes of skating
  (10 800 steps, about 1 250 cars, about 100 loose): before 0.776 ms mean, p95 1.480; after 0.148 ms, p95 0.233. The
  game on the Mac reads `car_traffic` 0.2 to 0.4 ms a frame, about three times the bench, so the desktop's 4 ms is
  consistent with that growth on a slower CPU. What changed:
  - **Far cars are stepped less often** (`step_lod` in `traffic.ron`): within 300 m of the eye every fixed step, to
    600 m every 2nd, beyond every 4th, each with the time it was owed (`Car::owed_s`), its turn counted from its id so
    the far cars' steps are spread evenly. Right of way is still settled for every car every step, so a far car's
    grant and the lights hold as before; a car stepped late sees the others where they are now, never nearer than they
    are, and it never moves past the gap it measured ahead (the car in front or the stop line). A car is drawn gliding
    between its own steps (`Car::drawn`), so a far car stepped every fourth step does not jump.
  - **A loose body at rest sleeps** while no mech or person is within 20 m, no other body is moving within 15 m and no
    blast went off within `fear_blast_m` since the last step.
  - **The lanes are refilled every 4th fixed step** (`refill_every_steps`), with the 4 cars a step owed since the last
    (`Traffic::refill`).
  - The per-step maps were rebuilt as sorted lists (each lane's cars, the junctions' waiting cars, the refill's lane
    counts), each lane's share of a car is worked out once, and a car that did not move is not placed again.
  Only the stepping by distance changes an outcome: with `step_lod` at 1 the seed 11 traffic is the old golden to the
  bit; the golden was re-recorded for it (455 cars, as before). The no-deadlock, red-light, fill and following tests
  pass unchanged. The game's timing guard is now `car_traffic` (it was `step_traffic`, the same name as the air
  traffic's, so the two were summed in the hitch reports). The desktop has not measured it yet.

## Open questions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| J6 | Traffic density | One car per 60 m in cities, 300 m on highways, within 1.5 km | decided (survey J6) |
| J7 | Which cars | 1980s civilian, no real brands: sedan, wagon, pickup, van, taxi, bus, semi | decided (survey J7) |
| W35 | J6 gives about 1 200 cars round the capital, not 150-250: keep it, thin it, or cap it? | Keep J6 until the 3070 measures the drawing | decided: the recommendation, accepted by the owner, 2026-10-26 (survey) |
| W36 | Are the seven block-outs right to model in detail? | Approve as they are | decided: approved, 2026-10-26 ("Approved all of the stand ins"); detailed models later |
| W37 | Which harm costs standing with the nearest base? | Only cars with a driver in them, by the player's fire, feet or car | decided: the recommendation, accepted by the owner, 2026-10-26 (survey) |
| W38 | What do cars do behind a wreck in their lane? | Queue until it is cleared | decided: the recommendation, accepted by the owner, 2026-10-26 (survey) |
| W39 | Junctions: take turns by arrival, or 1980s traffic lights on arterials? | Take turns by arrival | answered by the owner, 2026-10-26 (survey): "Lights on the arterials"; every other junction keeps taking turns by arrival; built (section 6, tested in cb-core) |
