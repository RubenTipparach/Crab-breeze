# Particle depth: soft edges, stable order, and an A/B bench

## Why

Owner, 2026-10-25, on stills of smoke over the island: "the transpatrency isnt working very well, lots
of zfighting on distant smoke still", "we still really need to ab test all these particles, the softer
ones are alright, but still suffer from zbuffer issues", "and depth tests".

Every particle in the game is a `bevy_hanabi` billboard (rev 77630ff). Two things in how hanabi draws
them show as the faults the owner sees:

1. **Hard cuts where a sprite meets the world.** A blended particle is depth tested against the scene
   and writes no depth (`render/mod.rs`, `depth_stencil_3d`). A smoke sprite 6 m across that sinks into
   the ground or a wall is cut along a straight line where it crosses it. From altitude that reads as a
   flat-bottomed blob; from the side, as a seam. Hanabi has no soft-particle fade.
2. **No order inside an effect.** One effect is one draw call and its particles draw in buffer order, not
   back to front. Where two blended sprites overlap, whichever was spawned later covers the other, even
   when it is behind it. As the view or the spawn order changes, the overlap flips and the smoke
   shimmers: the "z-fighting" on distant smoke. Effects are sorted against each other by Bevy (the
   `sorted` list in `fx.ron` exists for this), but not inside themselves.

The clouds do not have either fault. `cloud.wesl` fades each billboard where it meets the depth prepass
(`soft_edge`), and `clouds_render` sorts every billboard back to front on the CPU before drawing.

## What changes

- **An A/B bench first** (a measurement instrument, section 4): a capture mode that sets each smoke, dust,
  steam and trail recipe against the ground, a building face and open sky, near (15 m), mid (120 m) and
  far (450 m), seen from the seat, and renders the same frames under each variant into one sheet.
- **Variant B, soft particles**: hanabi's render shader reads the depth prepass and fades a particle's
  alpha over `soft_m` metres before the surface behind it, as the clouds do. This needs a patched
  hanabi (vendored, provenance kept).
- **Variant C, sorted smoke**: the big blended kinds (Smoke, Trail, Dust, Steam, Mist) drawn by a
  CPU-sorted billboard renderer on the clouds' pattern, with the smoke sheet and soft edges; hanabi keeps
  the small and additive kinds (sparks, fire, flame, dirt, debris, rubble, spray).
- The owner picks from the sheets. The pick is built; the others are deleted.

## What does not change

Recipes, sheets and colours. What each effect looks like is a separate question from whether its edges
and order are right.
