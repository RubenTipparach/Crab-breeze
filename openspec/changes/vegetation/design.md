# Design: vegetation

## 1. Scatter (cb-core::world::vegetation)

Poisson-disc on a 512 m tile grid, seeded per tile (`trees/<tx>/<tz>`), so a tile's trees do not
depend on any other tile. Density in data per biome mask: forest 1 tree per 60 m2, scattered 1 per
800 m2, riverside rows at 8 m. Species in data (`pine`, `oak`, `birch`) with height range, trunk
radius and break impulse. Estimate for seed 42: about 120 k trees on open ground.

## 2. Drawing

Instanced meshes per species and LOD, culled by a 64 m cell grid: under 80 m full model (about 3 k
triangles), 80-300 m LOD (about 400), past 300 m one camera-facing card; past 1.5 km none. Shadows
from LOD0 and LOD1 only.

## 3. Breaking

A standing tree is not a physics body (cheap): only the trees within 30 m of a mech or an
explosion are checked, each fixed step, against the mech's leg and body capsules. An impulse over
the tree's `break_impulse_n_s` (pine 20 m: about 150 kN s) converts it into an Avian dynamic body
(a capsule for the trunk, mass from volume and 600 kg/m3), hinged to its stump by a joint that
breaks at once, given the hit impulse. It sleeps when it comes to rest and stays as a fallen tree
(kinematic, visible, walkable over). At most 64 falling trees are simulated at once; beyond that
the oldest settle where they are.

## 4. Cost (expected)

| Item | Count |
| --- | --- |
| Visible trees in a forest view | about 8 k: 300 LOD0, 2 k LOD1, the rest cards |
| Draws | about 10 instanced batches plus shadows |
| Physics | up to 64 bodies while falling, then sleeping |
| Memory | 120 k x 32 bytes instance data = about 4 MB |

## Open questions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| W6 | Trees in the first world | conifer and broadleaf woods between cities, river rows, park trees | provisional (survey W6) |

## As built (2026-10-24)

- Trees are procedural meshes (cone pines, blob oaks and birches) until the art side's models arrive; one entity each, culled at 1.6 km. Seed 42 grows 41 571.
- A broken tree falls as a rigid rod pivoting on its stump (`theta'' = 3 g sin(theta) / 2 L`) in `cb-core`, deterministic, instead of an Avian body.
- Tuning is the `trees:` section of `data/world/world.ron`.
