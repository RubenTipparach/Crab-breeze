# Furniture

Real furniture for the houses and small businesses a pilot walks through on foot. The small buildings kit
(`docs/mockups/small_buildings_kit.js`, `FURN`) dresses rooms with furniture made of boxes; each item here is
a modelled, textured 1980s version of one of those block items (the setting: alternate-history 1980s American
suburbs, diners, corner stores, garages, a laundromat, a salon). Each fills the space of its block, so the game
swaps a block item for the real one with the same transform.

Status: generated and checked by reading the glbs back and by renders (`docs/screenshots/furniture/`). Not yet
wired into the game.

## The items

| item | what it is | box lo (x, y, z) m | box hi (x, y, z) m | tinted | triangles |
|---|---|---|---|---|---|
| aisle | Store gondola: three shelves a side of boxes, cans, bottles and bags | -2.00, 0.00, -0.45 | 2.00, 1.60, 0.45 | none | 5,884 |
| bed | Double bed: panelled headboard with turned posts, footboard, bedspread, turned-down sheet, two pillows | -0.80, 0.00, -1.06 | 0.80, 1.05, 1.00 | fabric | 3,488 |
| booth | Diner booth: channel-tufted benches, chrome-banded table on a pedestal, condiments | -0.90, 0.00, -1.05 | 0.90, 1.10, 1.05 | fabric | 5,718 |
| chair | Salon hydraulic chair: chrome base and pump, channelled back, arms, footrest | -0.40, 0.00, -0.32 | 0.40, 1.20, 0.55 | fabric | 4,488 |
| cooler | Three-door glass drinks cooler with lit header, shelves of bottles, cartons and cans | -1.50, 0.00, -0.40 | 1.50, 2.10, 0.41 | none | 5,510 |
| counter | Shop counter, panelled front, open shelves behind, chrome-edged top, cash register | -1.55, 0.00, -0.40 | 1.55, 1.35, 0.40 | trim | 2,574 |
| desk | Pedestal desk, beige home computer with monitor and keyboard, office chair | -0.75, 0.00, -0.40 | 0.75, 1.15, 0.95 | none | 4,516 |
| kitchen | Kitchen run: raised-panel cabinets, laminate top, double steel sink and faucet, almond fridge | -1.82, 0.00, -0.32 | 1.82, 2.15, 0.33 | trim | 5,732 |
| lift | Two-post car lift: channel posts, carriages, telescoping arms with pads, overhead beam, power unit | -1.60, 0.00, -1.20 | 1.60, 3.20, 1.20 | none | 3,244 |
| shelf | Bookcase with crown and plinth, five shelves of books, a vase and a photo | -0.90, 0.00, -0.18 | 0.90, 1.90, 0.18 | none | 3,408 |
| sofa | Sofa: skirted base, rolled arms, two seat and two back cushions, wooden feet | -1.00, 0.00, -0.45 | 1.00, 0.85, 0.45 | fabric | 4,520 |
| stool | Diner counter stool: chrome pedestal and foot ring, piped vinyl seat | -0.20, 0.00, -0.20 | 0.20, 0.78, 0.20 | fabric | 1,442 |
| table | Dining table with turned legs and a spindle-back chair at each end | -0.60, 0.00, -0.88 | 0.60, 0.95, 0.88 | none | 2,676 |
| tires | Stack of four tyres with tread grooves | -0.35, 0.00, -0.35 | 0.35, 0.86, 0.35 | none | 4,576 |
| tv | TV stand with a VCR, tapes and a console, and a CRT set with dials | -0.55, 0.00, -0.25 | 0.55, 1.08, 0.25 | none | 2,476 |
| washer | Laundromat front-loader: coin slide, dial, lamps, chrome porthole door | -0.34, 0.00, -0.34 | 0.34, 0.90, 0.34 | none | 2,070 |
| workbench | Garage bench: angle-iron frame, plank top, pegboard with tools, cabinet, vise, toolbox | -1.20, 0.00, -0.35 | 1.20, 1.90, 0.35 | none | 3,420 |

The triangle counts are read back off the glbs by the build; the limit is 6,000 per item and the build
refuses one over it.

## Conventions the game relies on

- **Frame.** Metres, glTF axes: y up, the item's front toward +z, origin on the floor at the block item's local
  origin. The model stays inside its block's box (above) within 5 cm; the build refuses one that does not.
  "Front" is the side a person uses: the sofa's seat, the TV's screen, the washer's door, the cooler's doors,
  the customer side of the shop counter (the register's keys face the clerk at -z, its display faces +z). The
  bed's headboard is at -z; the desk's chair is at +z facing the desk; the table and the booth have a seat at
  each end.
- **One draw call.** Each glb is one node `MODEL` with one mesh of one primitive in the one material
  `furniture`. Every item and every block swatch shares the atlas `assets/textures/furniture/` (`albedo.png`
  sRGB, `normal.png` tangent space OpenGL +Y up, `orm.png` R occlusion G roughness B metal), 4,096 px square.
  The glbs carry no images; the game binds the atlas to `furniture`.
- **Tint.** COLOR_0 is VEC4 with rgb 1 and alpha the tint mask: 1 on the surface the placement's colour tints
  (the sofa's, bed's, booth's, stool's and salon chair's upholstery, the kitchen's and shop counter's cabinet
  paint), 0 elsewhere. Tinted surfaces are baked in a light neutral (`TINT_ALBEDO`, #E4E2DE) so the game draws
  `albedo * mix(1, tint, alpha)`.
- **UVs.** TEXCOORD_0 lies inside the item's `atlas_rect` in `assets/models/furniture.json` (`[u0, v0, u1, v1]`,
  glTF UV, v down from the top). NORMAL is split on hard edges only (cushions are smooth, cabinets crisp);
  TANGENT is exported for the normal map.
- **Swatches for the block furniture.** The atlas's bottom strip holds a 256 px swatch, one metre of surface,
  for each kit key: fabric, wood, metal, counter, trim, paint, glassDark, glassShop, lamp, product, rubber,
  stainless, concrete. Each is light and neutral (the block's colour multiplies in) and tiles. Their rects are
  under `swatches` in the manifest, so the low-poly block furniture can draw with the same material.

## Texel density

One 4,096 atlas holds all 17 items. House furniture (sofa, bed, table, booth, chairs, stool, TV, desk, washer,
kitchen) is packed at 175 px/m (5.7 mm a texel); the large shop and garage fixtures (aisle, cooler, lift,
bookcase, bench, counter) at 0.55 to 0.85 of that (96 to 149 px/m), and packaged goods and books at 0.3 of their
item's. The measured density of each item is `atlas.texel_density_px_per_m` in the manifest. The brief's
400 to 512 px/m does not fit: the 17 items have several hundred square metres of surface (the aisle alone is
96 m², mostly goods), and 16.7 million texels at 450 px/m cover 83 m². Reaching it would need more atlas pages
(still one material if they are layers of an array texture) or tiling detail maps per surface.

## Rebuilding

    blender --background --factory-startup --python tools/blender/build_furniture.py [-- --sheet]
    python tools/blender/build_furniture.py [--sheet] [--only sofa,bed] [--no-build] [--quick]

Blender 4.4 or later (the binary, or the `bpy` module: `pip install bpy`). The build models every item, unwraps
and packs it, bakes the atlas in Cycles on the CPU (about 25 minutes on four cores), exports the glbs, reads
each back and writes `assets/models/furniture.json`. `--sheet` renders `docs/screenshots/furniture/`: one still
per item and `sheet.png`, every item at true relative scale beside a 1.8 m figure, drawn with the atlas and
example tints. `--only` limits the stills; `--quick` bakes a quarter-size atlas and exports nothing, to try a
shader change.

UVs are a box projection in metres (each face along the axis it faces most), islands are the connected faces
facing the same way, shelf-packed into the item's square; no Smart UV Project, whose packing differs run to
run. bmesh's triangulation can return faces in a different order each run, so the merged mesh is put in an
order set by positions alone. With that, two full builds give byte-identical glbs, PNGs and manifest.

The method is `blender-hard-surface`'s: rounded boxes, lathes, swept tubes and extruded profiles; openings cut
with Exact booleans (the sink bowls, the TV's screen recess and shelf bays, the monitor's bezel, the cooler's
open front and door frames, the washer's door well, the lift's post channels, the counter's back shelves);
chamfers where light catches; coplanar faces dissolved, floor-contact faces removed, triangulated.

The look is procedural Cycles shaders, one per surface key (fabric weave, wood rings, painted and chrome metal,
brushed stainless, laminate speckle, rubber grit, glass smudge, printed packaging, pegboard holes), baked per
item: colour and metal through the emission pass, roughness, the shaders' bump as a tangent-space normal, and
48-sample occlusion. `assets/textures/furniture/SOURCE.md` has the detail.
