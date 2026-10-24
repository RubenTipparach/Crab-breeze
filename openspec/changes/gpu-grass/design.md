# Design: gpu-grass

Everything numbered here is `data/world/grass.ron` (code defaults in `crates/cb-core/src/grass.rs`).
Costs are **estimates** until the benchmark measures them on the owner's machines (section 6); a
cloud session or this Mac's worktree measured nothing.

## 1. Best practice, and what Crab Breeze takes

Sources read: Ghost of Tsushima's grass talk (Eric Wohllaib, GDC 2021), its open reimplementations
(`2Retr0/GodotGrass` among them), and how Bevy 0.20 draws a custom material (its `Material` trait,
`MeshTag`, `ShaderBuffer`, `NoAutoAabb`, the `bevy_pbr` WESL modules, read from the 0.20.0 sources).

| Practice | In Crab Breeze |
| --- | --- |
| Blades built on the GPU, nothing per blade on the CPU | The vertex shader builds each blade from `(blade index, vertex in blade, level)`, the only vertex attribute. No entity, transform or instance row per blade. |
| A blade is a curved strip narrowing to a tip | `segments` segments (`2s + 1` vertices, `2s - 1` triangles: pairs up the blade, one tip vertex), width `1 - t^taper` of the root's. |
| Bent along a Bezier curve | Quadratic: root, a middle point at `mid_height` of the height (moved toward the tip for a soft blade: `1 - stiffness`), and the tip set by the lean, the wind and any press, its length kept to the blade's height. |
| Wind as a scrolling noise field | Value noise at `gust_per_m` cells a metre scrolled along `wind_heading_deg` at `wind_speed_m_s`, from Bevy's `globals.time`; a steady push `wind_push` of which `gust_share` comes and goes; each blade's own sway `sway` at `sway_hz` with a random phase. |
| Variety from a hash of the blade's index | PCG hash of the blade's cell counted from the world's origin (the same blade whatever tile or level draws it): jitter in its cell, height, facing, lean, sway phase, dry or green. |
| Clumps | Cells of `clump_m`; a clump's own hash pulls its blades' height and facing (`clump_strength`) and makes whole clumps dry. |
| Edge-on blades thickened | The width direction turns toward square-on to the eye by `edge_thicken` times how edge-on the blade is. |
| Rounded normals | The strip's normal tilted across its width by `normal_round`, so a blade shades like a curved leaf. |
| Colour root to tip | `root_colour` to `tip_colour` along the blade, dry blades toward `dry_colour`, the root darkened to `root_light`. |
| LOD: farther tiles fewer, wider blades, fewer segments | Section 3. A farther level's blades are a subset of a nearer one's (strides divide), and the blades it drops shrink away over `crossfade_m` before the boundary, while the kept ones widen to the next level's width: no pop when a tile changes level. |
| Per-tile culling and LOD | 32 m tiles, each its own instance with a tight bounding box (`Aabb`, `NoAutoAabb`), frustum culled by Bevy; level chosen per tile with hysteresis. |
| No shadows cast, shadows received, PBR lit | `enable_shadows() = false`; the fragment shader fills Bevy's `PbrInput` (shadow receiver flag, fog flag) and calls `apply_pbr_lighting` and `main_pass_post_lighting_processing`, so grass is lit by the sun, the ambient light and every clustered light, and fogs with the world. |

## 2. Drawing: one draw per level, not per tile

Options weighed:

1. **A mesh per tile with baked positions.** Simple, but the blades become CPU data, and every tile
   is its own mesh: no batching. Rejected (the owner asked for GPU grass).
2. **A compute pass writing blades and indirect draws** (GodotGrass's way). The most scalable, and
   the way `gpu-instancing` plans to take trees, but it needs a custom render-graph node and render
   command in Bevy 0.20's render world. More code, more risk, for a first version.
3. **A shared blade mesh per level, one material, per-tile data in a storage buffer** (chosen). Every
   tile entity has `Mesh3d(level mesh)`, `MeshMaterial3d(the one grass material)`, a `Transform` at
   its corner and a `MeshTag` naming its slot. Same mesh and material means Bevy's binned opaque
   phase batches all tiles of a level into one instanced draw (multi-draw indirect where the
   platform has it). The shader reads the tile's corner from `get_world_from_local` and its slot from
   `get_tag`. Pure Bevy material API; nothing in the render world.

The slot holds `GrassLayout::words_per_slot` 32-bit words: the ground's height at the terrain's
lattice points over the tile (`(tile_m / terrain_cell_m + 1)^2` = 5 x 5 floats), then where grass
grows, one byte a `density_cell_m` cell (32 x 32 bytes, four a word): 281 words, 1,124 bytes. The
buffer has room for every tile within `radius_m + release_m` (`TileGrid::capacity`: 529 slots, 581
KiB). It is a `ShaderBuffer` asset of fixed size: when a tile changes, the CPU copy is handed over
whole and Bevy writes it into the same GPU buffer (`write_buffer`), so the material's bind group
stays valid.

A second `ShaderBuffer` of `MAX_PUSHERS` (8) spheres holds the bodies pressing grass flat, rewritten
each frame (128 bytes). Tuning the shader reads once is the material's uniform.

Grass is on render layer 2 (`GRASS_LAYER`), which the pilot's eyes see (with layer 0) and the six
screen cube cameras do not: they only light the cabin, and a face looking down would otherwise cost
as much grass as the eye's own view.

Grass is out of the depth prepass (`enable_prepass() = false`): its prepass would need its own
vertex shader, and nothing reads grass depth (the water reads the prepass for the ground under it,
where no grass grows).

## 3. Levels of detail, and what is on screen

Tile 32 m, nearest-level blade spacing 0.25 m: 128 blade cells a side.

| Level | Tile nearer than | Stride | Blades/m² | Blades a tile | Segments | Vertices a blade | Triangles a blade | Vertices a tile | Triangles a tile | Width |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 0 | 25 m | 1 | 16 | 16,384 | 4 | 9 | 7 | 147,456 | 114,688 | 1.0 x 4.5 cm |
| 1 | 110 m | 2 | 4 | 4,096 | 3 | 7 | 5 | 28,672 | 20,480 | 1.7 x |
| 2 | 300 m | 4 | 1 | 1,024 | 2 | 5 | 3 | 5,120 | 3,072 | 2.8 x |

(`the_blade_meshes_have_the_counts_in_the_design` pins these.) A tile's level is set by its nearest
point's distance from the eye (across the ground and up to the eye), with 6 m of hysteresis.

Tiles in reach, eye on the ground (the tiles whose nearest point is within `R`: about
`(pi R^2 + 4 R t + t^2) / t^2`; land everywhere, no city or sea):

| Level | Tiles (whole circle) | Blades | Vertices | Triangles |
| --- | --- | --- | --- | --- |
| 0 | 6 | 98,304 | 884,736 | 688,128 |
| 1 | 46 | 188,416 | 1,318,912 | 942,080 |
| 2 | 263 | 269,312 | 1,346,560 | 807,936 |
| All | 315 | 556,032 | 3,550,208 | 2,438,144 |

On screen: the pilot's lens is 75 degrees vertical, about 107 degrees across at 16:9, so the far
rings are about 30 percent in view and the near tiles (round the eye) about two thirds:

| Level | Tiles drawn | Blade slots | Vertices | Triangles |
| --- | --- | --- | --- | --- |
| 0 | about 4 | 65,536 | 589,824 | 458,752 |
| 1 | about 18 | 73,728 | 516,096 | 368,640 |
| 2 | about 84 | 86,016 | 430,080 | 258,048 |
| All | about 106 | about 225,000 | about 1.54 million | about 1.09 million |

**Draw calls: 3** (one instanced draw per level), in the main pass only: none in the shadow
cascades, none in the prepass, none in the cube cameras or the cockpit pass. Blade slots where no
grass grows (a road, a city, water, past the fade) still run the vertex shader and collapse to a
point outside the view, so they cost vertex work but no triangles. Tiles with no grass at all
(downtown, the sea) have no entity.

## 4. The ground under the grass: one height query

- **Heights.** A tile's corner is on the terrain's 8 m lattice (tiles are counted from the terrain's
  first sample and `tile_m` must be a multiple of `terrain_cell_m`: `GrassConfig::validate_with`).
  The bake reads the 5 x 5 lattice heights with `Heightfield::sample` and the shader interpolates
  them on the same two triangles a cell as `Heightfield::height_at`; on the CPU that rule is now one
  function, `terrain::cell_height`, shared by the terrain, the dug ground and the grass bake, and the
  shader repeats it (the one copy on the GPU). `a_tiles_heights_are_the_grounds_heights` checks the
  bake against `height_at` at random points. No terrain noise is written in WGSL.
- **The drawn ground is the lattice.** Inside 700 m (less 64 m of hysteresis) the ground is drawn
  at full detail (`terrain_lod.ring_m[0]`), so a blade's root is on the drawn surface; the grass's
  reach (`radius_m + release_m` = 348 m) must stay inside it (validated).
- **Where grass grows**, 0 to 255 a metre cell, baked off the main thread (`bake_tile`):
  - bare where the shared cover rule says so (`WorldPlan::cover_at`, now also the trees' rule):
    inside a city except its parks (2 m inside a park's edge), within 3 m round a city, within 1.5 m
    of a highway's paved edge, near a bridge, within 0.5 m of a building;
  - bare on the beach (ground under `beach_m` = 2.5 m above the sea, the ground's own sand band) and
    in water (the one water query, `Waters::water_at`);
  - thinning on slopes from a rise of 0.6 to none at 1.1 (`slope_bare`), the slope from the
    lattice's own triangles.
- **Craters.** The deformation is not carried into the heights: blades stand on the undug lattice.
  Instead, on the main thread, a tile's density is masked to zero wherever a crater changed the
  ground by more than `crater_clear_m` (5 cm), when its bake lands and again for every tile a new
  blast reaches (`CraterConfig::reach_m`, the reach `Deformation::dig` uses). So a crater clears the
  grass it dug, and no blade floats over a bowl or sinks into a rim; the scorched ground shows.
  Masking costs one `delta_at` a cell: 1,024 hash lookups a tile, a few tiles a blast.

## 5. Streaming

Each frame, after the camera is placed (`stream_tiles`): take finished bakes (mask craters, write
the slot, spawn or update the entity, or leave a bare tile without one); mask tiles a new blast
reached; list the tiles within `radius_m` nearest first (`TileGrid::within`), set each one's level
(swapping its `Mesh3d` handle when it changes) and start up to `bakes_per_frame` (6) bakes on the
async compute pool; let go of tiles past `radius_m + release_m`; and if any slot changed, hand the
buffer over. Then `press` writes the eight bodies nearest the eye.

A bake reads 25 lattice heights (each a few octaves of noise) and runs the cover, water and slope
tests on 1,024 cells: estimated 1 to 3 ms of one worker thread a tile. At the start the 315 tiles take
about 53 frames to start (6 a frame) and fill nearest first; walking (1.5 m/s) or skating (up to
about 30 m/s) needs a row of about 20 tiles every 1 to 20 s.

## 6. Costs (estimates, pending the benchmark)

| Item | Estimate | How |
| --- | --- | --- |
| Draw calls | 3 | one instanced draw per level |
| Vertices shaded | 1.5 million | section 3 |
| Triangles | 1.1 million | section 3 |
| GPU, RTX 3070, 1080p | 1.0 to 1.6 ms | vertex 0.4 to 0.8 ms (about 300 instructions a vertex, the stage bound by vertex fetch and primitive setup), raster 0.2 to 0.3 ms for small triangles, fragment 0.3 to 0.5 ms (grass on about 45 percent of 2.07 million pixels, overdraw about 2, quad waste on thin triangles, Bevy's clustered PBR) |
| GPU, Apple M4 Max, 1080p | 1.3 to 2.5 ms | the tiler bins every triangle before shading (vertex-heavy work is the weaker side of a tile-based GPU), fragment cheaper than the 3070's thanks to hidden surface removal |
| CPU, main thread | 0.2 to 0.5 ms | about 315 tile entities visited by visibility, extraction and queueing in one view (the per-mesh costs `gpu-instancing` measured), the stream's bookkeeping, a 581 KiB copy on a frame a tile changed |
| CPU, workers | 1 to 3 ms a tile baked | off the main thread |
| GPU memory | 3.9 MB meshes + 0.6 MB buffer | level meshes: 1.77 + 0.34 + 0.06 MB positions, 1.38 + 0.25 + 0.04 MB indices |
| Upload | 581 KiB on a frame any tile changes; 128 bytes every frame | |

`CB_NO_GRASS=1` leaves the plugin out entirely (no entities, no pipeline, no buffers): the benchmark
suite runs it as a variant against the default, the way it runs `CB_NO_SHADOWS` and
`CB_NO_WATER_FX`.

## Seeing it: `--start meadow` (owner, 2026-10-24: "Screenshots with grass?")

The takes start in a city, where no grass grows, and fly high, beyond its reach. `--start meadow`
moves the start to the nearest open grassland: rings searched out from the city start every 50 m
to 6 km, the first spot where grass grows thick (density at least 0.8) at every point of a 5 x 5
grid across 80 m. The mech stands there and the pilot 30 m behind it, so any take (`--showcase
world --start meadow`) walks, boards and strides through grass. `cb_core::grass::meadow_start`,
tested on seed 42.

## Open questions

| # | Question | Provisional answer | Who decides |
| --- | --- | --- | --- |
| 1 | Is 300 m the right reach, and 16 / 4 / 1 blades a square metre the right density? | As built; the benchmark sets the budget. If grass costs more than about 1.5 ms on the 3070, the first lever is level 0's reach (25 m) and level 1's (110 m). | owner, after the benchmark |
| 2 | Cull blades in a compute pass (indirect draws, no wasted vertices on bare ground or behind the eye)? | Not yet: the instanced draw is three draws already. Revisit if the benchmark shows the vertex stage dominating, together with `gpu-instancing`'s tree pass. | after the benchmark |
| 3 | Grass in the screen cube cameras (the cabin's light)? | No: it would roughly double the vertex cost for a light the pilot cannot see grass in. | Claude, provisional |
| 4 | Should a mech leave the grass flattened where it walked (a trail)? | Not now: it needs per-blade or per-texel state; a trail texture per tile is the cheap way. Skating trails (survey M4) may want it. | survey, later |
| 5 | Should grass draw past 300 m as a cheaper far layer (impostor cards or a noise in the ground's colour)? | The ground's colour carries it for now. | after a playtest |
| 6 | Colour: does the grass match the ground's grass colour at the fade? | Tuned by eye in a playtest; the ground and blade colours are both data. | playtest |
