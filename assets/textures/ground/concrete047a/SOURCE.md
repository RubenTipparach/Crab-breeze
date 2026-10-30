# Concrete047A (weathered outdoor concrete), from ambientCG

- Source: https://ambientcg.com/view?id=Concrete047A (2K-JPG set), downloaded 2026-10-30.
- Licence: CC0 1.0 Universal (public domain); see https://docs.ambientcg.com/license/. No attribution required;
  noted here for provenance (CLAUDE.md section 6.5).
- Asked for by the owner, 2026-10-30: "also dock platform should use concrete textures", "and not textured". Drawn
  on the seaport's quays and their crown walls (`data/world/world.ron`, `sites.ground_textures`), tinted to the
  site page's own colour for each (`pbr_set::set_material_tinted`), so the set gives the grain and the page the
  colour.
- Seamless: checked by tiling 2 x 2. One tile covers 4 m (`sites.ground_texture_m`).
- What was changed, with Python (Pillow):
  - `albedo.jpg`: the colour map as it came.
  - `normal_gl.png`: the OpenGL (Y up) normal map, saved losslessly.
  - `orm.png`: packed the glTF way, R ambient occlusion, G roughness, B metal (0), from the set's AmbientOcclusion
    and Roughness maps.
