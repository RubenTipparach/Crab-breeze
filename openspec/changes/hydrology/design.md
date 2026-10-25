# Hydrology design

## 1. The crater range

`Shape::range_height(p)`: with `d` the distance from the rim circle, an envelope
`(1 - smoothstep(0, half_width, d))^2` times `peak * (0.25 + 0.75 * ridged)`, ridged value noise of
five octaves at 3.6 km, warped 900 m, plus a cliff step `scarp_m` where `d` crosses
`scarp_offset_m` (wobbled 300 m), all cut to nothing within `pass_width_deg` of each pass bearing.
No terraces: the owner asked for smoother ground. Peaks reach about 1.7 km on seed 42.

## 2. Drainage

- Grid: 128 m cells, 320 x 320 (102,400 shape samples, about 40 ms).
- Priority flood (Barnes et al. 2014) from the world's edge and every cell below sea level: a
  cell's filled height is at least its neighbour's plus 1 cm, and its receiver is the cell it was
  reached from, so every basin drains over its spill point. Ties go to the lower index.
- Flow: each land cell gives `lowland_rain` (0.08) plus melt ramping to 1 over 400 m below the
  snowline, summed in reverse flood order.
- Streams: flow at least 12. Courses run from each unfed stream cell, highest first, until the
  sea or a cell an earlier course took (a confluence).
- Lakes: flooded components at least 8 m deep and 16 cells, fed by flow of at least 12; the ten
  largest, as a circle of the same area round the component's middle, 70-900 m in radius.

## 3. Rivers, falls, lakes

- Course points are pushed up to 0.45 cell by noise (so a slope's course meanders), Chaikin-cut
  twice and resampled every 16 m. A tributary ends on the river it meets.
- Surface: running minimum of ground less 2 m freeboard; held at a lake's level inside it; never
  below the sea.
- Half width: `5 m * sqrt(flow / 12)`, at most 28 m. Channel depth: at most 7 m and at most the half
  width. Banks rise at 0.45, steepening quadratically to 600 m at 80 m out, so a carve never ends
  at its index's cell edge.
- Waterfall: a run of steps each dropping 12 m or more, at most 4 steps long, 25 m or more in all.
  A lip point is put where the ground first drops 1 m. A river at least 10 m in half width gets a
  plunge pool: radius 1.4 m per metre of fall, level at the foot.
- Lake shape: shore distance wobbled 12 % by noise; inside, a bowl `depth` deep (ground more than
  8 m above the water stands); outside, a bank cresting 2.7 m above the water 8 m out, then falling.
  Lakes stay clear of cities and highways.

## 4. Biomes

`Biomes::weights(p, h, slope)`: moisture is `1 - smoothstep(-2 km, 12 km, p . dir + wander)` with
25 % patch noise; desert under 0.40, savanna and grassland fading in over bands of 0.13. Shares on
seed 42: temperate 52 %, desert 23 %, savanna 10 %, snow 6 %, montane 5 %, rock 3 %, beach 1 %,
wetland 1 %.

## 5. Costs

- Generation: seed 42 in about 150 ms (test profile, release), was 60 ms.
- Draw calls: river strips in 4 km tiles (at most 100), one per lake (19), two per waterfall
  (140 for 70 falls; to be merged into tiles if measured costly). Triangles: lakes 96 each,
  falls 48 each.
- Height sample: the rim adds a ridged and two warp noises inside the ring (about 0.3 us more);
  lakes add a distance test per lake.
- Not measured on the owner's hardware: cloud sessions measure nothing (CLAUDE.md section 7).

## Open questions

| Id | Question | Provisional |
|---|---|---|
| H1 | Is a waterfall sheet with foam enough, or do falls want spray and mist particles? | Sheet and foam |
| H2 | Should passes be placed where the rivers leave, or stay at fixed bearings? | Fixed bearings |
