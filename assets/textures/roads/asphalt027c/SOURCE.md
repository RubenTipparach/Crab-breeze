# Asphalt027C (road asphalt), from ambientCG

- Source: https://ambientcg.com/view?id=Asphalt027C (2K-JPG set), downloaded 2026-10-27.
- Licence: CC0 1.0 Universal (public domain); see https://docs.ambientcg.com/license/. No attribution required;
  noted here for provenance (CLAUDE.md section 6.5).
- Asked for by the owner, 2026-10-27: "can you use material make or some.cc0 road textures with normal maps and stuff
  to make the road look nicer", then, from the Road Surface Bench page, "27c for downtown". Drawn on the cities' streets and the highways.
- Seamless: checked by tiling 2 x 2. One tile covers 4 m, the roads' UVs.
- What was changed, by ImageMagick:
  - `albedo.jpg`: the colour map as it came.
  - `normal_gl.png`: the OpenGL (Y up) normal map as it came, saved losslessly.
  - `orm.png`: packed the glTF way, R ambient occlusion, G roughness, B metal (0), from the set's AmbientOcclusion
    and Roughness maps.
