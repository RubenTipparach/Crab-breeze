# Furniture atlas: how it was made

`albedo.png` (sRGB), `normal.png` (tangent space, OpenGL convention, +Y up) and `orm.png` (R occlusion,
G roughness, B metal, packed the glTF way), 4096 x 4096 px, are the one texture set of the one material
`furniture` that every item in `assets/models/furniture/` uses, so all furniture draws in one call.

Generated, not painted and not downloaded. `tools/blender/build_furniture.py` writes them:

1. Each item is modelled (see `docs/design/furniture.md`), unwrapped by a box projection in metres and its
   islands shelf-packed into its own square region of the atlas. Regions share one texel density (house
   furniture at the planned density, large shop and garage fixtures at a fraction of it, packaged goods and
   books at 0.3 of their item's); the measured densities are in `assets/models/furniture.json`.
2. Each part carries its colour and finish (metallic, a printed pattern) as vertex attributes, and each
   surface key has one procedural Cycles shader (fabric weave, wood grain, painted and chrome metal, brushed
   stainless, laminate speckle, rubber grit, glass smudge, packaging print, pegboard holes).
3. Cycles bakes each item's region on the CPU: diffuse colour, the tangent-space normal of the shaders'
   bump, roughness, metal (as an emission pass) and ambient occlusion (48 samples, 0.4 m reach) into the R
   channel of the ORM map. Surfaces the game tints with the placement's colour are baked in a light neutral
   (`TINT_ALBEDO`) and marked by COLOR_0 alpha = 1.
4. The bottom strip holds one 256 x 256 px swatch per kit surface key (fabric, wood, metal, counter, trim,
   paint, glassDark, glassShop, lamp, product, rubber, stainless, concrete), each a 1 m patch baked light
   and neutral so a block's colour multiplies into it, then made tileable by blending it with itself shifted
   by half. Their UV rectangles are under `swatches` in `assets/models/furniture.json`.

The PNGs are written by the generator's own encoder (fixed zlib level); two full builds give byte-identical
files (Blender 5.1.2 `bpy` module, CPU, four threads). Rebuild: `blender --background --factory-startup --python tools/blender/build_furniture.py`.
