# Design: procgen-world

All numbers below are the defaults in `data/world/world.ron`; the code holds no tuning.

## 1. Scale

The yardstick is the 12 m mech (survey `M1`, provisional) and the 1.8 m pilot. The mech's feet sit
about 5 m apart and it strides about 8 m per cycle, so:

| Thing | Size | Why |
| --- | --- | --- |
| World | 4096 m square | a few minutes' stride end to end, room for five cities |
| Street | 24 m wide | two mechs pass with a sidewalk to spare |
| Arterial | 36 m wide, every 4th street | long sight lines downtown |
| Highway | 28 m wide | joins cities across open ground |
| Block | 70-110 m between street edges | three to four lots a side |
| Storey | 3.5 m | buildings run 2 storeys (7 m, below the cockpit) to 60 (210 m) |

## 2. Generation, in order

Each step draws from its own fork of the seed stream (`world`, `cities`, `city/<i>`,
`terrain`), so changing one step's data does not move another step's results.

1. **Base terrain**: four octaves of value noise, 900 m base wavelength, 35 m amplitude, sampled on
   an 8 m grid (513 x 513 samples).
2. **Cities**: rejection sampling of centres at least 1100 m apart and 300 m from the edge, up to 5
   cities; each gets a square half-size of 320-560 m and a ground height equal to the base terrain
   at its centre. City 0 is the largest and is where the pilot starts.
3. **Highways**: a minimum spanning tree over the city centres (Prim's, ties broken by index), each
   edge a straight road from one city's edge to the other's, graded linearly between the two city
   heights.
4. **Terrain shaping**: inside a city square the ground is flat at the city height, blending back to
   the base terrain over 120 m. Within 14 m + 12 m of a highway's centre line the ground is exactly
   the road grade, blending back over 60 m. The 12 m margin is wider than an 8 m cell's diagonal,
   so every terrain triangle touching a road surface lies in the road's plane: no clipping, and
   the road is drawn 0.1 m above it, so no z-fighting.
5. **Streets**: an axis-aligned grid of block widths (70-110 m, random per row and column) laid
   across the city square, with every 4th street an arterial. The outermost streets ring the city.
6. **Lots and buildings**: each block, inset by a 4 m sidewalk, is cut into lots of 18-36 m. 6 % of
   blocks are parks (no buildings) and 8 % of lots are plazas. A building fills its lot less a
   1-3 m setback. Its storeys fall from up to 60 downtown to 2 at the city edge:
   `max_storeys = lerp(60, 2, d^0.7)` where `d` is the lot's distance from the centre over the
   city half-size, then a random 35-100 % of that, at least 2.
7. **Stable ids**: buildings are numbered in generation order (city, block row, block column, lot),
   never by ECS or hash order.

## 3. Queries

- `height_at(x, z)` interpolates the 8 m grid with the same two triangles per cell the terrain mesh
  draws, so the pilot's feet and the drawn ground agree exactly.
- `push_out_of_buildings(pos, radius)` moves a circle out of every building footprint it overlaps,
  using a 64 m bucket grid over the buildings (sorted by id, so the result does not depend on
  bucket order). The pilot (0.35 m) and the mech (3 m) use the same function.

## 4. Drawing (`cb-app::world`)

- Terrain: 8 x 8 chunks of 64 x 64 cells, vertex-coloured by slope and height.
- Roads: one flat quad per street and per highway, 0.1 m above the ground.
- Buildings: one shared unit-cube mesh scaled per building, six concrete and glass tones picked by
  id. Bevy's automatic batching draws them in a handful of instanced draws.
- A sun with four shadow cascades to 900 m, and distance fog from 600 m.

## 5. Cost (expected; the measurement is in the PR and `docs/screenshots/`)

| Item | Count |
| --- | --- |
| Terrain | 263 k vertices, 524 k triangles, 64 draws |
| Roads | about 400 quads |
| Buildings | about 2000 boxes, 24 k triangles, batched into under 20 draws |
| Generation | under 0.3 s on the owner's Mac (release) |
| Per fixed step | one `height_at` and one `push_out_of_buildings` per body: microseconds |

## Open questions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| M1 | Mech height | 12 m, sets every size above | provisional (survey M1) |
| W1 | City street layout | axis-aligned grids now; organic layouts after the first real blocks exist | provisional (survey W1) |
| W2 | World size | 4 km square with 5 cities, streamed larger worlds later | provisional (survey W2) |
| T2 | Physics engine | none yet: the pilot and mech use kinematic ground and building queries until the physics change | provisional (survey T2) |
