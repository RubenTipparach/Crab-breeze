# Design: civilians

## 1. Simulation (`cb-core::civilians`, pure)

- People live in tiles round the pilot (as trees do, `Forest::keep_around`): spawned from the street pieces of the
  city blocks within 400 m, kept while within 600 m, gone beyond.
- Each walks a route of pavement points; a fixed step moves them; a danger event (a blast, a shot, a mech's footfall
  within a radius) turns them to flee for a time.
- Greeting: a person within 2.5 m in front of the pilot, Use pressed: the person stops and faces the pilot; a line is
  picked from `data/people/lines.ron` by place and seed.

### 1.1 As built (`crates/cb-core/src/civilians/`, tuning in `data/people/people.ron` and `lines.ron`)

- **Homes and density.** A home is a city block (its pavement, or its park) or an air base's apron. A home within
  400 m of the pilot is peopled, nearest first, at most 60 people a fixed step and 2,000 in all; a person past
  600 m is let go, and a home past 600 m may be peopled again on the pilot's return (a second visit draws other
  people). The pilot must move 8 m before the homes are looked at again. A pavement holds `8 + (1.5 - 8) d^1.5`
  people per 100 m of its loop, `d` the block's distance from the centre over the city's half size: a 90 m block's
  340 m loop holds about 27 downtown and 5 at the edge. A park holds 12 a hectare at the heart's rate, scaled the
  same way; an apron 24.
- **Routes.** Every route is a loop of eight waypoints (corners and midpoints) round a rectangle: the middle of a
  block's pavement (2 m in from the kerb), a park path 8 m inside the kerb, or an 18 m deep walk in front of a
  base's hangars. At a pavement corner a third of walkers cross: they stand at the kerb 2-9 s, then walk straight
  across to the corner of the block opposite in the same grid row or column. Two in five sit on a park bench they
  pass (one at each side's midpoint of the path, 1.2 m off it, facing it) for 20-120 s. One in seven stops 3-15 s
  at a pavement's midpoint or an apron's waypoint. Walk 1.1-1.7 m/s, run 3.5-5.5 m/s, stand 1.55-1.95 m. Feet
  are on the kerb (0.2 m up) on a block and on the road while crossing.
- **Danger.** A round's flight each step, a blast, and a mech's footfall are dangers. A round within 35 m, a blast
  within 30 m per cube root of its megajoules (48 m for a 4 MJ shell) or a foot within 70 m sends people running
  for 10-20 s: on a pavement along the pavement's own line away from it (never across a block into its
  buildings), in a park or on an apron straight away. A round within 0.9 m of the body (feet to head), a blast
  within 5 m per cube root of its megajoules (8 m for a 4 MJ shell) or a foot within 4 m kills. The struck lie still
  30 s and are cleared away.
- **Running inside.** `Doors::refuge(p, danger, reach)` names the street door a person runs for: the nearest within
  60 m that the danger is not nearer to. The building-entrances change (PR #73, not on main) has the doors; until it
  lands the game passes `NoDoors` and everyone flees down the street. A person who reaches a door goes inside and
  leaves the world.
- **Greeting.** Use on foot greets the nearest standing, unfrightened person within 2.5 m and 50 degrees either side
  of the view; they stop and face the pilot for 6 s. The answer is one of their role's lines, picked by their id and
  how many times they have been greeted. Roles live on one kind of place (passer-by, shopkeeper and dock worker on
  pavements; park-goer and pensioner in parks; mechanic, ground crew and quartermaster on aprons). A line marked
  `job` names `{job}` and is only said while a job offer is open, filled with its subject.
- **The toll (owner, survey W34).** Each death says what caused it (round, blast or foot) and whose it was. A death
  the pilot's own mech caused is counted against the air base nearest it (`Toll::by_base`) and sent to the game as
  a `CivilianLoss` message; the career's standing per base (its own branch) reads either and decides the cost.
  Deaths the enemy causes are counted apart and cost nothing.
- **Determinism.** Each person draws from their own stream, forked from the world seed by home, visit and place in
  it; the list is kept in the order people came, and walked in it. A golden test pins seed 42's people.
- **Cost.** Measured on the CPU in release (a measurement instrument, `civilians::tests::cost_of_a_step`, on the
  development Mac, not the 3070): 2,000 people, a step with a blast and twenty rounds every second, p50 0.019 ms,
  p95 0.025 ms, p99 0.045 ms per fixed step.

## 2. Drawing

Near (under 60 m): the rigged humanoid, walking or idle. Mid (60-250 m): a low-poly LOD. Far: instanced impostor
cards. Budget: 200 near, 2,000 in all, measured on the 3070.

### 2.1 As built, with stand-ins (`crates/cb-app/src/civilians.rs`)

The humanoid pipeline's packs are not on the development machine, so the bodies are **stand-ins**: box figures built
in code at true human size (modelled 1.75 m, scaled to each person's 1.55-1.95 m), 120 triangles, in eight clothing
palettes and four poses (standing, two strides, sitting), laid flat when struck. They are reviewed orbitable beside
the pilot's 1.8 m and the mech's 24 m in `docs/mockups/civilians.html` (its geometry written from the game's own
meshes by a test that fails while it is stale; `docs/screenshots/civilians/standins-overview.png`).

- Near (`draw.near_m`, 60 m): a pool of 200 entities, one shared material, posed each frame (a stride by distance
  walked). 200 x 120 = 24,000 triangles at most; each body casts shadows.
- Mid and far: one merged mesh of everyone else, rebuilt 30 times a second and whenever someone crosses 60 m: a
  three-box figure to 250 m (60 vertices), a card beyond (4). No shadows. One draw. Building it for 2,000 people
  (873 mid, 1,127 far, 56,888 vertices) took p50 0.59 ms, p95 0.90 ms on the development Mac
  (`civilians::tests::cost_of_the_crowd_mesh`, release): about 0.3 ms a frame at 60 fps.
- Park benches: every park's four, in one mesh at startup.
- `CB_NO_PEOPLE=1` leaves the plugin out, for the benchmark's comparison. No frame-time number exists yet: CLAUDE.md
  section 8 forbids running the game without the owner's word, so the 3070 measurement is task 2.2's.

## Open questions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| CV1 | Can civilians be hurt | yes: the owner decided (survey W34) that the pilot's fire hitting them costs standing; how it is shown is W41 | decided |
| CV2 | Voiced lines or text | text first, voice later | provisional |
| W40 | Use near a person and a stopped mech at once | the person within 2.5 m is greeted; boarding needs Use away from people | provisional (survey) |
| W41 | How someone struck down is shown | falls and lies still 30 s, then cleared; no blood | provisional (survey) |
| W42 | Which deaths cost standing | the pilot's own rounds, blasts and feet only | provisional (survey) |
| W43 | Bodies before the humanoid pipeline runs here | labelled stand-ins at true size | provisional (survey) |
| W44 | How busy the streets are | 8 per 100 m of pavement downtown, 2,000 at most | provisional (survey) |
