# Asphalt023L (clean asphalt: city streets), from ambientCG

- Source: https://ambientcg.com/view?id=Asphalt023L (1K-JPG set, https://ambientcg.com/get?file=Asphalt023L_1K-JPG.zip),
  downloaded 2026-10-26.
- Licence: CC0 1.0 Universal (public domain); see https://docs.ambientcg.com/license/. No
  attribution required; noted here for provenance (CLAUDE.md section 6.5).
- Asked for by the owner, 2026-10-26: "outsource some good CC0 road textures" (openspec change
  `road-surfaces`). Seamless: checked by tiling 2 x 2 and at full resolution across the seam.
- One tile is 2.5 m of real asphalt, drawn at that size (`road_surfaces.street_tile_m`).
- Made by `tools/textures/pack_cc0_sets.py`:
  - `albedo.jpg`: the colour map scaled per channel so it averages 0.8 (sRGB): the road's vertex
    colour sets the tone, the map adds the detail.
  - `normal_gl.png`: the OpenGL (Y up) normal map as it came.
  - `orm.png`: packed the glTF way, R ambient occlusion, G roughness, B metalness (black: the set
    has none).
