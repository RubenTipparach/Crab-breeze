# Design: gpu-instancing

## 1. Tree arrays (`cb-core::world::forest`, pure)

`Forest` keeps its trees as structure-of-arrays: `x: Vec<f32>`, `y: Vec<f32>`, `z: Vec<f32>`,
`yaw: Vec<f32>`, `height_m: Vec<f32>`, `species: Vec<u8>`, `tilt_rad: Vec<f32>`,
`tilt_dir: Vec<f32>`, `state: Vec<TreeState>`, by stable tree id within a tile. Falling is one pass
over the falling rows (`tilt += rate * dt`, clamped), written so the compiler vectorises it; the
changed ids go to a dirty list. Tests: the forest's existing tests, plus
`a_falling_tree_updates_only_its_own_row`.

## 2. GPU side (`cb-app::tree_draw`, after swarm-demo's `swarm.rs`)

- One storage buffer of packed instances (`[f32; 8]`: x, y, z, yaw, height, species, tilt,
  tilt_dir), grown in tiles; a tile's rows are written with `write_buffer` when the tile is grown and
  when a row is dirty.
- A compute pass per view (`Core3d`, before the prepass and the shadow passes) tests each instance
  against the view's frustum and draw radius and appends visible indices per species, with atomics
  into `DrawIndexedIndirectArgs`.
- Drawing: a `SpecializedMeshPipeline` over Bevy's `MeshPipeline` (lit, fogged, shadowed) whose
  vertex shader reads the instance by `instance_index` from the visible list and builds the model
  matrix (yaw, scale by height, tilt about `tilt_dir`). Render commands queue one item per species
  into `Opaque3d`, the prepass and `Shadow`, each a `draw_indexed_indirect`.
- Swarm-demo draws into `Transparent3d` only; trees need the opaque, prepass and shadow phases, so
  this is new work, and Bevy 0.20's phase APIs differ from 0.18's (read the 0.20 docs first).

## 2a. Step 1 as built: trees merged per tile (2026-10-24)

Before the GPU path, each grown tile's trees are one merged mesh per species in world space (about
24 x 24 sites a tile), built a few tiles a frame nearest first. A falling tree leaves its tile's
mesh and is drawn on its own while it moves; when it lands its tile is rebuilt. Measured uncapped on
the RTX 3070 over the defence take:

| | Entities | Meshes drawn | Frame (5 s medians) |
| --- | --- | --- | --- |
| a tree entity each | 16,088-19,524 | 4,810-7,631 | 6.6-10.9 ms |
| trees merged per tile | 7,422-8,816 | 2,335-3,334 | 6.0-11.9 ms |

Entities and meshes halved, frame time within the noise: trees were not the main CPU cost. A new
trace of this build decides the next step before more batching.

### The trace after batching (trace_chrome, 35 s of the defence take, uncapped, per frame)

With tracing on: 8.8 ms a frame. Render world 8.2 ms (the bottleneck; the main app runs beside it at
5.4 ms, physics 0.7 ms of it). Of the render world: the render graph 3.3 ms, of which submission is
1.8 ms (33 command buffers a frame, 1.1 ms in the driver); camera schedules 0.68 ms (eye) and
0.53 ms (cockpit pass); light clustering 1.0 ms (both views); shadow passes for both views;
visibility about 1.4 ms. Mesh count no longer leads: the fixed cost of each view does. The cockpit
pass (a second full camera) is now one of the largest single costs. Options, for the owner (T7):
keep the second camera and slim it (no shadow pass, no clustering beyond its own lights), or draw
the cockpit in the eye camera with a depth bias that keeps it in front (one view; but then world
lights reach it, so its own lighting becomes emissive and baked rather than lit).

## 2b. Buildings: element state in the vertex stage (from the building-research session)

Each building (later each block) is one mesh carrying a per-vertex element index (a `u32` vertex
attribute). A per-building storage buffer holds each element's state: standing bit, stress 0..1,
damage stage. A material extension on `StandardMaterial` reads it in the vertex stage of the main,
prepass and shadow pipelines and collapses a broken element's triangles to a point, so a damaged
building needs no mesh rebuild, and frame and dressing can share one per-block mesh. Structure,
falls and rubble stay in `cb-core` (`Structures`, `Frame`, `Deformation::pile`).

## 2c. Hitches in the defence take (measured 2026-10-24, RTX 3070, plain release runs of 130 s)

The frame graph and `hitch:` lines (frames over 50 ms), with `slow:` lines from systems timed past
4 ms (`perf_hud::slow`), found two causes:

- **Batched trees were invisible.** Each merged tile mesh sat at the world origin with world-space
  vertices, so its `VisibilityRange` measured from the origin and hid every tile more than the draw
  radius from it (the start is 2.3 km out). The 2a numbers above counted trees that were not drawn.
  Fixed: a tile's mesh sits at its first tree, vertices relative to it.
- **Crater tiles were meshed on the main thread**: `redraw_craters` took 56-262 ms per crater (a 129
  x 129 tile sampled the terrain's noise five times a vertex). Fixed: heights sampled once a grid
  point (five times fewer), and the tile meshed on a worker thread from a snapshot of its
  neighbourhood (`Deformation::around`, tested to mesh the same). Tree tiles are merged on worker
  threads too (`show_trees` took 37-75 ms a frame building them).

| Run | Frames over 50 ms | Worst |
| --- | --- | --- |
| before | 74 | 435 ms |
| crater and tree work off the main thread | 28 | 69 ms |

Still open: combat runs at about 60 fps (16-18 ms) against 120 in calm stretches, CPU-bound with the
GPU at 0.4 ms. None of the game's own systems passes 2 ms (`step_mech` is the largest at about 1.9 ms),
so the time is in Bevy's per-frame work that grows with combat (particles, lights, render
extraction). A traced build's own systems' numbers are not trusted: under `trace_chrome` the
explosion handler and the frame graph read 7 ms each, and under 1 ms in a plain run. The next step
is an A/B with the effects left out (`CB_NO_PARTICLES`, `CB_NO_FX_LIGHTS`, `CB_NO_SHADOWS`).

## 3. Cost (to be measured)

Expected: the tree entities (about 4,000-9,000 near the pilot) leave the ECS; the render world's
per-mesh CPU work drops by that share; the GPU adds one small compute pass per view (well under
0.1 ms at 100 k instances on the RTX 3070). This is what lets the woods be dense and reach the fog
again (owner: "More trees please").

## Open questions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| T6 | How far and how dense the woods go once trees are on the GPU | Back to 9.5 m sites drawn to 2.5 km, measured | provisional (survey T6) |
| T7 | The cockpit pass costs a second full view | Keep it (nothing clips through, its own lighting) and slim it: no shadow pass, its own light clustering only | provisional (survey T7) |
