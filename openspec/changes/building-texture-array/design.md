# Design: building-texture-array

## 1. The arrays

All layers 1024 x 1024 (the CC0 sets are; the generated skin maps are resized), full mip chains,
sRGB colour, linear normal and ORM. A layer without a map gets a flat one (normal (0.5, 0.5, 1),
ORM (1, roughness, metal)), so every look reads the same three arrays. Built at startup from the
same files and generated images the separate materials use today (`pbr_set`, `textures`).

## 2. Per vertex

UV0 stays the planar, metre-based UV, scaled per layer by its tile size at mesh build (so one
material serves every tile size). UV1.x is the layer. The colour stays the tint, its alpha the
element slot (`building-gpu-state`).

## 3. The material

`ExtendedMaterial<StandardMaterial, BuildingArrays>`: the base keeps Bevy's flags, fog and
lighting settings; the extension adds the three arrays and the element state buffer. Its fragment
shader samples the arrays at (uv, layer), builds `PbrInput` (base colour times tint, the normal
map through the vertex tangent, occlusion, roughness, metal) and calls Bevy's
`apply_pbr_lighting` and `main_pass_post_lighting_processing`, as the grass shader does. The
vertex shaders are `building-gpu-state`'s.

## 4. Cost

Draws per damaged building: about 10 per pass to 2 (opaque, glass); with the prepass and four
shadow cascades about 60 to 12. Texture memory about the same as today (the same maps, packed).
Measured: interleaved city combat benchmark against `building-gpu-state`, and a still of the same
demolition for the look.

## Open questions

| question | recommendation | status |
| --- | --- | --- |
| Merge glass in too (one draw) | no: transparent needs its own pass | provisional |
