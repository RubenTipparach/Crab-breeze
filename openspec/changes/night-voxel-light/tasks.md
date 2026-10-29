# Tasks: night-voxel-light

Written up 2026-10-29 (owner: "at night time, can we do voxel lighting that bakes vertex colors instead of fake
glass lighting?"). Survey N9-N11 open, built on the recommendations while open.

## 1. Measure first

- [ ] 1.1 Count the static world's vertex spacing in a city: the median and the 95th percentile edge length of
  the building frames, houses, ground and paving meshes, printed by `cb-tools night-light --spacing` (a
  measurement instrument, section 4 of CLAUDE.md). A mesh whose spacing is over 4 m where a pool falls is listed
  for section 3.3.
- [ ] 1.2 Count the light sources a city bakes: its lamps, and its lit facade cells.

## 2. The bake in `cb-core`

- [ ] 2.1 `cb_core::world::night_light::Voxels`: a city's sparse 2 m occupancy in 16-voxel bricks, built from the
  building frames' element boxes and the ground height (tests: a voxel inside a wall is solid, one in the
  street is air, one under the ground is solid).
- [ ] 2.2 The flood fill (section 3 of the design): every lamp's head and every facade window cell seeds its
  channel; a summing diffusion over the 6 air neighbours, double-buffered, a fixed number of iterations, the
  decay and the wall absorption from data (tests: light round a corner is dimmer than in sight at the same
  distance; none reaches behind a solid wall 2 voxels thick; the same city gives the same bytes twice; source
  order does not change the result).
- [ ] 2.3 `NightLight::at(p, n)`: trilinear in air at `p + n * sample_offset_m`, two channels, and a test against
  a hand-placed lamp over flat ground (the pool's radius and the light at its centre).
- [ ] 2.4 Tuning in `data/world/night_light.ron` (voxel size, brick size, lamp and window strength, falloff,
  range, sample offset), validated by `cb-tools validate`.

## 3. Baked onto the vertices in `cb-app`

- [ ] 3.1 The `night_light` vertex attribute (Float16x2) on the building frames, houses, ground, paving, lamp
  poles and rocks of a city, filled when its tile's meshes are built on the worker, before upload.
- [ ] 3.2 The shaders add it: `building_array.wesl`, the houses' and the ground's materials (a material extension
  where they are plain `StandardMaterial` today), by one shared WESL function `night_light()`.
- [ ] 3.3 Meshes over 4 m spacing under a pool (task 1.1) are subdivided where the bake's light changes faster
  than the mesh can carry; or, if task 1.1 shows the paving is the only one, the paving alone samples a 2D
  light map of the city's street level.
- [ ] 3.4 A damaged building rebakes its city's bricks within the reach of its windows on a worker, and swaps
  the vertices of the meshes in that reach when it is done. Its own windows go dark at once (already so).
- [ ] 3.5 Tier 1 shrinks to 32 real lamps within 120 m (survey N9, provisional), for moving things and glints.

## 4. Proof

- [ ] 4.1 Headless stills (lavapipe): the 20:00 and 00:00 street and air views of
  `docs/screenshots/2026-10-28-city-lights-at-night.png`, before and after, with a contact sheet looked at.
- [ ] 4.2 On the owner's machine: the bake's time per city and its memory peak, and the city at midnight before
  and after, p50, p95 and p99 (CLAUDE.md section 7).
- [ ] 4.3 Playtest at night: walk a street, board, fly over the city.

## 5. Survey

- [x] 5.1 N9-N11 on the survey page (2026-10-29)
