# House and small-business surface textures

One full PBR set per surface generator of the small-buildings kit (`SURFACES` in
`docs/mockups/small_buildings_kit.js`), so every wall, roof, floor and fitting of the houses and small businesses
(`assets/models/small_buildings/*.glb`) has a real texture. Each folder holds:

- `albedo.png`: base colour, sRGB. The game multiplies it by the building's vertex colour (its paint), so painted
  surfaces are light and near neutral (mean about 0.8 sRGB) and natural ones (brick, asphalt, wood...) carry their own
  colour at the Main Street mockup's levels.
- `normal.png`: tangent space, OpenGL convention (+Y toward the image top), drawn from true-depth height.
- `orm.png`: R occlusion, G roughness, B metal, packed the glTF way.
- `SOURCE.md`: which graph and renderer made it.

All maps tile seamlessly; one tile covers `tile_m` metres (the kit's UVs are metres and the game scales them by
1 / `tile_m`). **Up the surface is down the image**: the kit's v grows up a wall or roof slope and the glb keeps it, so
laps, shingle courses and louvres have their thick butt at the top of each course in the image.

| Set | Tile | Surface keys | What it is |
|---|---|---|---|
| brick | 0.9 m | brick | Face brick 215 x 65 mm, 10 mm joints, running bond (4 x 12 per tile) |
| block | 0.8 m | block | Concrete block 390 x 190 mm, running bond |
| siding | 1.0 m | siding | Painted clapboard, 167 mm exposure, 12 mm butts, butt joints |
| board | 1.0 m | board | Board and batten, 333 mm boards, 64 mm battens |
| stucco | 2.0 m | stucco | Knockdown sand-float stucco |
| adobe | 2.5 m | adobe | Mud plaster over 0.5 x 0.125 m adobe courses, straw flecks |
| shingle | 1.0 m | shingle | Three-tab asphalt shingles, 125 mm courses, 333 mm tabs |
| tile | 1.0 m | tile | Clay barrel tiles, 250 mm wide, 333 mm courses |
| metalRoof | 1.0 m | metalRoof | Standing seam, 500 mm panels, 25 mm seams |
| corrugated | 1.0 m | corrugated, corrugatedArc | Galvanised sheet, 13 corrugations of 77 mm, 18 mm deep, ridges along u |
| ribbed | 1.0 m | ribbed | Ribbed aluminium siding, 167 mm ribs |
| skirt | 1.0 m | skirt | Vinyl skirting, 125 mm panels |
| concrete | 2.0 m | concrete | Cast concrete, bug holes, hairline crack |
| roofing | 1.0 m | roofing | Gravel-surfaced built-up flat roof |
| asphalt | 2.0 m | asphalt | Asphalt paving, worn aggregate, fine cracks |
| smooth | 1.0 m | trim, paint, soffit, plaster, metal, product, rubber, paintCar | Smooth paint, faint stipple (512 px) |
| shutter | 0.5 m | shutter | Louvres, 12 of 42 mm |
| door | 1.0 m | door | Painted V-groove boards, 125 mm |
| brushed | 1.0 m | stainless | Brushed stainless steel, fully metal |
| laminate | 1.0 m | counter | Flecked plastic laminate (512 px) |
| wood | 1.0 m | wood | Bare timber, grain along u |
| planks | 1.0 m | deck, floorWood | Planks 143 mm with 5 mm gaps along v, butt joints, nails |
| pine | 1.0 m | woodInt | Knotty pine panelling, 167 mm boards along v |
| floorTile | 0.9 m | floorTile | Black and white checker, 450 mm tiles, 3 mm grout |
| wallTile | 1.0 m | tileWall, tileRed | Glazed tiles 111 mm, 2 mm grout |
| carpet | 1.0 m | carpet | Cut pile |
| fabric | 0.5 m | fabric | Plain weave, 3.9 mm threads |

Every set is 1024 x 1024 except `smooth` and `laminate` (512 x 512). Texture memory: 25 sets x 3 maps x 1024 px
RGBA8 with mips is about 25 x 3 x 5.3 MB = 400 MB uncompressed, plus 8 MB for the two 512 px sets (a block-compressed format would cut it to a quarter).

Where a set serves several keys whose own roughness or metal differ (`smooth` serves `paint` at 0.4 and `plaster` at
0.9, `metal` at metal 0.7), `data/materials/materials.ron` gives each set's mean `roughness` and `metallic`, so the game
can scale the map by the key's value over the set's.

## Rebuild

The graphs are written by `tools/materials/build_house_ptex.py` (to `tools/materials/ptex/house_<generator>.ptex`) and
rendered with Material Maker headless (`material-maker-headless` skill):

```sh
eval "$(bash .claude/skills/material-maker-headless/scripts/setup.sh | grep ^export)"
tools/materials/export_house_materials.sh            # every set, about 15 min on lavapipe
tools/materials/export_house_materials.sh brick tile # just these graphs, then every set re-written
```

Then look at `docs/screenshots/house_materials/sheet.png`: each set's albedo tiled 2 x 2, the same lit flat (world
up at the top, light from above) and a lit sphere.
