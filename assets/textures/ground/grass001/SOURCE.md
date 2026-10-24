# Grass001 (ground grass), from ambientCG

- Source: https://ambientcg.com/view?id=Grass001 (2K-JPG set), downloaded 2026-10-24.
- Licence: CC0 1.0 Universal (public domain); see https://docs.ambientcg.com/license/. No
  attribution required; noted here for provenance (CLAUDE.md section 6.5).
- Asked for by the owner, 2026-10-24: "we need better grass textures, find some online thats free
  and cc0", "and fully tileable". Seamless: checked by tiling 2 x 2.
- What was changed, by ffmpeg:
  - `albedo.jpg`: the colour map scaled per channel (r x3.0, g x2.23, b x5.06, sRGB) so it averages
    near white: the ground's vertex tint (grass, sand, rock) sets the hue, the map adds the detail.
  - `normal_gl.png`: the OpenGL (Y up) normal map as it came.
  - `orm.png`: packed the glTF way, R ambient occlusion, G roughness, B metal (0).
