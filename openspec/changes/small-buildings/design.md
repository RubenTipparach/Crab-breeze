# Design: small-buildings

## 1. Types (sizes from the mockup)

| Type | Footprint m | Height m | Storeys | Walls | Roof | Biomes |
| --- | --- | --- | --- | --- | --- | --- |
| Ranch house | 17 x 18 with drive | 8.3 (antenna) | 1 | brick front, siding garage | hip 0.33 | temperate, savanna |
| Colonial | 13 x 17 | 10 | 2 | clapboard, shutters | gable 0.62, columned portico | temperate |
| Craftsman bungalow | 11 x 16 | 7 | 1 | clapboard | gable to street, porch on brick piers | temperate, coast |
| Split-level | 16 x 18 | 7.4 | 1 and 2 | brick and siding | two gables | temperate |
| A-frame cabin | 9 x 13 | 8.3 | 1 and loft | board and batten, glass gable | steel A to the deck | montane forest |
| Pueblo Revival | 16 x 15 | 4.9 | 1 | adobe stucco 0.5 m thick, vigas | flat, stepped parapet | desert |
| Stilt house | 11 x 12 | 9.6 | 1 on 2.4 m piles | plank | steep corrugated hip, veranda | jungle, swamp |
| Beach cottage | 12 x 14 | 6.8 | 1 on piers | pastel clapboard | metal hip, screened porch | beach |
| Single-wide trailer | 21 x 10 | 3.4 | 1 | ribbed steel, stripe | flat steel | desert, savanna |
| Streamline diner | 24 x 15 | 6.6 | 1 | stainless, enamel stripes | flat, rounded ends, neon | roadside |
| Gas station | 24 x 27 | 8.9 | 1 | block kiosk | flat canopy on 4 columns | roadside |
| Corner store | 11 x 15 | 8.5 | 2 | brick, cornice | flat, parapet | towns |
| Auto repair | 15 x 21 | 6.1 | 1 | painted block | flat | towns, roadside |
| Motor court motel | 37 x 20 | 9.3 | 2 | block | tile hip over the walkway | roadside, beach |
| Strip mall | 36 x 30 | 5.9 | 1 | stucco | flat, shingled mansard canopy | towns |
| Burger drive-in | 20 x 17 | 9.3 | 1 | tile panels | swept shed | roadside |
| Quonset workshop | 11 x 28 | 5.6 | 1 | corrugated arc, block ends | barrel | savanna, coast |

## 2. The kit

One implementation of each part (CLAUDE.md section 6.1), used by every type:

**`wall(axis, line, span, thickness, height band, exterior side, openings)`**
- It cuts the openings as real holes: solid boxes left and right of each opening, below it and
  above it.
- It then fills each hole by its kind:
  - window: frame, glazing bars, sill, lintel, shutters, air conditioner;
  - picture window and ribbon;
  - door: slab, glazed lite, knob, stoop;
  - shop front: aluminium frame, kick plate, transom, glazed door;
  - garage door (four panels) and roll-up door with its housing.
- Glass sits mid-frame and is transparent, with the furnished room behind it.
- An interior plaster face runs 12 mm inside the wall, so the inside reads as a room and no two
  faces share a plane (CLAUDE.md, no z-fighting).

**`row(span, specs)`** spaces a storey's openings evenly.

**`house(...)`** is the generic dwelling, and most house types are a set of options to it:
- foundation, storeys, four walls with their openings;
- floors with a finish, partitions with door gaps, furniture by room, ceiling lights;
- roof, chimney, porch, gutters, antenna.

**Roofs:**
- `hipRoof`, `gableRoof` (ridge along x or z, with gable-end walls inside and out), `flatRoof`
  (parapet, coping, rooftop unit), and the shed, swept and barrel roofs drawn per type;
- every sloped roof has an underside, so the eaves read from below.

**Interiors:**
- Furniture comes from one table (`FURN`): sofa, bed, table and chairs, kitchen run, TV, desk,
  store aisle, cooler, counter, booth, stool, washer, salon chair, workbench, lift, tyres, pump.
- Cars: an 80s sedan and a pickup.

Signs are text on a face (`sign`), emissive at night for neon.

Each type's geometry is gathered per material, with UVs in world metres (textures tile at true
scale) and a vertex colour tint (paint colour, brick shade). So one material serves every building
of its kind, and a town draws in about 45 materials however many buildings stand in it.

## 3. Materials

Twenty-five surface sets:
- brick, concrete block, clapboard, board and batten, stucco, adobe;
- asphalt shingle, terracotta tile, standing-seam steel, corrugated steel, ribbed steel;
- concrete, roofing gravel, asphalt, painted trim, louvred shutter, panel door;
- brushed stainless, laminate, timber, decking, oak floor, knotty pine, checker tile, wall tile,
  carpet, fabric.

Each set is a Material Maker graph (CLAUDE.md section 10), with full PBR maps at 2048 px. The
mockup draws canvas stand-ins with normal maps derived from the same height. Glass is transparent
with reflections; lamps and neon are emissive.

**Texture memory (expected):** about 25 sets x 3 maps x 5.6 MB (BC7 with mips), about 420 MB.
Sets shared with `terrain-materials` (concrete, asphalt, gravel) and the city (brick, concrete) are
counted once. 1024 px for interior-only sets saves about 120 MB.

## 4. Level of detail (the owner's rule: no generated detail far away)

| LOD | Range | What is drawn | Triangles (mockup measures) |
| --- | --- | --- | --- |
| 0 | within 150 m of a body, or opened by damage | everything: frames, sills, interiors, furniture, cars | 1,000-12,000 a building (whole street of 17: 70,000) |
| 1 | 150-600 m | shell with openings as flat glass, roof, porch, sign; no interior | about 15 % of LOD 0 |
| 2 | 600 m-2 km | a box and roof with a baked facade texture, instanced | 12-40 |
| 3 | beyond | merged into the town's impostor tile | n/a |

A town of 150 buildings within 2 km costs roughly:
- 20 at LOD 0 (about 100,000 triangles);
- 60 at LOD 1 (about 50,000);
- the rest at LOD 2.

That is about 45 + 45 + 2 draws, measured on the owner's machine before it is called done.

## 5. Generation in the game

- Types and their options are data: `data/world/small_buildings.ron`, with sizes in metres, opening
  rows, materials, roof and plan, and weights by biome and by zone (suburb, roadside, village).
- `cb-core` builds a building's parts as plain data (boxes and polygons, materials and tints) from
  the seed, with no engine in it, as `world::dressing` does today. That also gives the damage
  model its pieces.
- `cb-app` turns the parts into meshes per material.
- Placement is the island session's: lots along the island's roads and round its cities, from
  `island-biomes`' biome weights.
- Same seed, same town (golden-seed test).

## 6. Destruction

What the mockup does (it is the model for the game; the game's version goes through
`building-destruction` and `debris-physics`, one implementation):

- **Pieces.** The kit records what breaks off together:
  - each wall segment between openings and each interior face is one piece;
  - each opening's frame, sill and glass together are one piece, as are each roof plane, each
    piece of furniture and each car.
  - A ranch house has about 160 pieces, the motel about 640, the street 3,300.
- **A shot** blows out every piece within its radius:
  - autocannon 1.3 m, 120 mm shell 3.2 m, missile 5.5 m, demolition 9 m;
  - near pieces are thrown at up to 9-22 m/s by weapon;
  - lone boxes over 1.6 m break into chunks about 1.2 m across, keeping their material and paint;
  - a fireball, a flash light, sparks, dust and a scorch on the ground go with it.
- **Support.** Pieces touch pieces (bounds within 4 cm). Any piece no longer joined to the ground
  through touching pieces creaks and trickles dust for 0.25-0.85 s, then falls.
- **Collapse.** A building that loses half its ground-floor wall area pancakes: everything above the
  floor slab falls, in chunks about 2 m across, under a dust cloud.
- **Rubble.**
  - Debris tumbles with gravity, bounce and friction on the ground or on rubble already there (a
    0.5 m height grid, so piles build).
  - After 0.25 s at rest each piece is baked into one static rubble mesh a material, so a flattened
    street draws in the same 45 draws as a standing one.
  - At most 1,200 pieces fall at once.
  - Measured headless: one demolition charge on the motel, two on the corner store (which
    pancakes), and a shell, a cannon round and a missile elsewhere throw 695 pieces. They all rest
    within 15 s, leaving 12,400 rubble triangles.

In the game:
- A house is light, wood framed, so one mech kick or a shell caves it in; brick and block take more.
- Interiors spill out with their furniture.

## Open questions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| S1 | The seventeen types | all seventeen; ranch, diner, gas station and motel first (they read best from a cockpit) | proposed (survey S1) |
| S2 | How dense are towns between the cities | 3-8 small towns of 20-60 buildings, a roadside stop every 3-4 km of highway | proposed (survey S2) |
| S3 | Do interiors come with every building near you | yes, within 150 m or when opened by damage, as `building-interiors` does for the city | proposed (survey S3) |
