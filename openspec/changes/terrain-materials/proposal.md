# Proposal: terrain-materials

## Why

Owner, 2026-10-24: "Someone needs to make pbr terrain textures." The ground is vertex colours.
High detail (CLAUDE.md section 10) needs PBR ground: grass, dirt, rock, mud by the rivers,
scorch where explosions hit, asphalt and concrete in the cities.

## What changes

- Art (cloud session, Material Maker, `material-maker` skill): tiling PBR sets (albedo, normal,
  ORM, height) for grass, dry grass, dirt, rock, mud, gravel, asphalt, concrete and scorch, 2048 px
  over about 2 m, in `assets/textures/terrain/`, listed in `data/materials/terrain.ron`.
- Engine: a terrain material (`ExtendedMaterial` on `StandardMaterial`) that splats up to 8 layers
  from per-vertex weights computed by rules in data: slope, height, distance to water, wetness,
  city ground, and a crater or scorch channel written by `terrain-deformation`. Triplanar on
  slopes over 35 degrees so cliffs do not stretch. Height-blended transitions.
- Roads and pavements take the asphalt and concrete sets.

## Out of scope

Grass blades and ground clutter (part of `vegetation`), virtual texturing, terrain decals.
