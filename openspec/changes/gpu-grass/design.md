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
grows, one byte a `density_cell_m` cell (32 x 32 bytes, four a word), then how dry it is, the same
way (section 8): 537 words, 2,148 bytes. The
buffer has room for every tile within `radius_m + release_m` (`TileGrid::capacity`: 529 slots, 581
KiB before the dryness bytes, 1.1 MiB with them). It is a `ShaderBuffer` asset of fixed size: when a tile changes, the CPU copy is handed over
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

Tile 32 m, nearest-level blade spacing 0.2 m: 160 blade cells a side (section 7 raised the
density from 16 to 25 blades a square metre and level 0 from 4 to 5 segments).

| Level | Tile nearer than | Stride | Blades/m² | Blades a tile | Segments | Vertices a blade | Triangles a blade | Vertices a tile | Triangles a tile | Width |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 0 | 25 m | 1 | 25 | 25,600 | 5 | 11 | 9 | 281,600 | 230,400 | 1.0 x 6 cm |
| 1 | 110 m | 2 | 6.25 | 6,400 | 3 | 7 | 5 | 44,800 | 32,000 | 1.7 x |
| 2 | 300 m | 4 | 1.56 | 1,600 | 2 | 5 | 3 | 8,000 | 4,800 | 2.8 x |

(`the_blade_meshes_have_the_counts_in_the_design` pins these.) A tile's level is set by its nearest
point's distance from the eye (across the ground and up to the eye), with 6 m of hysteresis.

Tiles in reach, eye on the ground (the tiles whose nearest point is within `R`: about
`(pi R^2 + 4 R t + t^2) / t^2`; land everywhere, no city or sea):

| Level | Tiles (whole circle) | Blades | Vertices | Triangles |
| --- | --- | --- | --- | --- |
| 0 | 6 | 153,600 | 1,689,600 | 1,382,400 |
| 1 | 46 | 294,400 | 2,060,800 | 1,472,000 |
| 2 | 263 | 420,800 | 2,104,000 | 1,262,400 |
| All | 315 | 868,800 | 5,854,400 | 4,116,800 |

On screen: the pilot's lens is 75 degrees vertical, about 107 degrees across at 16:9, so the far
rings are about 30 percent in view and the near tiles (round the eye) about two thirds:

| Level | Tiles drawn | Blade slots | Vertices | Triangles |
| --- | --- | --- | --- | --- |
| 0 | about 4 | 102,400 | 1,126,400 | 921,600 |
| 1 | about 18 | 115,200 | 806,400 | 576,000 |
| 2 | about 84 | 134,400 | 672,000 | 403,200 |
| All | about 106 | about 352,000 | about 2.6 million | about 1.9 million |

The first pass (16 / 4 / 1 blades a square metre, 4 / 3 / 2 segments) drew about 225,000 blade
slots, 1.54 million vertices and 1.09 million triangles; this pass is about 1.7 times that.

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
- **The ground takes the grass's colour** (section 7, practice 2). Every ground vertex's colour
  moves toward `ground_tint_colour` by `ground_tint` times `cb_core::grass::ground_cover`, which is
  `grows_at` (the rule the blades grow by) with the crater rule (`crater_clears`) the bake's mask
  uses. So the ground is grass-coloured exactly where blades stand, roads and beaches keep their
  own colour, a crater's bowl shows its scorch, and far meadows past the grass's 300 m still read
  green. The terrain nodes compute it off the main thread as they build (`ground.rs`
  `grass_cover`), crater tiles with the dug ground's change. Measured in a release test run on this
  Mac (a timing instrument, not kept): `grows_at` costs 0.21 microseconds a call, so a 65 x 65
  node gains about 1 ms of worker time and the 90 nodes built at startup about 0.1 s.

## 5. Streaming

Each frame, after the camera is placed (`stream_tiles`): take finished bakes (mask craters, write
the slot, spawn or update the entity, or leave a bare tile without one); mask tiles a new blast
reached; list the tiles within `radius_m` nearest first (`TileGrid::within`), set each one's level
(swapping its `Mesh3d` handle when it changes) and start up to `bakes_per_frame` (6) bakes on the
async compute pool; let go of tiles past `radius_m + release_m`; and if any slot changed, hand the
buffer over. Then `press` writes the eight bodies nearest the eye.

A bake reads 25 lattice heights (each a few octaves of noise) and runs the cover, water and slope
tests on 1,024 cells: measured 0.19 ms of one worker thread a tile (release, this Mac, the
timing instrument above). At the start the 315 tiles take
about 53 frames to start (6 a frame) and fill nearest first; walking (1.5 m/s) or skating (up to
about 30 m/s) needs a row of about 20 tiles every 1 to 20 s.

## 6. Costs (estimates, pending the benchmark)

The first pass ran at the 120 Hz frame cap in the owner's meadow screenshots (GPU time was not
read). These are this pass's estimates, for section 3's counts and section 7's heavier shaders.

| Item | Estimate | How |
| --- | --- | --- |
| Draw calls | 3 | one instanced draw per level |
| Vertices shaded | 2.6 million | section 3 |
| Triangles | 1.9 million | section 3 |
| GPU, RTX 3070, 1080p | 1.8 to 3.0 ms | vertex 0.8 to 1.5 ms (about 450 instructions a vertex now: the Voronoi clump's nine cells add about 150; the stage is bound by vertex throughput and primitive setup, not arithmetic), raster 0.3 to 0.5 ms, fragment 0.6 to 1.2 ms (tall grass covers about 60 percent of the frame with overdraw 3 to 4, Bevy's clustered PBR plus one more shadow-map read for the translucency) |
| GPU, Apple M4 Max, 1080p | 2.5 to 4.5 ms | the tiler bins every triangle before shading, the weaker side of a tile-based GPU for vertex-heavy work; hidden surface removal keeps the fragment side cheaper than the 3070's |
| CPU, main thread | 0.2 to 0.5 ms | about 315 tile entities visited by visibility, extraction and queueing in one view (the per-mesh costs `gpu-instancing` measured), the stream's bookkeeping, a 1.1 MiB copy on a frame a tile changed |
| CPU, workers | 0.19 ms a tile baked (measured), about 1 ms a ground node more for its tint | off the main thread |
| GPU memory | 7.2 MB meshes + 0.6 MB buffer | level meshes: 3.38 + 0.54 + 0.10 MB positions, 2.76 + 0.38 + 0.06 MB indices |
| Upload | 581 KiB on a frame any tile changes; 128 bytes every frame | |

If the benchmark shows this over budget, the levers in order: level 0's reach (25 m) and segments
(5), the near density (`blade_spacing_m` 0.2 back to 0.25 is 25 to 16 blades a square metre),
level 1's reach (110 m), then the Voronoi clumps (a single-cell clump saves about a third of the
vertex arithmetic).

`CB_NO_GRASS=1` leaves the plugin out entirely (no entities, no pipeline, no buffers): the benchmark
suite runs it as a variant against the default, the way it runs `CB_NO_SHADOWS` and
`CB_NO_WATER_FX`. The ground's tint stays (it is the ground's colour, built with its meshes, and
costs no frame time), so the variant measures the blades alone.

## 7. Tall, lush grass (owner, 2026-10-24: "that seems kinda weak. Look up more best practices with tall grass")

The first pass's meadow screenshots (`docs/screenshots/grass/2026-10-24-meadow-01..03.jpg`): blades
ankle-high to a person next to a 12 m mech, pale ground between them, flat colour, a thin field.
Sources: GPU Gems 2 chapter 1 (Pelzer, "Rendering Countless Blades of Waving Grass"), the
hexaquo.at grass series, `2Retr0/GodotGrass` after Ghost of Tsushima, Orthmann et al. 2008
("Towards Real-Time Grass Rendering"), and the three.js procedural grass layered-wind write-up.

| # | Practice | In Crab Breeze (data key) |
| --- | --- | --- |
| 1 | Tall grass | Blades 0.6 to 1.6 m (`height_m`); 18 percent of clumps stand 1.3 times taller (`clump_tall_share`, `clump_tall_scale`), to about 2.1 m: a mech's ankle and knee, a pilot's waist and over. Wider (`width_m` 4.5 to 6 cm). Level 0 has 5 segments so the long blades curve smoothly; tall blades droop over at the tip (`droop` 0.35 of the height at the tallest, added to the lean, the stiff middle control point keeping the bend near the tip). |
| 2 | No bare ground | The ground under grass takes the grass's colour by the one rule (section 4). The biggest single lushness gain in every source. |
| 3 | Density | 25 blades a square metre near (`blade_spacing_m` 0.2), 6.25 and 1.56 farther, the far levels' wider blades kept. Section 3. |
| 4 | Voronoi clumps | Each blade finds its nearest clump seed in the 3 x 3 cells round it (`clump_m` 2.5 m); the clump shares height, lean, facing (blended as vectors) and hue (`clump_strength` 0.5), some clumps are tall, and each is a mound: blades at its edge are up to 30 percent shorter (`clump_edge_shrink`). |
| 5 | Fake ambient occlusion | The root is darkened to `root_light` (0.35), deeper by up to `ao_dense` (half) in dense, tall clumps, fading out over the lower `ao_height_share` (55 percent) of the blade. |
| 6 | Translucency | In the fragment shader, the first directional light (the sun) adds `translucent_colour` times its light times `max(0, dot(-view, light))^translucency_power` times `translucency`, more toward the thin tip, read through the sun's own shadow map so a blade in shadow does not glow. |
| 7 | Specular | Bevy's PBR specular at `reflectance` 0.35 and `roughness` 0.65 near, the roughness fading to `far_roughness` 0.95 between 20 and 120 m (`gloss_fade_m`), and the normals blending to straight up between 40 and 220 m (`normal_up_m`), so far grass shades like the ground and does not sparkle. |
| 8 | Colour variety | Each clump's tips move toward a second green (`hue_colour`) by a random share of `hue_variety`; 22 percent of blades are straw-dry, scattered one by one (`dry_share`); tips are lighter (`tip_lighten`). |
| 9 | Layered wind | Shares of the height at the tip, so all of it is weighted to the tip by the Bezier curve: a steady push (`wind_push`); a broad sway, a sine 70 m long rolling with the wind (`broad_*`); gust waves, value noise scrolled at `wind_speed_m_s` and sharpened by `gust_sharpness` into bands, lightening the blades they bend by up to `gust_brighten` (the bands seen rolling across a field); each blade's sway (`sway`, `sway_hz`); and a flutter across the blade growing with the square of the place along it (`flutter`, `flutter_hz`). |

Tests: the new counts (`the_blade_meshes_have_the_counts_in_the_design`), the ground tint
(`the_ground_is_tinted_exactly_where_grass_grows_and_not_in_a_crater`,
`ground_under_full_grass_takes_the_grass_tint_and_bare_ground_keeps_its_own`), the tall clump's
height (`tall_clumps_stand_about_two_metres`), and the shader compiled against Bevy 0.20's
`bevy_pbr` with naga, its uniform's size checked against the Rust struct.

## 8. Grass the colour of its ground, dry on sand (owner, 2026-10-27: "Grass color should better match ground color", "Grass in sand , you should get beige grass or dryer grass")

The stills showed bright pure-green blades (red about a sixth of the green) over an olive ground
(about sRGB 0.25, 0.37, 0.16: red two thirds of the green), and the same green grass on beaches and
desert edges. Two changes, both data with code defaults:

- **Blade colours in the ground's olive** (`root_colour`, `tip_colour`, `hue_colour`): the tips
  move from (0.05, 0.32, 0.04) to (0.085, 0.20, 0.04) linear, the clump hue to (0.12, 0.22, 0.035).
- **Dryness by biome** (`biome_dryness`, 0 to 1 a biome; Desert and Canyon 1.0, Beach 0.9, Savanna
  0.6, Cliff 0.4, Alpine 0.3, the rest 0). `GrassConfig::dryness(weights)` is the one rule: the
  biome weights at a point times their dryness. The tile bake writes it per density cell, one byte
  each, after the density bytes; the shader mixes each blade toward `parched_colour` (0.40, 0.31,
  0.15, darker at the root) by it. The ground under the grass takes the grass tint scaled by
  `1 - dryness`, so sand under dry grass stays sand.

Cost: 256 more words a slot (the slot from 1,124 to 2,148 bytes; 529 slots, about 1.1 MiB of
storage buffer), one more buffer read a blade. No measurement: a cloud session has no GPU.
Tests: the pack layout (`pack` writes heights, density, then dryness), the config's validation.

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
| 1 | Is 300 m the right reach, and 25 / 6.25 / 1.56 blades a square metre the right density? | As built; the benchmark sets the budget. If grass costs more than about 2 ms on the 3070, the levers are in section 6. | owner, after the benchmark |
| 2 | Cull blades in a compute pass (indirect draws, no wasted vertices on bare ground or behind the eye)? | Not yet: the instanced draw is three draws already. Revisit if the benchmark shows the vertex stage dominating, together with `gpu-instancing`'s tree pass. | after the benchmark |
| 3 | Grass in the screen cube cameras (the cabin's light)? | No: it would roughly double the vertex cost for a light the pilot cannot see grass in. | Claude, provisional |
| 4 | Should a mech leave the grass flattened where it walked (a trail)? | Not now: it needs per-blade or per-texel state; a trail texture per tile is the cheap way. Skating trails (survey M4) may want it. | survey, later |
| 5 | Should grass draw past 300 m as a cheaper far layer (impostor cards or a noise in the ground's colour)? | The ground's colour carries it for now. | after a playtest |
| 6 | Colour: does the grass match the ground's grass colour at the fade? | Blades retuned to the ground's olive from cloud stills (section 8); confirm by eye in a playtest. | playtest |
| 9 | Which biomes are dry, and how dry (section 8)? | Desert, Canyon 1.0; Beach 0.9; Savanna 0.6; Cliff 0.4; Alpine 0.3. | owner, playtest |
| 7 | The ground tint's colour against the ground's grass texture (the tint multiplies it), the translucency's strength and the gust bands' brightness | Set by reasoning, not by eye: tune in a screenshot pass. | playtest |
| 8 | Should the wind's heading and strength come from a weather system rather than data? | Data for now; no weather exists yet. | later change |
