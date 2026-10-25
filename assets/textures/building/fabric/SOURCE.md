# Fabric061 (fabric: seats, sofas and bedding), from ambientCG

- Source: https://ambientcg.com/view?id=Fabric061 (1K-JPG set), downloaded 2026-10-24.
- Licence: CC0 1.0 Universal (public domain); see https://docs.ambientcg.com/license/. No
  attribution required; noted here for provenance (CLAUDE.md section 6.5).
- Asked for by the owner, 2026-10-24: "The parts on the buildings need textures. All furniture
  needs textures." (openspec change `building-surfaces`). Seamless: checked by tiling 2 x 2.
- Made by `tools/textures/pack_cc0_sets.py`:
  - `albedo.jpg`: the colour map scaled per channel so it averages 0.8 (sRGB): the vertex colour
    sets the hue, the map adds the detail.
  - `normal_gl.png`: the OpenGL (Y up) normal map as it came.
  - `orm.png`: packed the glTF way, R ambient occlusion (white where the set has none), G
    roughness, B metalness (black where the set has none).
