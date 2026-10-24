# Design: terrain-water

## 1. Generation (cb-core::world::water), after terrain, before roads are graded

1. **Sea level** `sea_level_m` (data, default the 20th percentile of base terrain height). Basins
   below it become lakes, but never inside a city square (cities are placed on ground above it).
2. **Rivers** `river_count` (default 2): each starts at the world edge at a high sample and
   descends by steepest descent on a coarse 32 m grid, with seeded meander noise, until it reaches a
   lake or the opposite edge. Cities are obstacles; the path bends round them at
   `river_city_clearance_m` (default 80 m).
3. **Carving**: along the path the ground is lowered to a channel: `river_width_m` (default 40 m,
   growing 30 % along its length), `river_depth_m` (default 6 m at the centre, a smooth U section),
   banks blending back over 30 m. The water surface sits `freeboard_m` (default 1.5 m) below the
   banks and falls monotonically downstream.
4. **Highways cross on bridges**: where a highway crosses a river the grading stops; a bridge deck
   (box placeholder, a `blender-csg-levels` model later) spans the channel at road grade.

Every step draws from its own fork (`water`, `river/<i>`). Golden seed extended.

## 2. Wading (cb-core::mech, fixed step)

For each leg, submerged length `s` (0 to the hip) from `water_at` at the foot:
- drag force `0.5 * rho * Cd * A * v_rel^2` per leg, `rho = 1000 kg/m3`, `Cd = 1.1`,
  `A = leg width * s`, with `v_rel` the leg's speed relative to the flow;
- top speed scales by `1 - 0.6 * s / hip_height`, and the stride lifts `+0.4 * s`;
- depth over `max_wade_depth_m` (data, default the cockpit floor, about 8 m) refuses the step, as a
  wall does.
Mass and leg sizes come from the mech's data. Until the heat model exists, cooling is a number the
cockpit can show (kW removed), computed but unused.

## 3. Drawing (cb-app::water)

- One mesh per lake and per river (a strip along the path, 4 m spacing), `StandardMaterial` with
  alpha, two scrolling normal maps along the flow, depth tint from a depth texture baked at
  generation, foam where depth is under 0.5 m.
- **Interactive ripples**: a 256 x 256 ripple height texture covering 256 m around the camera,
  stepped by a wave equation in a compute pass (or on the CPU at 64 x 64 first), stamped by feet,
  splashes and shots, sampled as extra normal in the water material.
- **Spray**: a particle burst on each foot strike in water (data: count, speed, lifetime), a ring
  stamp, and steam if the leg is hot.

## 4. Cost (expected; measured before it is accepted)

| Item | Count |
| --- | --- |
| Water meshes | 2 rivers x about 2 k triangles, lakes about 1 k each; 3-6 draws |
| Ripple simulation | 64 x 64 CPU: about 0.05 ms a step; 256 x 256 compute: under 0.1 ms GPU |
| Spray | up to 2 k particles while wading, 1 draw |
| Transparency | one sorted transparent pass over water; overdraw is the risk, measured in the cockpit |
| Generation | under 50 ms more |

## Open questions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| W3 | How deep can a mech wade? | to the cockpit floor (about 8 m); deeper is refused | provisional (survey W3) |
| W4 | Water in the first world | 2 rivers and lakes below sea level | provisional (survey W4) |
