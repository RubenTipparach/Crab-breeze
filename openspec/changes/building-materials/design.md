# Design: building-materials

## 1. The four skins

| Skin | Set | Tile | What it is | Mean roughness |
|---|---|---|---|---|
| Brick | `building/skin/brick` (graph `house_cityBrick`) | 1.8 m | 215 x 65 mm bricks, 10 mm joints, running bond, 8 bricks by 24 courses a tile, near white | 0.89 |
| Concrete | `building/skin/concrete` (graph `house_boardFormed`) | 2.4 m | 16 form boards of 150 mm, a fin at each board joint, tie cones of 25 mm on 600 mm, bug holes, rust runs below ties | 0.87 |
| Roof | `houses/roofing` (existing) | 1.0 m | Gravel on tar, built-up flat roof | 0.93 |
| Wood | `houses/door` (existing) | 1.0 m | Painted V-groove boards, 125 mm | 0.49 |

Every tile repeats on a whole brick, board or tie, so the 2 x 2 tiling on the contact sheet shows no seam. Texel
density at 1024 px: brick 569 px/m, concrete 427 px/m, roof and wood 1024 px/m; all above the 256 px/m the
`material-maker` skill starts buildings at.

**Near white.** The game multiplies a skin texel by its piece's colour (`dressing.rs` `BRICKS`, `CONCRETES`, the roof
tone, the door colours), so the brick and concrete albedos are light grey (mean about 0.8 sRGB) and differ in value,
not hue. The house brick, red in its own right, keeps its red: the two are one graph function, `brick(title, across,
neutral)`, so the bond, arrises, pits and mortar are written once.

**Up the wall is up the image** on the city buildings (`world.rs` `MeshBuilder::uv`: v = -height / tile), unlike the
houses; the rust runs below the ties run toward larger image y.

## 2. Wiring

`building_arrays::Source::Graph(dir)` reads `albedo.png`, `normal.png` (OpenGL convention, as the houses read it) and
`orm.png` (R occlusion, G roughness, B metal) from `assets/<dir>`. The skin layers keep their indices
(`Layer::index`), so nothing else in the arrays moves. `buildings::tile_m` is the set's `tile_m` in
`materials.ron`; the test `every_skin_wears_a_material_maker_set_at_its_listed_tile` reads the real file and the real
maps.

The far facades sample the near colour layers at `skin_layer`'s tile, so they change with the skins and need no code.
Their roughness and occlusion stay their own (`far_facades.rs`), fine at 250 m and past.

## 3. Cost

- **Video memory: unchanged.** The four skin layers were already 1024 px layers of the three arrays (the generated
  maps were resized to `building_arrays::SIZE`): 4 layers x 3 maps x 1024 x 1024 x 4 bytes x 4/3 (mips) = 67 MB, as
  before. The eight generated source images (512 and 256 px, about 3 MB) are no longer made.
- **Disk:** two new sets of three 1024 px PNGs, about 5 MB.
- **Startup:** twelve PNG decodes in place of four colour and four normal maps generated per pixel in Rust; not
  measured (cloud sessions measure nothing, CLAUDE.md section 7).
- **Draw calls and triangles:** unchanged.

## 4. How it is checked

- The contact sheet `docs/screenshots/house_materials/sheet.png`, each set tiled 2 x 2, lit flat and on a sphere,
  looked at.
- The test above; `cargo test --workspace`.
- Cloud stills from the overlook and in a street, beside the 2026-10-26 ones (`docs/screenshots/building-materials/`).
- A playtest on the owner's machine: walk a street of walk-ups and offices in daylight and at night.

## Open questions

| # | Question | Provisional answer |
|---|---|---|
| BM1 | Mortar takes the building's brick tint (one multiply per texel), so red brick has pinkish mortar. Keep, or carry a second tint for mortar? | Keep: one tint, as before; the joints read darker by their occlusion and normal |
| BM2 | Should the far facades take their roughness and occlusion from the sets too, not only the colour? | No: at 250 m and past the difference is under a pixel |
