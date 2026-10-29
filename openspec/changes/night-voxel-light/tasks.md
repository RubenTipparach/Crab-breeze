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
  channel; per source a bucket-queue Dijkstra over 26 air neighbours, `I / (1 + (d / r0)^2)` faded to the
  range, glass 2 m of extra path, the sources summed in id order (tests: light round a corner is dimmer than in sight at the same
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

## 3c. Houses (owner: "Do the same for houses")

- [x] 3.12 Houses' glass: no glow, 25 % see-through at night through the same dusk curve
- [x] 3.13 Houses' inside surfaces and furniture carry the room light from 4 m cells a storey; a whole house is
  lit or dark by a hash of its lot (owner: "consider an entire house lit or unlit"; design section 2b)

## 3e. Airports, seaports and runway lights (owner: "airports and seaports lit like this at all times", "runway lights ... just light dots")

- [x] 3.15 Mockup `docs/mockups/runway_lights.html`, published (https://claude.ai/artifact/CyXuvuLXdAoWfqcctdUKkr); waiting for the owner's look before 3.16-3.19
- [ ] 3.16 Site buildings (Airport, Seaport zones) carry `ALWAYS_LIT`: lit through dusk and all night, no share
- [ ] 3.19 The control towers' cabs `ALWAYS_ON`, lit day and night at twice a room's brightness (owner: "top control towers should always be lit")
- [ ] 3.18 Apron masts and car-park lamps as lamp kinds placed from the sites' layouts (design section 2d)
- [x] 3.21 Seaport night on the approved seaport page (Night button): masts, lamps, crane lights, and every ship lit (design section 2d); waiting for the owner's look
- [ ] 3.22 Ships carry their lights: rooms `ALWAYS_LIT`, deck lamps as lamp kinds on the kit, anchor and navigation lights as glow dots (design section 2d)
- [ ] 3.17 Runway, threshold, end, centre line, approach, PAPI and taxiway lights as glow dots (design section 2d)

## 3b. Lit rooms (owner: "I dont want the glass itself to glow, I want to see the room lits")

- [x] 3.6 The glass loses its night emission (`window_light` off in `building_array.wesl` and `building_surface.wesl`)
- [ ] 3.7 (Room light by distance from the room's point, gated per room, built for towers and houses; the
  ceiling fitting's emissive panel is not yet) A ceiling fitting in the furniture kit for every room type, emissive when its room is lit; each room
  vertex brightened by its distance from its room's light point when the room mesh is built (owner: "brighten
  them by distance from a point"), gated by its room slot through `WindowLights` (tests: a dark room's vertices
  get no light at any hour; the light falls with distance from the fitting; rooms of one flat switch together)
- [ ] 3.10 A night reflection cube from a CC0 night HDRI, crossfaded with the day sky, and the glass's alpha 0.92 by day to 0.75 at night (N12, owner: "25% is the ticket"), both on one smoothstep of the sun's height from +4 to -8 degrees ("gradually as it gets dark"); test: the alpha and the mix at noon, at -2 degrees and at midnight
- [ ] 3.11 The building light cache: each building's lamp and spill channels kept beside its meshes and on disk, keyed by seed, building id, bake version and the damage stamp of its neighbours in reach; a damaged building rebakes only those in reach (tests: a second load of a city reads the cache and bakes nothing; a damaged building invalidates its neighbours in reach and no others)
- [ ] 3.8 Past 250 m the glass draws an interior-mapped room per window cell, lit by the same rule
- [ ] 3.9 Glass is air to light in the voxels, so a room spills out of its window (test: the street in front of
  a lit room's window is lit, the street in front of a solid wall is not)

## 3d. First in-game stills (2026-10-29, lavapipe, 22:00)

- [x] 3.14 `docs/screenshots/night-light/ingame-2200-city-street.png`: lit offices seen through clear glass on both
  sides of the street, some on, some off; far towers keep their painted windows. `ingame-2200-house-near.png`: the
  whole house lit from inside behind its windows. Room light 12 cd/m2 (3 read dim behind the 25 % night glass).
  `CB_STILLS_SETTLE_S` lets a view settle until late pipelines are in (a late road pipeline showed as pale fog).

- [x] 3.20 House stills at 22:00 in game: `ingame-2200-house-near.png` (the towers' glass, the lit room behind it:
  `ingame-house-glass-before-after.png`), `ingame-2200-house-far.png` (far shells' windows lit house by house)

## 4. Proof

- [ ] 4.1 Headless stills (lavapipe): the 20:00 and 00:00 street and air views of
  `docs/screenshots/2026-10-28-city-lights-at-night.png`, before and after, with a contact sheet looked at.
- [ ] 4.2 On the owner's machine: the bake's time per city and its memory peak, and the city at midnight before
  and after, p50, p95 and p99 (CLAUDE.md section 7).
- [ ] 4.3 Playtest at night: walk a street, board, fly over the city.

## 4b. The comparison (done)

- [x] 4.4 `docs/mockups/night_light.html`: today, glowing panes in vertex colour, and lit rooms behind clear
  glass, side by side at 22:00 with the real voxel bake, shot into `docs/screenshots/night-light/`; its
  `bench()` timed the room light both ways (section 2a: the point is about 320 times faster)

## 5. Survey

- [x] 5.1 N9-N11 on the survey page (2026-10-29)
