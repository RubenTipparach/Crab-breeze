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

## 5. The sun moves (a line across the ground at dawn and dusk)

**Seen** (owner, 2026-10-29, from the cockpit over open country: "weird hroizon line at dusk here", "dusk and dawn
have it, not noon or night"): a straight line across the ground at about 700 m, dark green this side, lit orange
beyond.

**Why.** Section 1 bakes the far shade once, for `sun_shade::to_sun()`, the fixed sun of the game before it had a time
of day (about 43 degrees up). `day_night` now moves the live sun. At noon both agree closely enough; at night there is
no sun to cast either. At dawn and dusk the live cascades draw the hills' long shadows over the near ground, and the
far ground beyond `live_far_m` still wears the short shadows of the fixed sun: lit, where it should be shaded. The two
meet at 700 m, which from the cockpit's height reads as a straight line.

**Plan.**
- The bake takes the live sun (`day_night`'s direction), not the fixed one, and is redone off the main thread
  (`AsyncComputeTaskPool`) when the sun has moved more than `rebake_deg` (data, provisional 2 degrees: at a 24 min
  about every 16 s of play) since the last bake. 445 ms on one thread, so it never touches a frame.
- The far chunks' colour is not rewritten: the shade moves from vertex colours to a 1024 x 1024 one-byte texture
  (1 MB) the far ground's material samples by world position, swapped whole when a bake finishes, so a rebake costs
  one texture upload and no remesh. Between bakes the shade lags the sun by under `rebake_deg`.
- With the sun below the horizon the bake is skipped and the factor is 1 (the night has no sun shadow to match).
- The seam itself: the last cascade fades out over its last `fade_m` (data, provisional 100 m) into the baked shade,
  so the two never meet in a hard edge even while the bake lags.

**Cost.** CPU: 445 ms on a worker thread every 16 s or so, nothing on the main thread. GPU: one 1 MB upload a rebake,
one texture sample a far-ground fragment. To be measured (perf-measure) with the time of day running at dusk.

**Check.** A still at dusk and at dawn from the cockpit over open country with no line; a test that the bake for a sun
5 degrees up shades the lee of a ridge the 43 degree bake leaves lit.

## Open questions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| DS1 | How dark distant shade is | 0.55 of the colour | provisional |
| DS2 | How far the sun moves before the far shade is rebaked | 2 degrees | provisional |
