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
  `building_surface.wesl`). It stays the plain reflective, see-through glass it is by day.

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

| Item | GPU | CPU | Memory |
| --- | --- | --- | --- |
| Diffuse term | one multiply-add a pixel on the static world | none | none |
| Vertex attribute | 4 bytes a vertex fetched | none | 4 bytes a vertex (about 4 MB for 1 M vertices in view) |
| Bake | none | 0.2-0.4 s per city on a streaming worker; rebake of a damaged building under 20 ms | 3 MB peak per city, freed |
| Tier 1 from 96 to 32 real lamps (N9) | saves an estimated 0.3-1.0 ms at 1080p in a street | less clustering | none |

**Net, estimated:** cheaper than today at night, because fewer real lights are clustered and shaded. To be
measured (task 4.2) before it is called a speedup.

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
| N12 | A far window (past 250 m, no room geometry): an interior-mapped lit room, or the glass glowing as today? | Interior-mapped room | owner, 2026-10-29: "I dont want the glass itself to glow, I want to see the room lits" (provisional on the far tier) |
| N11 | Lamp pools everywhere, seen from the air too (replaces N4's "pools only within 250 m")? | Everywhere | open (survey) |
