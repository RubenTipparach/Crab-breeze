# Design: night-voxel-light

All numbers here are estimates until task 1 and task 4.2 measure them on the owner's machine. Cloud sessions
measure nothing (CLAUDE.md section 7).

## 1. What is fake today, and what the bake replaces

| Light | Today | After |
| --- | --- | --- |
| Lit window | Emission in the glass. It lights nothing. | The same emission, and its light on the walls, the street and the facing buildings, baked |
| Street lamp, within 250 m | One of 96 real spot lights. It lights everything under it. | Baked pool on the static world. A real light remains only within 120 m, for moving things and glints (N9). |
| Street lamp, past 250 m | A glowing head and a glow sprite. They light nothing. | Baked pool, the same as near ones |
| Moon, sky | Live, unchanged | Unchanged |
| Cars, the player's lamps | `day-night-cycle` section 7 | Unchanged (they move) |

## 2. Voxels

- Each city gets an axis-aligned box: its square, plus 40 m round it, from the ground's lowest point to its
  tallest roof plus 10 m.
- **Occupancy** is sparse, in bricks of 16 x 16 x 16 voxels, and only bricks with air near a light source are
  allocated:
  - A voxel is solid if it lies in a building frame's element box (slabs, walls, columns, cores), in a house's
    box, or under the ground's height at its centre.
  - The ground is read from `Shape::height`, the same height the meshes use.
  - Glass counts as solid: light leaves a facade outward, it does not cross the building.
- **Two channels**, each one byte a voxel, laid out so the bricks of different cities run on different workers:
  - Lamps, at 2 m, in a band from the ground to 40 m above the street (lamp heads are at 9-12 m).
  - Windows, at 4 m, through the city's whole height.
- **Size of one city**, estimated: 960 m by 960 m by 200 m, with 35 % of the plan street or plaza.
  - Lamp channel: 480 x 480 x 20 x 0.35, about 1.6 M voxels.
  - Window channel: 240 x 240 x 50 x 0.35, about 1.0 M voxels.
  - Both in bricks: about 3 MB at the bake's peak. It is dropped once the vertices hold the result.

## 3. Light propagation (the voxel part)

**A diffusion over the air voxels, Minecraft-style but summing.** Each source injects its strength into the
voxel it sits in. Then, for a fixed number of iterations, each air voxel takes:

    L_new = S + k * mean(L of its 6 neighbours that are air)

- `S` is the voxel's source strength. `k` is the step's decay, from data.
- Light spreads through doorways and round corners and pools in the streets.
- It never crosses a solid voxel, so it does not leak through walls.
- Overlapping pools add up.
- The iteration count is the reach in voxels:
  - Lamps: 35 m reach at 2 m is 18 iterations.
  - Windows: 24 m at 4 m is 6 iterations.
- The fill is double-buffered and reads only its neighbours, so a brick's iteration runs on a worker with its
  neighbours' last values.
- The result depends only on the voxels and the sources, never on visiting order: the same city gives the same
  bytes (CLAUDE.md section 6.3).

**Sources:**
- **Lamps.** Each lamp (`cb_core::world::lamps`) injects `lamp_strength` at its head's voxel, and half that in
  the voxel under it, so the pool is brightest under the head rather than round it.
- **Windows.** Each facade cell 4 m wide and one storey tall injects `window_strength`. The cell sits in the air
  voxel in front of the facade and the injection is scaled by the glass share of the cell. Every window is
  baked as lit; the hour scales the whole channel at draw time (section 5).
- An air voxel next to a solid one loses a further `wall_absorb` share each step. That keeps the light from
  creeping along walls and makes the pool read as falling on the ground.

**Cost, estimated:**
- Per iteration, each voxel does 7 reads and a multiply-add.
- Lamps: 1.6 M voxels x 18 iterations, about 200 M operations. Windows: 1.0 M x 6, about 40 M.
- Together about 0.2-0.4 s of one worker per city. That runs while the city's other meshes are being built on
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
| N10 | Do lit windows light the street and the facing buildings, or only the lamps? | Both | open (survey) |
| N11 | Lamp pools everywhere, seen from the air too (replaces N4's "pools only within 250 m")? | Everywhere | open (survey) |
