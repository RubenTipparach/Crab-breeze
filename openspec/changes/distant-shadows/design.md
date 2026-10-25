# Design: distant-shadows

## 1. The bake (`cb-core::world::sun_shade`, pure)

The sun is fixed (the game has no time of day yet), so its terrain shadow is baked once.

- **The grid:** `SunShade` is 1024 x 1024 cells over the island (40 m a cell), each a light factor
  from 0 (in shade) to 1 (sunlit), as one byte a cell: 1 MB.
- **The sweep:** lines are laid along the sun's horizontal direction, one per cell across, and
  walked from the sun's side. Each line keeps the height the shadow stands at: the highest of the
  ground seen so far, falling by `tan(elevation)` per metre walked. A point is in shade when its
  ground is below that height, softened over 4 m so the edge is not a stair.
- **Cost:** the lines read a 1025 x 1025 grid of the ground's corner heights, sampled once from the
  terrain's lattice (1 million samples), then about 9 million cheap grid lookups: 445 ms measured
  (release, one thread), at load behind the loading screen. Reading the terrain on every step of
  every line took 10.7 s.
- **Lookup:** `SunShade::at(xz)` is bilinear.

## 2. Drawing (`cb-app::ground`)

- Chunks drawn at step 2 or coarser multiply their vertex colour by `mix(1, shade_albedo, 1 - at(xz))`.
  `shade_albedo` is 0.55 in data: what is left of the sun's share of the light in shade.
- Full-detail chunks (step 1, within 700 m) keep live shadows.
- The live cascades end at 500 m today. A chunk's step changes at 700 m, so a 200 m ring has
  neither. The cascades' far end moves to 700 m to close it, with its cost measured
  (perf-measure).
- The sun's direction is the game's sun (`world.rs`), handed to the bake: one sun.

## 3. Craters

A crater moves the ground a few metres: too little to change a shadow kilometres long. The bake is
not redone.

## 4. Cost

| Item | Cost |
| --- | --- |
| Bake | under 1 s at load, measured |
| Memory | 1 MB |
| Per frame | nothing: vertex colours |
| Cascades to 700 m | measured against 500 m with the perf suite |

## Open questions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| DS1 | How dark distant shade is | 0.55 of the colour | provisional |
