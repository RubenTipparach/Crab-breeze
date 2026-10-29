# Design: night-voxel-light

All numbers here are estimates until task 1 and task 4.2 measure them on the owner's machine. Cloud sessions
measure nothing (CLAUDE.md section 7).

## 1. What is fake today, and what the bake replaces

| Light | Today | After |
| --- | --- | --- |
| Lit window | Emission in the glass. The glass glows; the room behind it is dark and nothing outside is lit. | The glass is clear, no emission. The room behind it is lit (section 2a), and the light from the room spills out through the glass onto the street and the facing buildings, baked |
| Street lamp, within 250 m | One of 96 real spot lights. It lights everything under it. | Baked pool on the static world. A real light remains only within 120 m, for moving things and glints (N9). |
| Street lamp, past 250 m | A glowing head and a glow sprite. They light nothing. | Baked pool, the same as near ones |
| Moon, sky | Live, unchanged | Unchanged |
| Cars, the player's lamps | `day-night-cycle` section 7 | Unchanged (they move) |

## 2a. Lit rooms, not glowing glass (owner, 2026-10-29)

Owner: "I dont want the glass itself to glow, I want to see the room lits". So:

**The glass**
- The window glass loses its night emission (`window_light` in `building_array.wesl` and
  `building_surface.wesl`). Otherwise it stays the daytime glass (`glass-reflections`): mirror-coated, tinted
  per building, premultiplied alpha.
- **A night reflection map** (owner, 2026-10-29: "cant we just put a night time reflectioon map on that?").
  - Today the eye camera's one environment map is the cached day sky, only dimmed at night
    (`day_night.rs`, `sky.intensity = day_sky_intensity * sky_light_scale`).
  - A second cube is baked the same way (`tools/textures/gen_sky_cube.py`) from a CC0 night HDRI. Below its
    horizon it carries the lamp-lit street's warm tone.
  - The two cubes crossfade by the sun's height: the same curve as `sky_light_scale`, a mix uniform, two
    texture reads a reflecting pixel.
  - The street lamps' highlights in the glass come from the real lamp lights near the eye (N9).
- **How much of a lit room shows through.**
  - The day glass lets 8 % of what is behind it through (alpha 0.92), chosen so unlit interiors read dark by
    day. At night that leaves a lit room faint.
  - The glass's alpha follows the clock: 0.92 by day, 0.75 at night, so 25 % shows through (owner,
    2026-10-29: "25% is the ticket").
  - **Gradual** (owner: "I expect this transition to happen gradually as it gets dark"). The alpha and the
    night map's share both follow one smoothstep of the sun's height, from +4 degrees (all day) to -8 degrees
    (all night).
    - That is 12 degrees of sun travel, about 1.6 minutes of play at 24 minutes a day.
    - The rooms come on one by one by the window rule (17:30-20:00, `day-night-cycle` section 6), so a tower
      lights up through dusk instead of switching on at once.
  - In the mockup a lit office reads clearly at 25 % and faintly at 8 %
    (`docs/screenshots/night-light/*-c.png` against `*-c25.png`).

**What lights up**
- What lights up is the room behind a window. A lit room has:
  - a ceiling fitting: an emissive panel on the room's ceiling, part of the furniture kit, on when its room is
    lit;
  - **its own vertices brightened by their distance from the room's light point** (owner, 2026-10-29: "we just
    need to find vertices for each window cell area, and just brighten them by distance from a point").
    - Every vertex of the room (walls, floor, ceiling, partitions, furniture) is lit by
      `I / (1 + (d / r0)^2)`, where `d` is its distance from the fitting.
    - That is shaped by how much the vertex faces the fitting: `0.45 + 0.55 * max(0, n . l)`.
    - No voxels are used inside a room. A room is a box with one light in it, so nothing in the room blocks
      that light that matters.
  - The value is written once, when the room's mesh is built on its worker, into the window channel of the
    room's vertices. The shader gates it per room by the same lit rule the glass uses today (`WindowLights`: a
    hash of the building and the room, with a switch-on and a switch-off hour), using the room slot the mesh
    already holds. A dark room stays dark inside.
  - **Measured against the voxel way** in the comparison mockup (`docs/mockups/night_light.html`, `bench()`),
    same session, interleaved, 7 runs each, on a 10-storey tower: 80 offices, 37 lit, 75,944 room vertices.
    - Voxels: median 1,514 ms (1,412-1,608 ms).
    - Distance from the point: median 4.7 ms (3.3-14.2 ms).
    - About 320 times faster, with no overlap between the two.
    - JavaScript in a cloud container, so only the ratio counts.
    - Both cost the same per frame: a number on the vertex.
- Two rooms of one flat or office floor switch together, so a floor reads as an office with its lights on, not
  as a chessboard.
- The light leaves the room through the glass. In the voxels, glass is air for light and solid for nothing
  else, so a lit room spills onto the street and the building across it.
- That spill is the window channel outside the building. It is scaled by the hour's lit share, not per room:
  outside the building, single rooms blur together.

**Near and far**
- **Within 250 m** the rooms are real geometry (`building-interiors`: rooms and furniture are drawn within
  `rooms_radius_m`). The lit room is that geometry with its baked light, seen through clear glass.
- **Past 250 m** there is no room geometry. The glass draws an interior-mapped room instead: a box raytraced
  behind the pane, one per window cell, with floor, ceiling and back wall, the depth of the building's room
  bands. This is the far tier `building-interiors` section 3 already plans for past 150 m.
  - It is lit by the same per-room rule, with the ceiling fitting's warm light falling off from the ceiling
    across the back wall.
  - From 1 km it reads as a lit room in a window, not as a glowing pane.
  - The 4 x 4 atlas of room renders that `building-interiors` plans can replace the plain box later.
  - Cost: one ray-box intersection per glass pixel past 250 m, about the price of today's window hash.
- **Far facade impostors** (`building-type-lods`) keep their own window layer. At that range a window is under
  a pixel, so a lit room and a lit pane look the same.

## 2b. Houses and small businesses (owner, 2026-10-29: "Do the same for houses")

Houses get the same night as the towers (`day-night-cycle` task 4.5 said their glass "holds zero for now").
- **Glass.** It behaves like the tower glass: no glow, and 25 % see-through at night, gradual through dusk.
  A house's glass material is `AlphaMode::Blend` (`small_buildings.rs`), so the same alpha curve applies to
  its own alpha.
- **Rooms.** A kit has no floor plan, so a house's rooms are cells:
  - storeys of `house_storey_m` (3.0 m) from the kit's floor, cut into `house_cell_m` (4 m) squares in the
    kit's frame;
  - each cell has a light point under its ceiling.
  - Only the inside surfaces take it: `plaster`, `floorWood`, `floorTile`, `carpet`, `woodInt` (the kit's
    `NO_SHADOW` inside list) and the furniture.
  - Outside walls, roofs and glass take none.
- **Switching on.** A kit's meshes are shared by every lot of that kit, so the lit rule is not baked into the
  mesh. The vertex carries its cell's index. The shader hashes that index with the drawn mesh's world origin,
  so two lots of one kit light different rooms.
- **A whole house switches as one** (owner, 2026-10-29: "To simplify, consider an entire house lit or unlit
  instead of individual rooms"). Every room of a house carries the same index, so the hash depends only on the
  lot's origin: a house is lit or dark, and so is a business, open or closed. The 4 m cells still place the
  light points, so a lit house is brightest under each ceiling's middle.

## 2c. How the room light rides on the vertex (implementation)

The building and house shaders share one vertex shader (`building_state.wesl`), and every building mesh can
carry a second UV. So the room light uses that second UV's spare half, with no new vertex layout:

- `uv_b.y = cell + min(light, 3.999) / 4`:
  - `cell` is the room's index in its mesh: a whole number below 4,096; 0 means not in a room.
  - `light` is the room light at the vertex, `1 / (1 + (d / r0)^2)` shaped by its facing.
  - The tower meshes already use `uv_b.x` for their texture layer, and `uv_b.y` was always 0. House and
    furniture meshes gain a second UV for this.
- **The vertex shader** decodes it and decides whether the room is lit: a hash of the cell and the mesh's world
  origin, against the share lit now (`WindowLights::lit_share`). It hands on `light * lit` in `uv_b.y`.
  - This has to be the vertex shader: an index cannot be interpolated across a triangle, but a lit light can.
- **The fragment shaders** (`building_array.wesl`, `building_surface.wesl`) add
  `albedo * room_colour * uv_b.y * room_cd_m2`.
- **The room uniform** is a new `rooms: vec4` at binding 107 in both building material extensions, so the one
  vertex shader binds it whichever material draws:
  - x: the share lit now;
  - y: the room light's brightness in cd/m2;
  - z: the glass's night share, 0 by day to 1 at night.
- **Packing in Rust.** `cb_core::night_light::{pack_room, unpack_room}` packs the second UV, and a test round-
  trips it at the edges.
- **Not exact.** A vertex on the line between two rooms belongs to the one its normal faces. A floor bay
  spanning a lit and a dark room shades from one to the other across the bay.

## 2d. Airports, seaports and runway lights (owner, 2026-10-29: "I also want to see airports and seaports lit like this at all times. runways will need runway lights as well (not actual lights, just light dots)")

**The sites' buildings are always lit at night.**
- The airport's and seaport's buildings are kit lots of the `Airport` and `Seaport` zones, drawn by the houses'
  code (`small_buildings`), so they already carry rooms and far windows (section 2b).
- Their rooms and far windows carry one more index, `ALWAYS_LIT` (4095, the last a mesh can number). The vertex
  shader lights it whenever it is getting dark, with no hash and no hourly share. Its light is scaled by the
  glass's night share (`rooms.z`), so a terminal comes on through dusk and stays on all night.
- They use the same glass (25 % through at night), the same room light and the same far windows as a house: one
  look for every lit building.

**The control towers' cabs are always lit** (owner, 2026-10-29: "top control towers should always be lit").
- The glazed cab on top of the airport's control tower, and the air bases' towers, carry `ALWAYS_ON` (4094): lit
  at full by day and night, with no hash, share or dusk curve.
- The cab is lit at twice a room's brightness, so on a dark field it is the brightest building, as in the
  owner's reference.
- By day it shows as a lit cab behind the mirror glass's 8 %; at night it shows through the 25 %.

**Runway lights are dots, not lights.**
- Each is a glow quad like a street lamp's far glow (`lamp_glow.wesl`):
  - at least 3 px across on screen, 0.6 m in the world;
  - coloured, and lit by no light source, so they cost nothing in the lighting;
  - on with the street lamps at dusk (`lamps.on_deg`).
- **Where**, on each runway (2,000 x 45 m and 1,500 x 45 m, `air_bases`) and the airport's (ICAO's pattern, cut to
  what reads):

| Lights | Where | Colour |
| --- | --- | --- |
| Runway edge | every 60 m along both edges, 3 m outside the paved edge | white; yellow over the last 600 m (caution) |
| Threshold | across each threshold, every 3 m | green |
| Runway end | across each end, every 3 m | red |
| Centre line | every 30 m on the centre line | white |
| Approach | on the existing approach posts' crossbars (`airport-layout` section 3), five a post | white |
| PAPI | the four existing boxes | two red, two white (on the glide path) |
| Taxiway edge | every 30 m along both edges of the parallel taxiway and its links | blue |

- **Drawing.** One mesh of quads per site and runway, coloured per vertex, built with the runway. One draw each.
- **Cost:** a 2,000 m runway with its taxiway is about 70 edge lights, 30 threshold, 30 end, 67 centre line,
  100 approach and about 140 taxiway: about 440 quads. Under 3,000 for the island.

**The apron and car parks get lamps** (owner, 2026-10-29: "area near terminals should probably be lit with parking
lot lamps", with a reference of a terminal apron at night: tall white masts along the stands).
- **Apron masts:** 25 m, every 60 m along the terminal's apron edge and between stand rows, each head a cluster
  of four floodlights. White mercury (survey N5: "white mercury at the air bases"), against the town's orange
  sodium.
- **Car-park lamps:** 9 m poles every 30 m along each car park's aisles, white.
- **Light.** Both are `cb_core::world::lamps` kinds, placed from the site's layout with the street lamps, so they
  are drawn, switched at dusk, made real lights near the eye (tier 1), glow dots far off (tier 3), and later baked
  into pools (section 3), with no new code path. On every night, as the sites' buildings are.
- **Estimated:** about 40 masts and 60 car-park lamps an airport, about 30 and 20 a seaport. That adds about 150
  lamps to the island's 5,241.

**Mockup first** (CLAUDE.md section 4, owner 2026-10-28: "Please make mockups before editing game code"):
`docs/mockups/runway_lights.html`. A 2,000 m runway at night with every light group, the taxiway, a terminal
with its apron masts and car park, and a seaport shed lit, from final approach, on the ground and from above, beside the Foreman and a pilot. No game
code for this until the owner has looked at it.

## 2. Voxels

- Each city gets an axis-aligned box: its square, plus 40 m round it, from the ground's lowest point to its
  tallest roof plus 10 m.
- **Occupancy** is sparse, in bricks of 16 x 16 x 16 voxels, and only bricks with air near a light source are
  allocated:
  - A voxel is solid if it lies in a building frame's element box (slabs, walls, columns, cores, partitions),
    in a house's box, or under the ground's height at its centre.
  - The ground is read from `Shape::height`, the same height the meshes use.
  - Glass is air to light (section 2a), so a lit room spills out of its window.
- **Two channels**, each one byte a voxel, laid out so the bricks of different cities run on different workers:
  - Lamps, at 2 m, in a band from the ground to 40 m above the street (lamp heads are at 9-12 m).
  - Window spill, at 4 m, outside the buildings, through the city's whole height. Inside a room no voxels are
    needed (section 2a).
- **Size of one city**, estimated: 960 m by 960 m by 200 m, with 35 % of the plan street or plaza.
  - Lamp channel: 480 x 480 x 20 x 0.35, about 1.6 M voxels.
  - Window channel: 240 x 240 x 50 x 0.35, about 1.0 M voxels.
  - Both in bricks: about 3 MB at the bake's peak. It is dropped once the vertices hold the result.

## 3. Light propagation (the voxel part)

**Per source, a shortest path through the air, summed.**
- **What was tried first.** A diffusion, `L = S + k * mean of the air neighbours` iterated. It does not
  carry light far enough. A diffusion spreads about the square root of its iteration count in voxels, so 70
  iterations at 0.5 m reached about 4 m: the comparison mockup measured 4e-9 of a lamp's light between two lamps
  20 m apart. It is not used.
- **Distance.** Each source runs a bucket-queue Dijkstra over the air voxels:
  - 26 neighbours, chamfer steps of 10, 14 and 17 per voxel;
  - limited to the source's range;
  - glass costs 2 m of extra path.
  That gives the shortest path `d` through air: round corners, never through a wall.
- **Light.** Each voxel reached gains `I / (1 + (d / r0)^2)`, faded to zero over the last 40 % of the range.
- **Sum.** The sources' fields add up. The result depends only on the voxels and the sources (sums are taken in
  source-id order), so the same city gives the same bytes (CLAUDE.md section 6.3).
- **Street lamps point down.** Above its head a lamp gives a fifth of its light. Without that, the voxels light
  the tower's spandrels orange up to the tenth floor.

**Sources:**
- **Lamps.** Each lamp (`cb_core::world::lamps`) injects `lamp_strength` at its head's voxel, and half that in
  the voxel under it, so the pool is brightest under the head rather than round it.
- **Rooms, outside the building only** (the spill; the room's own vertices use the point, section 2a). Each
  window cell injects `room_strength` just inside its glass. Every room is baked as lit; outside the building
  the hour's lit share scales the whole channel (section 5).
  - Crossing glass costs 2 m of path, so a lit room lights its neighbours' glass only faintly.
- An air voxel next to a solid one loses a further `wall_absorb` share each step. That keeps the light from
  creeping along walls and makes the pool read as falling on the ground.

**Cost, estimated** (the mockup measured the room case only, section 2a):
- Each voxel a source reaches costs about 26 neighbour checks.
- Lamps: about 220 per city, each reaching about 12,000 air voxels at 2 m (35 m range in a street canyon).
  About 70 M checks.
- Window spill: one source per lit window cell at 4 m voxels, each reaching about 200. About 600 M checks for
  120,000 window cells, so the spill sources are merged per 4 m facade cell first (about 30,000). That is
  about 150 M.
- Together an estimated 0.3-0.6 s of one worker per city, while the city's other meshes are being built on
  the streaming workers. Measured in task 4.2.

## 4. On the vertices

- `NightLight::at(p, n)` reads both channels, trilinearly, at `p + n * sample_offset_m` (1 m).
  - The offset puts a wall's sample in the air in front of it, so a wall facing away from the street reads its
    own side.
  - A sample that lands in a solid voxel takes the mean of its air neighbours, or 0 if it has none.
- The bake writes a `night_light` vertex attribute, Float16x2 (lamp, window):
  - 4 bytes a vertex, on the meshes that already hold their colour attribute.
  - The colour attribute itself is untouched: the buildings use its alpha for the element slot, and the RGB is
    the tint.
- **Meshes that carry it** (the static world in a city's box): building frames, houses and small businesses,
  ground and paving tiles, lamp poles, rocks.
- **Vertex spacing.**
  - A lamp pool is about 35 m across, so a mesh needs vertices about every 4 m to draw it.
  - Building frames have a vertex at every storey and bay (about 3.5 m by 2.5-4 m). That is enough.
  - Near ground has 8 m cells and the paving is to be measured (task 1.1). If the paving is the only mesh too
    coarse, it reads a 2D street-level light map (one 512 px R16G16 texture per city, 2 MB) rather than being
    subdivided (task 3.3).
- **When a building is damaged**, the bricks within the window reach of it are rebaked on a worker. The meshes
  in that reach then get their vertices replaced.
  - That is a few bricks, estimated under 20 ms of worker time.
  - Its own windows go dark at once (already so, `day-night-cycle` section 6). Its spill on its neighbours
    lingers until the rebake lands.
- **Rooms near the eye.** A building's rooms are generated on a worker when the eye comes within 250 m. Their
  light from the point is written in that same job, so a room's light arrives with the room.

## 5. In the shaders

One WESL function, `night_light(albedo, light, lamp_on, lit_share)`, shared by every material that carries the
attribute. It returns the colour to add after `apply_pbr_lighting`:

    albedo * (light.x * lamp_colour * lamp_on + light.y * window_colour * lit_share) * view.exposure

- `lamp_colour` is sodium, 2,000 K. `window_colour` is the windows' mix: 70 % 2,700 K, 25 % 4,000 K, 5 % TV blue
  (`day-night-cycle` section 6).
- `lamp_on` is the lamps' dusk switch, the same one that lights the heads.
- `lit_share` is `WindowLights::lit_share(hour)`, as a uniform.
- Both are zero by day, so the term costs a multiply-add and nothing shows.
- The light is diffuse only. Specular glints on wet road come from the remaining real lights (N9).
- **Materials:**
  - The buildings' `building_array.wesl` is already a custom shader and gains the input.
  - Ground, paving and the houses' materials get a `MaterialExtension` (the way `building_state.rs` extends
    `StandardMaterial`) that reads the attribute and adds the term.

## 6. Costs

**Per frame: about nothing.** The baked light is two numbers on each vertex and a multiply-add in the shader.
The night map is one more cube read on reflecting pixels. Cutting the real lamps from 96 to 32 (N9) should save
more than all of that costs. That is an estimate, to be measured on the owner's machine (task 4.2).

**The bake: once per building, cached.** Measured in the comparison mockup: JavaScript in a cloud container,
5 runs each, medians, 0.5 m voxels. Only the ratios carry over to the game.

| Bake | Measured | Per source |
| --- | --- | --- |
| Room light, 75,944 room vertices by distance from their point | 4.7 ms (3.3-14.2) | 62 ns a vertex |
| 8 street lamps, 26 m reach | 545 ms (487-634) | 68 ms a lamp |
| Spill of 37 lit rooms, 26 m reach | 1,507 ms (1,430-1,763) | 41 ms a room |

**Scaled to a city at the design's voxel sizes** (estimated). A source's cost goes with the voxels it reaches,
so 2 m lamp voxels reach 64 times fewer than 0.5 m, and 4 m spill voxels 512 times fewer. Rust runs about 5
times faster than this JavaScript (to be measured).
- **Lamps:** about 220 a city at about 0.2 ms each in Rust. About 50 ms.
- **Spill:** about 30,000 facade cells a city at about 0.02 ms each. About 0.6 s.
- **Room light:** rooms are built only within 250 m, about 40 buildings at a time, at about 12 ns a vertex in
  Rust. Under 1 ms a building, done in the job that builds its rooms.
- Together, under a second of one worker per city on its first load, spread over the city's buildings as they
  stream in.

**Cached per building** (owner: "I also assume this would get cashed per building in the city").
- **What is cached.** A building's two light channels per vertex, lamp and spill, for each of its meshes, kept
  beside the meshes in `BuildingStates`.
- **Key.** The world seed, the building id, the bake's data version, and a damage stamp of the buildings within
  its spill reach. The voxels are dropped once the vertices hold the result.
- **Kept on disk** under the world's cache directory, keyed the same way, as the horizon map plans to be
  (`day-night-cycle` section 4). The second visit to a city reads it back instead of baking.
- **What a cache hit costs.** A city that streams back in costs only the read: about 4 bytes a vertex, about
  2 MB for a downtown block.
- **When a building is damaged.** Only the buildings within its spill reach rebake, on a worker. Its own
  windows go dark at once.
- **Room light is not cached.** At 62 ns a vertex it is cheaper to compute than to read, so it is redone
  whenever a building's rooms are built.

| Item | GPU | CPU | Memory |
| --- | --- | --- | --- |
| Diffuse term | one multiply-add a pixel on the static world | none | none |
| Night map | one cube read on reflecting pixels, crossfaded | none | one 512 px cube, about 12 MB filtered |
| Vertex attribute | 4 bytes a vertex fetched | none | 4 bytes a vertex |
| Bake | none | under 1 s per city on its first load (estimated); a cache read after | voxels freed after |
| Tier 1 from 96 to 32 real lamps (N9) | saves an estimated 0.3-1.0 ms at 1080p in a street | less clustering | none |

## 7. Determinism and tests

- The bake lives in `cb-core` (`world::night_light`): no renderer, seeded only by the city's plan.
- It is tested headless for:
  - light round a corner: dimmer than in plain sight, and above zero;
  - a wall that stops it;
  - the same bytes twice;
  - a pool's radius under one lamp over flat ground, against the data.
- `cb-tools validate` checks `data/world/night_light.ron`.

## Open questions

| Id | Question | Built while open | Status |
| --- | --- | --- | --- |
| N9 | What lights moving things (cars, people, the mech) at night? | Keep 32 real lamps within 120 m | open (survey) |
| N10 | Does a lit room's light spill out of its window onto the street and the facing buildings? | Yes | open (survey) |
| N12 | How much of a lit room shows through the glass at night? | 25 % at night, 8 % by day, gradual through dusk | owner, 2026-10-29: "25% is the ticket", "I expect this transition to happen gradually as it gets dark" |
| N13 | A far window (past 250 m, no room geometry): an interior-mapped lit room, or the glass glowing as today? | Interior-mapped room | owner, 2026-10-29: "I dont want the glass itself to glow, I want to see the room lits" (provisional on the far tier) |
| N11 | Lamp pools everywhere, seen from the air too (replaces N4's "pools only within 250 m")? | Everywhere | open (survey) |
