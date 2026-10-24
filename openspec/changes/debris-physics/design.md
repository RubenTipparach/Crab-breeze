# Design: debris-physics

Avian runs in `FixedPostUpdate` at the sim rate. Colliders: terrain chunks as heightfields (65 x 65
samples), crater tiles as trimeshes, ground plane fallback. Chunks: cuboids from the building
cells, density 600 kg/m3 (concrete with voids), friction 0.8, restitution 0.05. Wrecks: a cuboid
around the cut bones. Felled trees: a capsule. Sleeping on; 600 bodies at most.

Cost: Avian's own timing per step, measured in `perf-bench` with a collapse in view.
