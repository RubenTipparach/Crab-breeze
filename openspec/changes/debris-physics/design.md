# Design: debris-physics

Avian runs in `FixedPostUpdate` at the sim rate. Colliders: terrain chunks as heightfields (65 x 65
samples), crater tiles as trimeshes, ground plane fallback. Chunks: cuboids from the building
cells, density 600 kg/m3 (concrete with voids), friction 0.8, restitution 0.05. Wrecks: a cuboid
around the cut bones. Felled trees: a capsule. Sleeping on; 600 bodies at most.

Cost: Avian's own timing per step, measured in `perf-bench` with a collapse in view.

## As built (2026-10-24)

- Ground colliders are triangle meshes built from the drawn chunk and crater meshes (not heightfields), so the physics ground is exactly the drawn ground.
- Debris bodies collide with the ground and each other; standing buildings are not colliders yet. Felled trees keep the deterministic rod model from `vegetation`.

## Physics near the player only (2026-10-24)

Owner, 2026-10-24: "If the mech is too far away there should be no physics sim right? Just the
cell sim and some particles". Until now every chunk a hit broke off became an Avian body wherever
it was, so raiders shelling the far side of a city filled the 600-body budget with debris nobody
could see.

- A chunk breaking off within `physics_radius_m` (400 m) of the pilot's eye becomes a dynamic body
  as before. Farther, no body: it plays the `member_fail` puff (dust and small rubble particles)
  where it broke, at most one puff per building per frame. The cell simulation (`cb-core`) runs
  everywhere as before: the building still breaks, falls, lands with its `collapse_dust` and
  leaves its rubble heap, which are not physics.
- A moving body that ends up beyond `physics_radius_m x 1.2` (480 m, so a body on the edge does not
  flicker) is made static where it is, checked every 0.5 s.
- The budget constants move to data (`world.ron`, `debris`): `physics_radius_m`, `max_dynamic`
  600, `max_kept` 2500 (CLAUDE.md section 6.4).
- Cost: one distance test per new chunk, one per moving body every 0.5 s. Saving: no Avian bodies,
  broad-phase or solver work for debris out of sight; measured with the city combat benchmark.
