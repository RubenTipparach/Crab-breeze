# Proposal: terrain-materials

## Why

Owner, 2026-10-24: "Someone needs to make pbr terrain textures." Then, with the new island
(`island-biomes`): "a bunch of interesting pbr materials for the landscape, I want things to look
realistic and varied as we fly around this island doing missions". The ground is vertex colours,
with one grass texture on main (ambientCG Grass001). Ten biomes need about twenty ground materials,
and none may look like a repeated tile from 2 km up or from the pilot's boots.

## What changes

- **Art** (cloud session, Material Maker, the `material-maker` skill): twenty tiling PBR sets
  (albedo with height in alpha, normal, ORM), 2048 px over 2-4 m, in `assets/textures/terrain/`,
  listed in `data/materials/terrain.ron`. Section 1 of the design lists them.
- **Splatting by biome:** each ground vertex carries its biome weights (`island-biomes`). The
  fragment shader takes the two strongest biomes, and each biome picks its flat, slope and cliff
  layers by the pixel's slope. So at most four layers are sampled, height-blended.
- **Triplanar** on slopes over 35 degrees. **Strata:** sandstone and canyon walls are banded by
  world height through a 1D strata strip, so the layers line up across a whole canyon.
- **Macro variation** at 40 m and 400 m scales, and a far colour map at 8 m a texel, so the island
  reads as varied from altitude and no tile repeats.
- **Scorch** from `terrain-deformation` on top. Roads and pavements keep the asphalt and concrete
  sets.

## Out of scope

Grass blades and ground clutter (`vegetation`), virtual texturing, decals, rock meshes
(`rock-formations`).
