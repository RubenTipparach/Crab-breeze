# Design: debris-physics

Avian runs in `FixedPostUpdate` at the sim rate. Colliders: terrain chunks as heightfields (65 x 65
samples), crater tiles as trimeshes, ground plane fallback. Chunks: cuboids from the building
cells, density 600 kg/m3 (concrete with voids), friction 0.8, restitution 0.05. Wrecks: a cuboid
around the cut bones. Felled trees: a capsule. Sleeping on; 600 bodies at most.

Cost: Avian's own timing per step, measured in `perf-bench` with a collapse in view.

## As built (2026-10-24)

- Ground colliders are triangle meshes built from the drawn chunk and crater meshes (not heightfields), so the physics ground is exactly the drawn ground.
- Debris bodies collide with the ground and each other; standing buildings are not colliders yet. Felled trees keep the deterministic rod model from `vegetation`.
