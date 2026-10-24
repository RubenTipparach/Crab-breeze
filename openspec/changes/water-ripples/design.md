# Design: water-ripples

## 1. The wave field (`cb-core::world::waves`, pure)

`h` and `h_prev` on an N x N grid of `cell_m`, centred on a point that moves in whole cells with
the pilot (the field shifts by whole cells, so nothing smears). Each fixed step (1/60 s):
`h_next = (2h - h_prev + (c dt / dx)^2 * laplacian(h)) * (1 - damping * dt)`, with c = 8 m/s,
dx = 1 m (c dt / dx = 0.13, stable below 0.7) and damping 0.6 per second; the outer 8 cells damp to
zero. A splash of energy E MJ lowers `h` by `0.8 * E^(1/3)` m in a disc of `2 * E^(1/3)` m. A mech
landing counts its mass times its vertical speed squared, halved, as energy. Cost: 65 k cells a
step, about 0.15 ms on one core, to be measured before and after.

Tests: `a_splash_spreads_as_a_ring`, `the_waves_die_away`,
`the_field_follows_the_pilot_by_whole_cells`, `the_same_splashes_give_the_same_waves`.

## 2. Drawing

The field uploads as an R16F texture (128 KB) each frame. The water near the pilot is a 256 x 256
vertex patch (130 k triangles) with an `ExtendedMaterial` on the water's `StandardMaterial`: the
vertex shader adds `h` to y and builds the normal from neighbouring texels; the far water keeps its
flat sheet with a hole cut to the patch. The ripple normal map stays on top.

## 3. The UV fix (built)

All water surfaces sample the ripple map in world space (one tile every 24 m) and each river scrolls
along its whole course's direction.

## Open questions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| W16 | How big a patch of live water | 256 m round the pilot at 1 m cells; farther splashes show flat rings | provisional (survey W16) |
