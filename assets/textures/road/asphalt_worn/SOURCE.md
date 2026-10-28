# Asphalt022 (worn asphalt: arterials, highways, runways and taxiways), from ambientCG

- Source: https://ambientcg.com/view?id=Asphalt022 (1K-JPG set, https://ambientcg.com/get?file=Asphalt022_1K-JPG.zip),
  downloaded 2026-10-26.
- Licence: CC0 1.0 Universal (public domain); see https://docs.ambientcg.com/license/. No
  attribution required; noted here for provenance (CLAUDE.md section 6.5).
- Asked for by the owner, 2026-10-26: "outsource some good CC0 road textures" (openspec change
  `road-surfaces`). Seamless: checked by tiling 2 x 2 and at full resolution across the seam.
- One tile is 1.85 m of real asphalt, drawn at that size (`road_surfaces.worn_tile_m`).
- Made by `tools/textures/pack_cc0_sets.py`:
  - `albedo.jpg`: the colour map scaled per channel so it averages 0.8 (sRGB): the road's vertex
    colour sets the tone, the map adds the detail.
  - 2026-10-26, after the first look in the game ("a flat, uniform navy blue"): contrast x1.6 about its mean; roughness moved to a mean of 0.86 with its variation x2.0; normal map x and y x1.5, renormalised.
  - `normal_gl.png`: the OpenGL (Y up) normal map as it came.
  - `orm.png`: packed the glTF way, R ambient occlusion, G roughness, B metalness (black: the set
    has none).
