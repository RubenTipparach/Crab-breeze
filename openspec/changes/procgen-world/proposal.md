# Proposal: procgen-world

## Why

The mech needs somewhere to walk, and the owner wants cities (owner brief, 2026-10-23: "model
buildings, cities, people"). Before hand-built city blocks exist (`blender-csg-levels`), a seeded
generator gives the engine a real-scale world to stride through, to measure against, and to play
the same way twice.

## What changes

- A world generator in `cb-core` that turns a seed and `data/world/world.ron` into a `WorldPlan`:
  a terrain heightfield, several cities with street grids, city blocks split into lots with
  buildings from 2 to 60 storeys, highways joining the cities, and open rolling ground between
  them. Pure and deterministic: the same seed and data give the same plan, pinned by a golden-seed
  test.
- One ground query (`height_at`) used by the terrain mesh, the pilot's feet and the mech's root,
  and one collision query (`push_out_of_buildings`) used by both the pilot and the mech.
- `cb-app`'s `WorldPlugin` draws the plan: terrain chunks, road surfaces and placeholder box
  buildings, sun and fog. `--seed <n>` picks the world.

## Out of scope

Real building models (later, from `blender-csg-levels`, placed by this plan's lots), streaming,
traffic, people, destruction, and a physics engine (survey `T2`).
