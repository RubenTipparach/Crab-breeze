# Design: building-type-lods

## 1. One look per building

`cb_core::world::dressing::look_of(storeys, area_m2, rng)` is private today and called only by the
near dressing. It becomes `pub fn look_for(building: &Building, seed: &SimRng) -> Look` (with `Look`
and its fields public), forking the same stream the dressing uses, so near and far read one answer:
its `Style` and `punched_grid` (the facade kind), and its `brick`, `concrete`, `glass`, `frame` and
`mullion` colours. A test checks the dressing and the far look agree for every building of seed 42.

The facade kinds today, from that answer:

| Kind | From | Bay x storey | Far features |
| --- | --- | --- | --- |
| walk-up | `WalkUp` | 4.0 x 3.5 m | brick, punched sash windows, shopfront ground storey, cornice, water tank, chimneys |
| ribbon office | `Office`, not punched | 4.0 x 3.5 m | concrete spandrels, ribbon glass, column fins, HVAC, dish |
| punched office | `Office`, punched | 4.0 x 3.5 m | deep concrete grid, recessed glass |
| tower | `Tower` | 4.0 x 3.5 m | curtain wall, mullions, dark spandrels, masts, tank |
| warehouse | `Warehouse` | 4.0 x 3.5 m | brick, steel-sash windows, loading doors, skylights |

## 2. Distances

| Detail | Within | Mesh | Facade |
| --- | --- | --- | --- |
| Dressed (as now) | `dress_radius_m` 250 m | the full skin, glass, roof props | the material sets |
| Massing | `massing_radius_m` 1500 m | box faces, plus the type's parapet, cornice band, setbacks and the 4 largest roof props from the dressing | the type's baked atlas |
| Box | beyond | 12 triangles, roof cap | the type's baked atlas |

Each boundary has the dressing's 15 % hysteresis (`keep_ratio: 1.15`), so a building on the
line does not flicker. Both radii and the ratio live in `dressing:` in `data/world/world.ron`, and one pure rule
(`DressingConfig::far_detail`, `shows`) decides every boundary for both the dressing and the massing (survey L2,
owner 2026-10-26: "Approve all").

## 3. The far facades, rendered at startup (as built, 2026-10-26)

The near skin's brick and concrete maps are generated when the game starts (`textures`), so an
offline tool could not see them; the far facades are rendered at startup instead, from the same
maps, the moment the near arrays are built (`cb-app/src/far_facades.rs`), and nothing is written to
disk to go stale.

- **The texels are pure** (`cb_core::world::far_facade`): for each variant, `dress_variant` dresses a
  4 x 3 x 2 bay sample building in that variant's look, and `render` draws one bay of its long face
  face-on, orthographic: each pixel the nearest piece's material, colour and depth.
- **The maps** (`far_facades`): colour is the material's own near map sampled where the pixel is on
  the facade, times the piece's colour (glass at 60 %: far glass shows the sky, not the room); the
  normal comes from the depth steps; occlusion from a pixel sitting deeper than its neighbours (a
  recessed window, a reveal); roughness and metal by material.
- **One tile per kind and main colour, not a tinted neutral tile**: the near shader multiplies the
  whole layer by one vertex colour, which would tint the glass with the brick. So each kind is
  rendered in each of its palette's main colours (6 bricks for walk-ups and warehouses, 4 concretes
  for each office kind, 4 glasses for towers): 24 variants, a ground storey and a typical storey
  each, and the roof, 49 layers. The rest of a far look (window frames, a walk-up's glass) is the
  first choice of its palette; the main colour, which is most of what is seen past 250 m, matches.
- **Resolution**: a pixel 250 m away covers about 0.25 m from the cockpit at 1920 x 1080; a tile is
  64 x 64 px for one 4 m bay by one 3.5 m storey, 16 px a metre along the face.

The walls (`buildings::far_mesh`) are laid on the building's own cell grid, one bay per cell and
one storey per band repeat, the ground storey its own band, and a roof cap. Side 0 and side 2 show
the bay mirrored (the texture runs with world x and z); every bay is near symmetric, so it does not
read. The massing (`dressing::massing`) is the real dressing's pieces on and above the roof line
(parapet, coping, cornice, roof props), the 16 largest by volume, in the near skin's own arrays.

## 4. The far material

The far walls wear the opaque building material (`ArrayMaterial`, `building_array.wesl`, unchanged)
pointed at the far arrays: layer from the vertex's second UV, untinted, slot 0 (always standing).
The massing wears the same material pointed at the near arrays. Both are made once the element
state buffer exists and follow it when it grows (`buildings::bind_far_looks`).

## 5. New types

A test, `every_building_type_has_its_far_look`, walks every facade kind and fails for one without
a baked layer in the manifest or a massing recipe. `building-variety` (six types) and
`small-buildings` (seventeen) each gain a task: bake its far look.

## 6. Cost

- **Textures**: 49 layers x 3 maps x 64 x 64 px x 4 bytes x 4/3 for mips is 3.2 MB. The far boxes'
  five tone materials and their use of the generic facade texture are retired (a damaged frame's
  facade panels still use it).
- **Startup**: rendering the 48 tiles is a few milliseconds on the CPU.
- **Triangles**: massing at most 16 pieces (about 200 triangles), walls and roof 20. Seed 42 from the start: about 400
  massing buildings within 1.5 km and 3,200 boxes, about 120 k triangles, under today's dressed
  buildings near the pilot.
- **Draws**: one far material for every kind (today five tone materials), so Bevy batches more of
  them; measured on the owner's machine (`perf-measure`), the cockpit combat bench, old against new
  interleaved, since a cloud session measures nothing.
- **CPU**: the swap check already runs for the dressing; the massing mesh is built once per
  building when it first comes within 1.5 km, off the main thread like the dressing.
- **Bake**: offline, seconds; not in the game.

## Open questions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| L1 | How the far facade textures are made | rendered by a tool from each type's own near facade pieces and material sets | decided (owner, 2026-10-26: "Approve all", then "Go and implement"; survey L1). Built as the renderer `cb_core::world::far_facade` with `cb-app/src/far_facades.rs`, run at startup rather than offline because the near maps it samples are themselves made at startup (section 3); listed in CLAUDE.md section 13 |
| L2 | Where the massing gives way to the box | 1.5 km | decided (owner, 2026-10-26: "Approve all", then "Go and implement"; survey L2). `DressingConfig::far_detail` in `cb-core`, one rule for both boundaries with `keep_ratio` 1.15 (tests `the_massing_gives_way_to_a_plain_box_at_one_and_a_half_kilometres`, `a_building_on_the_line_does_not_flicker`) |
