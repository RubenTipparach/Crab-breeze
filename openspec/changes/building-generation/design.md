# Design: building-generation

All numbers are the defaults the mockup uses and would be the starting values in
`data/world/buildings.ron`. Code holds no tuning (section 6.4).

## 1. The grammar

| Step | Input | Output | Rule |
| --- | --- | --- | --- |
| Mass | lot (from `procgen-world`), archetype | grid `nx x nz` cells of 6 m, storey count | `n = floor((lot - 2 x setback) / 6 m)`; storeys from the archetype range, capped by the lot's `max_storeys` |
| Storeys | mass | `base[k]` floor levels, footprint per storey (a cell rectangle) | ground storey `ground_m`, then `storey_m`; a podium keeps the full footprint, the tower above steps in one bay all round |
| Columns | footprints | one column per grid node per storey, 0.6 m square | perimeter columns inset `col_inset_m` (behind a curtain wall) or stand proud as 0.75 m brick piers |
| Floors | footprints | one 0.25 m floor bay per cell per storey; a roof membrane on cells with no storey above | |
| Facade | each perimeter cell edge per storey | one wall panel of `skin` | street sides of a retail ground storey get a storefront |
| Roof | top cells of each footprint | parapets with coping (and a cornice on walk-ups), a bulkhead over the core, props | props by rejection sampling, 40 tries each, clear of the roof edge by 1.0 m and of each other by their radii, all cells under a big prop on one roof level |

Archetypes in the mockup:

| Archetype | Storeys | Ground / upper (m) | Skin | Cladding | Roof props |
| --- | --- | --- | --- | --- | --- |
| Brick walk-up | 4-6 | 4.2 / 3.3 | punched: two 1.1 x 1.75 m sash windows per bay, stone sills and lintels | brick | timber water tank, chimneys, TV aerials, vents, hatch |
| Brutalist office | 7-10 | 4.5 / 3.6 | ribbon: 1.0 m spandrel standing 0.18 m proud, glass band, concrete fins 0.8 m deep on column lines | board-formed concrete | HVAC, dish, mast, vents, ducts |
| Curtain-wall tower | 15-19 | 5.0 / 3.6 | curtain: mullions every 1.5 m, 0.75 m spandrel over each slab edge; 3-storey podium | glass, dark metal | water tank, HVAC, masts, dish |
| Brick warehouse | 3-4 | 5.5 / 4.5 | industrial: one steel-sash window per bay (7 x 5 lights), soldier-course header, roll-up loading doors | brick | skylights, tank, HVAC, vents |

## 2. Rules that keep it clean

- **Everything is on the 6 m grid.** Facade bays are 3 m (half a structural bay), so a partition
  (`building-interiors`) always meets a mullion or pier, never a window.
- **No z-fighting** (section 10): walls overlap the slab edge by 2 cm below and 3 cm above, never
  meeting it in a plane. Trims (sills, lintels, copings) stand 2 cm off the faces they sit on. The
  mockup was built to this, and the game checks it with the zfight check on the generated meshes
  (section 3 of tasks).
- **Determinism**: one SplitMix64 stream per building, forked by label. The same seed and data give
  the same building, piece for piece (golden-seed test).

## 3. Modules and textures

- The **kit** (`tools/blender/build_building_kit.py`, on `hs_kit` booleans) writes one glb per
  module family: window bays per skin, storefronts, doors, cornice and coping runs, fire escape bays,
  roof props.
  - Each module has LOD0, LOD1 and LOD2, nodes named for the element role they carry (`WALL_`,
    `GLASS_`, `TRIM_`), and `FRAC_<n>` children: its pre-fractured cells, cut at build time.
  - A manifest lists triangles per LOD, and the build refuses one over budget (section 7).
- **Bulk materials** are Material Maker graphs: brick (stretcher bond, 215 x 65 mm with 10 mm joints),
  board-formed concrete (150 mm boards, tie holes on 600 mm), stucco, roofing felt, gravel and
  asphalt.
  - Full PBR sets at 2048 px over 2.4 m (850 px/m), with a height map for
    `ParallaxMappingMethod::Relief`.
- **Rendered bays**: each facade module is also rendered face-on in Cycles (Star Crew's
  `build_wall_panels.py` method, PBR passes instead of a lit albedo).
  - Output: albedo, normal, ORM, height and emission (lit windows at night) at 256 px/m.
  - These maps texture LOD1 (flat facade quads with relief) and the LOD2 facade atlas.
  - Trims go into one trim-row atlas, as Star Crew stacks its seven rows.
- **Decals** for signage, stains and grime (`ForwardDecal` everywhere; `ClusteredDecal` where the
  platform has it, not macOS).

## 4. Drawing and LOD

| LOD | Range (from the camera) | What | Draws |
| --- | --- | --- | --- |
| 0 | under 250 m, or damaged | every module as geometry, one mesh per material per building, vertex attribute `element` | 7 per building |
| 1 | 250-900 m | each facade a set of flat quads per storey band with the rendered bay maps and relief parallax; roof props instanced | 3 per building |
| 2 | over 900 m | the box with the facade atlas and roof cell render | batched, under 20 for the city |

A damaged building is always LOD0, so destruction never shows a baked facade.

## 5. Cost

From the mockup (box geometry, a browser; not a game measurement):

| Item | Value |
| --- | --- |
| Generation, 7 buildings and 56 storeys with interiors | 63-75 ms in JavaScript |
| Triangles, 7 buildings with interiors | 527 k |
| Draw calls, at rest, with shadows | 189 |

Expected in the game, measured in `perf-bench` before this is called done:

- Generation: about 4 ms per building in Rust on the owner's machine, off the main thread.
- LOD0 with the real kit: about 3 x the mockup's triangles, about 225 k per building. 25 LOD0
  buildings in a downtown cockpit view is about 5.6 M triangles, inside an RTX 3070's means. Measured
  against the 8.3 ms p95.
- Texture memory: 6 bulk materials x 4 maps x 2048 px (BC7) is about 100 MB; rendered bay atlases
  about 60 MB; trim atlas 16 MB.

## Open questions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| G1 | Setting | 1980s alt-history industrial city | decided (survey G1) |
| W10 | Building styles | walk-ups, warehouses, brutalist offices, early glass towers | provisional (survey W10) |
| W14 | Facade texture source | Material Maker for bulk materials, Star Crew face-on renders for bays, trims and far facades (needs a section 13 exception) | provisional (survey W14) |
