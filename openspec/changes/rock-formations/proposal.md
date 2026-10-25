# Proposal: rock-formations

## Why

Owner, 2026-10-24: "rock formations", "cliffs". Terrain alone cannot make arches, overhangs, stacks
or hoodoos: they need meshes.

## What changes

- A Blender kit (`tools/blender/build_rock_kit.py`, on `hs_kit`): sea stacks, arches, hoodoos and
  spires, boulders and boulder fields, cliff overhang pieces, canyon ledges. Each is in 3-5
  variants with LOD0-2 and full PBR from Material Maker rock graphs (granite, red sandstone strata,
  limestone).
- Placed deterministically per tile like the trees, by biome:
  - stacks and arches off the cliff coast;
  - hoodoos and spires in canyons;
  - boulders on the range and beaches.
  They are grown near the bodies and drawn instanced per kit piece.
- Colliders for the mechs and the pilot (capsules and boxes per piece); debris bounces off them.
