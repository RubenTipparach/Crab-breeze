# Proposal: water-ripples

## Why

Owner, 2026-10-24: "I also want like 3d rippling when explosions or mechs land in the water". Today
water is a flat sheet with a scrolling normal map and flat rings drawn on top; a 6 MJ shell in a
lake or a 50 t mech dropping from its rockets leaves the surface flat. In the same messages: the
ripple pattern "looks kinda random" (fixed here first, task 0.1).

## What changes

- **A height field on the water near the pilot**: a 256 x 256 grid, 1 m a cell, following the
  pilot's mech, stepped as a damped wave equation each fixed step. Explosions in water, shells,
  footfalls and a mech landing push it down by their energy; the waves spread at about 8 m/s and
  die away, and the edge absorbs them.
- **Drawn as real geometry**: the water inside the grid is displaced by the height field in the
  vertex shader with normals from it, so a ring of waves rises, catches the light and passes under
  the mech. Beyond the grid the water stays as it is.
- **The flat rings stay** for the far field.
- **The ripple texture in world space** (built): every water surface samples the ripple map at one
  tile every 24 m in world space, and each river scrolls along its course, so the pattern runs on
  across bends and where rivers meet lakes instead of turning with each strip.

## Out of scope

Buoyancy and waves moving bodies, foam, refraction, shorelines bending waves.
