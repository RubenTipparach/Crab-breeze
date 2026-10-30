# Proposal: building-materials

## Why

The city buildings' brick, concrete, roof and timber come from a hand-written Rust image generator
(`crates/cb-app/src/textures.rs`, `building_textures`): a colour map and a normal map each, no occlusion, one roughness
for the whole surface, and a brick tile of 9.3 bricks that does not repeat on a whole brick. CLAUDE.md section 10 says
textures are Material Maker graphs and never a procedural generator that imitates one, and that every custom texture is
a full PBR set (colour, normal, ORM). The owner asked for it (2026-10-30, after "You need material maker. Did you get
that skill": "Yup. Also make a pr").

What the player sees: up close, brick walls with real joints that catch the light and darken in their shadow, bricks
that differ one from the next, chipped arrises; office walls of board-formed concrete with the form boards' grain, the
tie holes and rust running from them; gravel roofs; painted timber doors. From far off the same, because the far
facades are rendered from these maps at startup (`building-type-lods`).

## What changes

- Two new Material Maker graphs in `tools/materials/build_house_ptex.py`, the houses' graph builder (one helper, one
  pipeline): `cityBrick`, the house brick's own bond made near white so each building's brick colour tints it (the
  house brick graph is parameterised, not copied, and its output is unchanged byte for byte), and `boardFormed`,
  board-formed concrete. Rendered headless with `export_house_materials.sh` and written by `postprocess.py --houses`
  to `assets/textures/building/skin/{brick,concrete}/`, listed in `data/materials/materials.ron`.
- The roof and the timber wear the houses' existing sets (`houses/roofing`, gravel; `houses/door`, painted boards),
  which are already near white or neutral.
- `building_arrays::Source::Graph` reads a Material Maker set (`albedo.png`, `normal.png`, `orm.png`);
  `buildings::layer_sources` points the four skin layers at the sets, and `tile_m` lays each at its set's tile.
- `textures.rs` loses `BuildingTextures`, `Pbr`, `pbr` and `building_textures`.
- The far facades need no change: they read the skin colour layers and tiles through `buildings::skin_layer`.

Not changed: the facade panel and the other stand-ins in `textures.rs` (grass, metal, foliage), the CC0 surface sets
of the interiors, the glass.
