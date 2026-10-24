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

## 3. Cost (to be measured)

Expected: the tree entities (about 4,000-9,000 near the pilot) leave the ECS; the render world's
per-mesh CPU work drops by that share; the GPU adds one small compute pass per view (well under
0.1 ms at 100 k instances on the RTX 3070). This is what lets the woods be dense and reach the fog
again (owner: "More trees please").

## Open questions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| T6 | How far and how dense the woods go once trees are on the GPU | Back to 9.5 m sites drawn to 2.5 km, measured | provisional (survey T6) |
