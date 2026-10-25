# Proposal: rubble

## Why

Owner, 2026-10-24: "rubble". A building brought down should leave a pile the pilot can climb on
foot and a mech can stride over or take cover behind. Today fallen chunks are up to 600 cuboids that
nothing can stand on, and the oldest freeze in the air of the budget.

## What changes

- **The heap is ground.** The fines of each landed island, half its volume, are added to
  `terrain-deformation`'s 0.5 m height tiles. That is the same layer craters dig, so `height_at`
  returns ground plus rubble and the pilot and the mech walk on it with no new collision code. A
  deposit spreads as a Gaussian over the island's landing footprint, then slumps to its angle of
  repose (37 degrees).
- **Pieces on top**: shattered pieces settle on the heap and on standing floors, and freeze when at
  rest for 0.3 s. Past the 9 000-piece pool, the oldest resting piece is removed (its volume is
  already in the heap). Glass shards go when they settle.
- **Dressing**: the heap's surface gets the rubble layer of `terrain-materials` (crushed concrete,
  brick, dust) by depth, and instanced rubble meshes from the kit (slab chunks with rebar, brick
  clumps, pipes, furniture remnants). They are scattered on it by the tile's seed.
- **Dust**: a cloud on every collapse, sized by the island (6-30 m), and a puff on each broken
  member.
- **Deterministic**: the core decides how much rubble lands where, from its kinematic island fall.
  Where Avian puts loose pieces is presentation, so two machines grow the same heap.

## Presented by

`docs/mockups/buildings.html`: shots `warehouse_drop` and `volley`. The rubble heap counter is in
the Counts panel.

## Out of scope

Digging through rubble, rubble that slides on a slope after it settles, fire and smoke columns.
