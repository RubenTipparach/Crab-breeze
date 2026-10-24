# Design: island-world

Numbers below come from the shipped `data/world/world.ron` and seed 42, printed by the tests in
`crates/cb-core/src/world/tests.rs` (run them with `-- --nocapture`). Frame times are not measured:
the owner is using the Mac and no game may be launched (CLAUDE.md section 8). Every cost here is an
expectation until an owner-approved playtest measures it (`perf-measure`).

## 1. The ground at levels of detail (cb-core `world::lod`, cb-app `ground`)

**Chunks and steps.** The ground is 80 x 80 chunks of 64 x 64 cells (8 m cells, 512 m chunks). A
chunk `d` metres from the eye wants step `step_for(d)`: 1 within 700 m, 2 within 1,400 m, then 4, 8,
16 and 32 past 11,200 m (`terrain_lod.ring_m`). Each ring doubles both the distance and the cell, so
a drawn quad subtends about the same angle everywhere: 16 m at 1,400 m is about 0.65 degrees, about
8 px at 1080p. The distance is to the chunk's nearest point and up to the eye's height above sea
level, so flying high coarsens the ground under the mech.

**Hysteresis.** `next_step(current, d)` moves to a coarser step only once the chunk is
`hysteresis_m` (64 m) past the ring's edge, and to a finer one only once it is 64 m inside, so a
mech standing on a ring's edge does not make the ground flicker. Test:
`a_chunk_on_a_ring_edge_does_not_flicker`.

**The quadtree (fewer draw calls).** Drawing 6,400 chunks as 6,400 meshes is 6,400 draws (see
section 7 for why each distinct mesh is a draw on Metal). Instead the chunks are the leaves of a
quadtree whose 25 roots are 8,192 m squares (16 x 16 chunks, `terrain_lod.max_level: 4`). A node of
level L covers 2^L chunks a side and is one mesh of 64 x 64 quads drawn at step 2^L, so every node
costs the same 8,704 triangles (8,192 plus its skirt). `lod::select` walks down from the roots and
splits a node while its distance wants a finer step than its own (through `next_step`, with the
nodes split last frame, so the hysteresis holds), or while it holds a crater chunk (craters are
drawn at full detail). Tests: `the_drawn_nodes_cover_the_ground_once`,
`nodes_near_the_eye_are_fine_and_far_ones_coarse`, `a_crater_chunk_is_drawn_at_full_detail_however_far`,
`a_split_node_on_a_ring_edge_stays_split`.

| Eye | Ground meshes | Triangles (before culling) |
| --- | --- | --- |
| On foot at the start | 88 | 766,000 |
| Flying 1,500 m over the centre | 52 | 453,000 |
| On a beach | 85 | 740,000 |
| (6,400 chunks, one mesh each, for comparison) | 6,400 | 459,000 |

The quadtree draws more triangles than chunks would (far nodes cannot drop below step 16, and
every node keeps 64 x 64 quads), and 70 times fewer meshes. With a 75 degree vertical field of view
(113 degrees across at 16:9) about a third of the nodes are in view: about 30 draws and 250,000
triangles for the ground in the main pass, plus the shadow cascades (which reach 900 m: about 10
nodes in 4 cascades).

**Seams.** Every node hangs a skirt `4 + 2 x step x 8 m` deep under its edges, wound to face out of
the node (test: `a_skirt_hangs_below_every_edge_and_faces_out`), so where a fine node meets a
coarse one the gap shows the skirt, never the sky. The quadtree is not kept 2:1 balanced; a skirt
is deep enough for the next step up, and a level jump of two is rare (the rings double).

**Building meshes.** A node's mesh is built off the main thread (`AsyncComputeTaskPool`), at most
6 started a frame, nearest first; a node stays on screen until what replaces it is ready (a split
waits for all four children, a merge for the parent: tests in `cb-app/src/ground.rs`), so the
ground never opens and no frame waits on a build. One build samples 67 x 67 heights (the node plus
a one-sample border for normals) and costs 0.3 ms (measured by
`ground_chunks_cost_less_the_coarser_they_are_drawn`, test profile and release alike; a sample is
about 70 ns). At 120 m/s the eye crosses a 512 m chunk every 4.3 s and a few nodes per level change:
about 5 builds a second, under 2 ms a second of worker time. The first frame builds the 88 starting
nodes on the main thread: about 30 ms at startup.

**Colliders** (Avian, for debris and shot-off parts only: the mechs and the pilot walk on
`height_at`) are full-detail triangle meshes of single chunks, built off the main thread (2 a
frame), only for chunks within 400 m of the eye or of a dynamic body, and let go past 600 m. A
chunk dug by craters gets its collider from the dug ground (`delta_at` added per vertex), so debris
no longer falls through crater holes.

**Craters** keep their 64 m tiles at 0.5 m, drawn over holes cut in the full-detail chunk; the
quadtree forces the chunks holding them to level 0 however far they are.

**The world's edge.** Past it, four flat strips of sea floor at exactly the floor's depth run out to
the far plane; the floor's noise fades out over the last 1,500 m inside the edge, so the terrain
meets them exactly (test: `the_open_sea_floor_is_between_twenty_and_sixty_metres_down`, last
assertion).

## 2. The island (cb-core `world::terrain`)

**Heights are a function, not a store.** At 8 m, a 40,960 m world is 5,121 x 5,121 = 26.2 million
samples: 105 MB as `f32`, and about 8 s to compute up front (the old generator spent about 300 ns a
sample on noise, cities and highways). Instead `Heightfield` keeps no heights: a sample is a pure
function of its lattice position, worked out when asked for, and `height_at` interpolates the four
around a point on the same two triangles per cell the full-detail mesh draws (tests:
`height_at_matches_the_grid_at_every_sample`, `height_at_lies_on_the_drawn_triangles`). It stays the
one ground query for the drawn ground, the colliders, the pilot, the mechs, the trees and the
rivers (CLAUDE.md 6.1). A sample is about 70 ns; a `height_at` about 0.3 microseconds. A stored grid
is a cache this design can add later if a profile asks for it.

A sample is, in order: the island's shape, then each city's flat ground, then each highway's grade
(both found through a 512 m index, so a sample visits only the few that reach it), then the river
channels (through a 64 m index of river segments).

**The shape.** With `r` the distance from the centre plus up to `coast_noise_m` (2,500 m) of coast
noise (3 octaves, 6 km first wavelength), `past = r - land_radius_m` (13,000 m) and
`t = smoothstep(0, shelf_m, past)` (3,000 m shelf):

```
land = land_height_m (30) + highland_m (90) x (inland share)^2 + hills (35 m, 1,100 m, 4 octaves)
sea  = -sea_depth_m (38) + floor_noise_m (9) x floor noise x edge fade
h    = sea_level_m (0) + land x (1 - t) + sea x t
```

| Number | Seed 42 |
| --- | --- |
| World | 40,960 m square (1,678 km2) |
| Land | 682 km2, highest ground 151 m |
| Open sea floor | 29 to 47 m deep (test asserts 20 to 60 m at 2,000 points) |
| Shore | the floor never climbs faster than 1 in 4 below sea level (test: `the_shore_shelves_gently_enough_to_walk_out_of_the_sea`); a mech walks up 1 in 2 |
| Lakes | inland ground below sea level (hills dip below the 30 m land height) |

**Water.** `Waters::water_at(p, ground)` reports the sea (surface 0 m, no flow) wherever the ground
is below sea level, anywhere, past the world's edge too, since `height_at` goes on past it (test:
`the_island_has_land_in_the_middle_and_sea_at_the_edges`). Rivers start at the highest of 24
spots within 9.1 km of the centre and run downhill until they reach the sea or a lake; a river's
surface never falls below sea level. River channels are no longer carved into a stored grid but
applied in the sample, and they no longer cut a highway's flat band except under a bridge (seed 42
crosses its highways 69 times: the rivers wind; a bridge per crossing).

**Cities.** 24 placed (up to `city_count`), 2,600 m apart with 600 m of open ground, only where the
square and its blend stand inland on ground at least 2 m above the sea. The first, the capital, is
the largest size and is placed within 2.5 km of the centre; the game starts in it (test:
`the_start_is_in_the_capital_near_the_islands_centre`), and the three enemies of
`data/world/encounters.ron` (420 to 700 m ahead) start on dry land (test:
`every_encounter_starts_on_dry_land`). Highways: a minimum spanning tree, 23 of them, 68 km; a
straight highway across a bay is graded as a causeway. Buildings: 6,100.

**Generation** takes 7 ms (the plan: cities, highways, rivers, indexes); no step walks the ground.

## 3. Trees grown near the bodies (cb-core `world::forest`, cb-app `world`)

The old world grew 41,571 trees over 16.8 km2. At the same density the island would hold about 1.7
million: 70 MB of trees, 14 million sites to decide, and an entity each. So trees grow by tile:

- A tile is 24 x 24 sites (264 m); a tree is a pure function of its site (seeded by the site, not
  by order), and its id is the site's index, so a tile grown twice is the same tile and ids never
  depend on what grew first (test: `trees_grown_in_any_order_are_the_same_trees`).
- Each fixed step, before the pilot and the mechs move (`SimSet::World`), `Forest::keep_around`
  grows every tile within 400 m of a body at once, then up to 2 more within 1,500 m, nearest
  first, and lets go of tiles beyond 2,000 m of every body. It reads only the simulation's positions,
  so it is deterministic. A felled tree in a tile let go is remembered and lies felled when it grows
  again (test: `a_felled_tree_stays_felled_when_its_tile_is_let_go_and_grown_again`). A blast far
  from every body grows the tiles it reaches first.
- Kept round the start: 14,044 trees in 126 tiles, grown in 10 ms. One body keeps at most about
  130 tiles (test: `growing_trees_is_bounded_by_the_bodies_not_the_island`), so memory stays near
  1 MB per body however big the island is.

**Drawing.** Tiles within 1,200 m of the eye are put on screen (6 tiles a frame, nearest first) and
taken off past 1,350 m: about 87 tiles, 9,700 tree entities, against 41,571 before. Every tree of a
species shares one mesh and all share one material, so Bevy draws each species as one instanced
batch: 3 draws in the main pass. Only tiles within 300 m cast shadows (about 1,200 trees; the
others carry `NotShadowCaster`, switched per tile as the eye moves): 3 draws per shadow cascade. Only
falling trees have their transforms touched (`redraw_trees` reads the ids that moved this step).

## 4. Indexes

Every "what is near here" lookup now uses one compressed-rows cell index (`world::grid::CellIndex`):
four bytes a cell and four per entry, instead of an empty `Vec` (24 bytes) per cell.

| Index | Cells | Old | New |
| --- | --- | --- | --- |
| Buildings, 64 m | 641 x 641 | 9.9 MB | 1.6 MB plus ids |
| River segments, 64 m | 641 x 641 | 9.9 MB | 1.6 MB plus ids |
| Trees, 32 m (old) | 1,281 x 1,281 | 39 MB | per tile: 1.2 KB of slots |
| Flats, 512 m | 80 x 80 | none | 26 KB |
| Clouds, 2 km tiles | 27 x 27 | none (every puff tested every frame) | 3 KB |

## 5. Precision, the far plane and the haze

**f32 is enough on the island; no floating origin.** At 20 km from the origin an f32 has a step of
1.95 mm (2^-9 m), fine for the simulation: a body at 1.4 m/s moves 23 mm a step, and only speeds
under 6 cm/s (1 mm a step) would round away at the edge, under 3 cm/s at 14 km. Rendering is the
real question: Bevy transforms each vertex in f32 world space (`clip_from_world x world_from_local x
local`), so geometry near the eye at large coordinates shimmers. Measured with a scratch program
(glam, the same f32 pipeline against an f64 reference, a cockpit panel 0.6 m from the eye, 1080p,
75 degree field of view, 4,000 positions of a moving mech):

| Distance from the origin | Mean error | Worst error |
| --- | --- | --- |
| 2,048 m | 0.02 px | 0.12 px |
| 8,192 m | 0.18 px | 0.30 px |
| 14,000 m (the coast) | 0.41 px | 0.58 px |
| 20,480 m (the world's edge) | 0.72 px | 1.29 px |
| 28,960 m (the corner) | 0.72 px | 2.17 px |

Under a pixel everywhere on the island, and the start is within 2.5 km of the centre. A floating
origin (or camera-relative rendering) would have to move every pose system, the mech's among them,
for a sub-pixel gain; it is left for the day the sea beyond the island is played in (open question
I4).

**The far plane** is 40 km (`view.far_m`). Bevy 0.20's `PerspectiveProjection` builds
`perspective_infinite_reverse` (bevy_camera 0.20.0, `projection.rs`): reverse Z with an infinite
far plane, so `far` only culls, and a larger one costs no depth precision. **Clear-air visibility**
is 9 km (`view.clear_visibility_m`, was 3,500 m); the haze is the sky's colour, so the sea meets the
sky with no visible end.

**The sea sheet** is one flat square at sea level, 2 x (40 km + 2.4 km) across, that follows the eye
in steps of 2,400 m (100 ripple tiles, so its UVs never slide). It uses `water_material` like every
other water surface; its geometry and placement are kept apart (`ocean_sheet`, `follow_ocean`) from
the water's look, which another change owns.

## 6. Clouds

`Clouds::density_at` visited every puff (288 then; 6,700 on the island). Puffs are now indexed by
2 km tile and the query visits only the tiles within a puff's reach (test:
`the_indexed_density_is_the_density_over_every_puff`); sprites are put up by tile within 9 km of the
eye. Coverage is 0 for now (the owner's GPU load: large, low, alpha-blended sprites overdraw); the
clouds change redoes them cheaply.

## 7. Batching on the owner's Mac (Metal), from the source

- **GPU preprocessing is on.** `GpuPreprocessingSupport::from_world` (bevy_render 0.20.0,
  `batching/gpu_preprocessing.rs`) picks `Culling` when compute shaders are available,
  `INDIRECT_FIRST_INSTANCE | IMMEDIATES` are supported and a stage allows 12 storage textures and 10
  storage buffers. wgpu-hal 30's Metal adapter reports `IMMEDIATES` and sets
  `INDIRECT_FIRST_INSTANCE` on GPUs with indirect draw dispatch (Apple silicon), so on the owner's Mac
  Bevy should build its batches on the GPU with GPU frustum culling. To be confirmed by the log line
  "GPU preprocessing is fully supported on this device" at the next owner-approved run.
- **Multi-draw indirect is one Metal draw per entry.** Bevy issues one `multi_draw_indirect` per
  batch set, but wgpu-hal's Metal `draw_indexed_indirect` loops over the entries and issues one
  `drawIndexedPrimitives` each, and `draw_indexed_indirect_count` is an empty TODO (so
  `MULTI_DRAW_INDIRECT_COUNT` is not offered and Bevy uses the plain path). The CPU cost of encoding
  is small, but the GPU still sees one draw per entry.
- **An entry is one (mesh, material) pair, instanced.** Entities that share both are one entry
  however many there are. So the draws are the distinct meshes in view: the ground nodes (one each),
  one per tree species, one per building (6,100 buildings each have their own box mesh: the next
  thing to merge, out of scope here), the roads, kerbs and lane marks (one each), the rivers (one
  each).

Without GPU preprocessing (a GL backend, or an old device), Bevy batches on the CPU: entities sharing
mesh and material still go into one instanced draw, so the same sharing pays off there too.

## 8. Costs (expected; frame time pending an owner-approved playtest)

| Item | Count or cost |
| --- | --- |
| Ground meshes | 88 (about 30 in view), 766,000 triangles (about 250,000 in view) |
| Ground mesh builds | 0.3 ms each off the main thread, about 5 a second at 120 m/s; 30 ms at startup |
| Ground colliders | about 3 chunks round the eye, 8,192 triangles each, 2 builds a frame at most |
| Trees | 14,000 kept, 9,700 drawn as entities, 3 draws (plus 3 per shadow cascade, 1,200 trees) |
| Tree growth | 10 ms at startup; 2 tiles a step at most beyond the urgent ones (about 0.1 ms each) |
| Sea | 1 sheet, 2 triangles; far floor 1 mesh, 8 triangles |
| Generation | 7 ms; memory: no height store (was going to be 105 MB), indexes about 4 MB |
| Frame time, p50/p95/p99 | not measured: needs the owner's go-ahead to run the game |

## Open questions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| I1 | "10x bigger": ten times the side (40,960 m, 100 times the area) or ten times the area (12,950 m)? | ten times the side, as the brief to this change read it | provisional |
| I2 | How deep is the open sea? | 29 to 47 m: a sunk 12 m mech is under, and can walk ashore | provisional |
| I3 | How far should the pilot see? | 9 km of clear air, 40 km far plane | provisional |
| I4 | Floating origin? | not now: under 0.6 px on the island (section 5); revisit if the open sea is played in | provisional |
| I5 | Highways across bays are graded causeways; should they be bridges? | causeways until a bridge model exists | provisional |
| I6 | Buildings are one mesh each (6,100 draws if all were in view) | merge per city block in the buildings change | provisional |
