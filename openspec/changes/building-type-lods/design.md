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

Each boundary has the dressing's 15 % hysteresis (`keep = radius * 1.15`), so a building on the
line does not flicker. Both new radii live in `dressing:` in `data/world/world.ron`.

## 3. Baked facade atlases

`cargo run -p cb-tools -- bake-facades` builds, per facade kind, the pieces of one bay: the ground
storey, a typical storey and the crown band (parapet and coping), with the dressing's own piece
generator, then renders them face-on, orthographic, on the CPU. The pieces are boxes with a
material and a colour, so the render is exact: a depth and material buffer, colour sampled from the
same PBR set the near skin uses (brick, concrete, trim, glass), a normal from the depth steps and
the set's own normal map, occlusion from the depth cavities, roughness and metal from the set.

The bake is neutral in colour where the near skin is tinted: walls in the set's own colour with a
wall mask, glass with a glass mask. The far material multiplies them by the building's own wall
and glass colours from `look_for`, carried as vertex colours, so one atlas per kind covers every
colour the dressing picks.

**Resolution**: from the cockpit at 1920 x 1080 with a 60 degree view, a pixel 250 m away covers
about 0.25 m. The atlas is 16 px per metre, twice that: a 4 m bay is 64 px wide, a 3.5 m storey 56
px tall. Per kind, three bands in one 64 x 192 px tile (ground, typical, crown).

Output: `assets/textures/buildings/far/<kind>_colour.png`, `_normal.png`, `_orm.png`, and a
manifest the game checks against the kinds it knows. Loaded as one texture array per map, one layer
per kind, full mips.

## 4. The far material

One material for every far building (massing and box), reading the three arrays: its layer is the
kind, its tint the building's wall and glass colours (vertex colours), its UV the bay and storey
grid from world metres (u along the face at 1 / 4 m, v by storey, the bottom band for the ground
storey and the top for the crown). It hands its result to Bevy's PBR lighting the way the building
texture array material does (`building-texture-array`), so sun, shadows and fog are unchanged.

## 5. New types

A test, `every_building_type_has_its_far_look`, walks every facade kind and fails for one without
a baked layer in the manifest or a massing recipe. `building-variety` (six types) and
`small-buildings` (seventeen) each gain a task: bake its far look.

## 6. Cost

- **Textures**: 5 kinds x 3 maps x 64 x 192 px x 4 bytes x 4/3 for mips is 0.98 MB; with the 23
  planned types, 5.5 MB. The generic facade texture is retired.
- **Triangles**: massing about 120-300 per building, box 12. Seed 42 from the start: about 400
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
| L1 | How the far facade textures are made | rendered by `cb-tools` from each type's own near facade pieces and material sets (section 3); this renders the real geometry rather than imitating a graph, and needs listing as an exception in CLAUDE.md section 13 | provisional (survey L1) |
| L2 | Where the massing gives way to the box | 1.5 km | provisional (survey L2) |
