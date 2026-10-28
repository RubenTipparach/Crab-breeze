# Proposal: road-surfaces

## Why

Owner, 2026-10-26: "outsource some good CC0 road textures". Every road in the game wears a stand-in
generated at start (`textures.rs`, `asphalt`, 4 m a tile): from the street it reads as dark noise
with black crack squiggles, and it has no normal or roughness map, so the sun never catches it.
The pavements wear a generated slab pattern of the same kind.

## What changes

- Three CC0 PBR sets from ambientCG (colour, OpenGL normal, packed occlusion-roughness-metal), checked
  seamless by tiling 2 x 2, packed by `tools/textures/pack_cc0_sets.py` into `assets/textures/road/`
  and loaded with full mips by `pbr_set` (the CLAUDE.md section 13 CC0 exception, as the grass and the
  building surfaces):
  - `asphalt`, Asphalt023L: clean fine asphalt for city streets.
  - `asphalt_worn`, Asphalt022: weathered, cracked asphalt for arterials, highways, and the air bases'
    runways and taxiways.
  - `paving`, PavingStones133: square concrete slabs for the pavements and kerbs.
- Tile sizes and a large-scale tint by world position live in data (`road_surfaces:` in
  `data/world/world.ron`), so the repeat does not show from the cockpit.
- The generated `asphalt` and `concrete` stand-ins are removed: nothing else uses them.
- A stills instrument, `CB_STILLS_ROADS=1 --stills <dir>`, shoots the roads on foot and from the
  cockpit's eye height, before and after.

## Out of scope

Lane markings (kept as they are: flat painted quads over the asphalt), road geometry, crossings with
zebra stripes, road decals, wet-road reflections, and parks (still drawn on the block's mesh with the
pavement set under their green tint, as before).
