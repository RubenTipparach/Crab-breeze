# Design: small-buildings

## 1. Types (sizes from the mockup)

| Type | Footprint m | Height m | Storeys | Walls | Roof | Biomes |
| --- | --- | --- | --- | --- | --- | --- |
| Ranch house | 17 x 18 with drive | 8.3 (antenna) | 1 | brick front, siding garage | hip 0.33 | temperate, savanna |
| Colonial | 13 x 17 | 10 | 2 | clapboard, shutters | gable 0.62, columned portico | temperate |
| Craftsman bungalow | 11 x 16 | 7 | 1 | clapboard | gable to street, porch on brick piers | temperate, coast, montane forest |
| Split-level | 16 x 18 | 7.4 | 1 and 2 | brick and siding | two gables | temperate, montane forest |
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

The kits are exported now: `assets/models/small_buildings/`, made by
`tools/buildings/export_small_buildings.cjs` from the same kit the mockup draws. How to load and
break them is in `docs/design/small-buildings-kit.md`. The game can load these glbs first and
port the kit to `cb-core` later; the plan below stays the target.


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
- **Support.** A piece stands while something holds it up: a piece below whose top meets its
  bottom, or a neighbour beside it sharing its height, at most three pieces sideways from one that
  stands on something. Bounding boxes merely touching do not count: that let a single wall stub
  hold a whole roof in the air (owner, 2026-10-24: "A lot of stuff stuck floating"). The rest creak
  and trickle dust for 0.25-0.85 s, then fall.
- **Collapse.** A building that loses half its ground-floor wall area pancakes: everything above the
  floor slab falls, in chunks about 2 m across, under a dust cloud.
- **What breaks into what.**
  - Thin pieces (plaster faces, glass, signs, under 6 cm) turn to dust.
  - Lone boxes break into chunks about 1.5 m across, keeping their material and paint.
  - Roof planes are cut into panels about 1.6 m across (owner, 2026-10-24: "I wish roofs were
    more segmented instead of giant triangle"). Each panel is held by the wall under its eave or
    the panel below it, or by neighbours beside it, so a shell holes a roof (one shell on the
    bungalow's ridge takes 26 of its 76 panels) and a lost wall peels that side off. A panel falls
    as one tilted chunk of its roofing. Flat roofs, ceilings and upper floors are tiles about
    2.5 m across. This adds about 10,000 triangles to the street.
  - Other pieces (a window with its glass, a sofa, a car) fall whole.
- **Cost** (owner: "Doesn't seem very performant, is there too much detail in debris"):
  - Chunks are drawn instanced, one draw a material (a unit box scaled and tinted per chunk), and
    a chunk at rest simply stops moving, so rubble costs nothing more.
  - Whole pieces bake into one static mesh a material when they rest, rebuilt for that material
    only and at most twice a second.
  - Flying debris casts shadows only through the instanced draws.
  - At most 1,500 things fall at once, and none falls for more than 6 s. Physics for 500 falling
    things costs about 0.02 ms a frame headless.
- **Piles.** Debris lands on the ground or on rubble already there (a 0.5 m height grid). A pile
  steeper than 35 degrees sheds what lands on it, so walls falling straight down spread into heaps
  instead of standing back up as columns. A pile rises by the thickness of what lands, not by its
  tilted bounding box.
- **Measured headless:**
  - the shots: a shell on the ranch house; one demolition charge on the motel; two each on the
    corner store and the colonial (the colonial pancakes); a cannon round on the diner and a
    missile on the Quonset;
  - 468 things fall and rest within 20 s, as 422 chunks;
  - no chunk rests above 6 m, and the highest pile, the colonial's, is 4.5-5.5 m;
  - every standing piece is held.

In the game:
- A house is light, wood framed, so one mech kick or a shell caves it in; brick and block take more.
- Interiors spill out with their furniture.

## 7. The first build in the game (owner, 2026-10-26)

The owner's asks, in order: "we should implement houses/small businesses pack, the one house I dont like is the one
with the single big triangle roof, all others look good"; "we may need a few variations of the subrabn house though,
also make sure I can walk into the interiors"; "maybe we need to load in lowpoly furniture at 100m or less"; "right
now some furniture is just blocks", "I'd like real furniture for when the player is on foot", "real furniture should
not be used when in mech tho".

**What the player sees.** Suburbs of houses round every city, small towns strung along the highways with a main
street of businesses, and a gas station or diner every few kilometres of highway. Walk up to a house and its front
door swings open as you reach it; inside are the rooms, the stairs and real furniture. From the cockpit the same
houses stand with their windows lit and their rooms behind the glass, furnished with simple blocks, and a stride
through one breaks it apart.

### 7.1 Variants of the suburban houses

The four suburban types (ranch, colonial, bungalow, split-level) get two more variants each, so a suburb has twelve
houses to draw from instead of four. A variant changes the plan, not only the paint:

| Type | `a` (the original) | `b` | `c` |
| --- | --- | --- | --- |
| Ranch | brick front, hip roof, attached garage | painted siding, side-gable roof, carport | L-plan: a gabled bedroom wing stepped forward |
| Colonial | white clapboard, five bays, portico | red brick, three bays, a one-storey sunroom wing | painted clapboard, full-width porch, attached garage |
| Bungalow | gable to the street, porch on brick piers | hip roof, porch on plain posts | side gable, brick, porch across the front |
| Split-level | brick living wing, siding bedroom wing | hip roofs, painted siding throughout | board and batten, gables to the street, chimney |

Each is the same kit (`docs/mockups/small_buildings_kit.js`, `build(id, seed, M, variant)`), exported as its own glb
(`ranch.glb`, `ranch_b.glb`, `ranch_c.glb`...). The seed still varies the paint, the doors, the roof and the car.
Twelve suburban glbs instead of four: about 3 MB more on disk.

### 7.2 Walking in

- **Doors open.** A door's slab, glazing and knob are one *leaf*, exported on its own with its hinge. The game hangs
  it as a `StreetDoor` (`cb_core::world::door`), the city's door code: it swings away from a pilot on foot who comes
  within 1.5 m and shuts behind them (survey W30). Front and back doors and the glazed door of each shop front are
  leaves; garage doors stay shut.
- **Colliders.** On foot, every house within `collision_radius_m` (60 m) gets a `Walkable`
  (`cb_core::world::interior`), the city's rule: a box no higher than `step_m` (0.45 m) above the feet is stood on,
  a taller one starting below `head_m` (1.8 m) is a wall. The boxes are the kit's pieces (their bounds), in the
  house's own frame, so a house turned to any heading is met exactly: the body is moved into the house's frame,
  pushed, and moved back. Stairs are boxes too, so the colonial's and the split-level's upper floors are reached on
  foot. In the mech there are no house colliders: the mech breaks what it walks into (7.6).
- One `Walkable` gains a list of doors (`doors`) where it had one, so the city and the houses share it.

### 7.3 Furniture by distance and by who is looking

| Pilot | Within 60 m | 60-100 m | Past 100 m |
| --- | --- | --- | --- |
| On foot | real furniture (modelled, `assets/models/furniture/`) | the kit's blocks | none |
| In the mech | the kit's blocks | the kit's blocks | none |

- The kit's block furniture moves out of LOD0 into its own node, `FURN`, drawn within 100 m (it was LOD0's, to
  150 m).
- Real furniture is a Blender generator, `tools/blender/build_furniture.py`: 17 items (sofa, bed, dining table with
  chairs, kitchen run, TV, bookcase, desk, store aisle, cooler, shop counter, diner booth, stool, washer, salon chair,
  workbench, car lift, tyres), each modelled to fill the block it replaces, so it is placed with the block's
  transform. 400-4,000 triangles an item. The exporter lists every block item's placement (item, position, quarter
  turns, tint) in the pieces table.
- In the mech nothing loads the real furniture (owner: "real furniture should not be used when in mech").
- **One material, one atlas** (owner, 2026-10-26: "make sure to texture furniture too! use texture atlas if you have
  to ... I'd prefer a single material for all furniture if possible, they should all be 1 draw call"). Every
  modelled piece shares one 4096 px PBR atlas (`assets/textures/furniture/`: albedo, normal, ORM), baked in Blender
  from its high-detail shaders, and the blocks are drawn on the same atlas's flat swatches of each surface. A
  building's furniture is merged into one mesh in that one material, so every building's furniture batches into one
  multi-draw.
- **Part of the building's cells** (owner: "furniture are part of the building cells anyways, so that should save
  on perf when updating the state of the furnitures mesh"). Each furniture vertex carries its piece's slot in the
  building state buffer (`building-gpu-state`), as the city's rooms do: a broken piece is hidden by a word written to
  the GPU, never by rebuilding a mesh. A damaged house's LOD0 is drawn the same way.
- **Cost**, a suburb within 60 m of a pilot on foot: about 15 houses x 6 items x 2,500 triangles = 225,000
  triangles in one material (one multi-draw); the atlas is 3 maps x 4096 px, about 200 MB with mips uncompressed
  (BC7 would be a quarter). The blocks: 100-300 triangles a house, same material.

### 7.4 Where they stand

Placement is part of world generation (`cb_core::world::small_buildings`), from `data/world/small_buildings.ron`,
deterministic from the seed:
- **Suburbs**: round every city, in a band 40-260 m out from its square, in loose rows that follow the city's edge
  and bend with the ground (survey I11: "more organic, less square"), each house turned to face the city, with a
  little jitter in heading and spacing. Suburban types only, chosen by biome.
- **Towns**: 3-8 on the highways, each a main street of 20-60 buildings: businesses (diner, gas station, corner
  store, garage, strip mall, motel, drive-in) on the highway in the middle, houses behind and towards the ends, all
  facing the road.
- **Roadside stops**: every 3-4 km of highway outside towns and cities, one to three businesses facing the road.
- **Biome**: each type's biomes from the manifest (temperate, savanna, desert, beach, montane forest, jungle,
  swamp, coast). A lot whose biome no type suits stays empty.
- **Lots** keep clear of water, the coast, highways (beyond their flattened shoulder), air bases, city squares and
  each other. Each lot is flattened to one height by a pad, as a city's square is: flat for 2 m round the footprint,
  blending to the ground over 10 m (`Flats`), so a house never floats or sinks. A lot steeper than 3 m across its
  footprint is skipped.
- Trees, grass and rocks keep clear of lots as they do of buildings.
- **Cost**: about 1,500 lots on the island; placement and pads add about 50 ms to world generation (measured on the
  owner's machine, not here).

### 7.5 Drawing

- One mesh handle per (type, LOD, surface), shared by every house of that type, and one material per surface key
  for the whole island, so Bevy batches every house of a type. Stand-in colours until the Material Maker sets land
  (task 1.1).
- LOD1 and LOD2 are spawned for every lot at load with `VisibilityRange` (LOD1 150-600 m, LOD2 600 m-2 km, 10 %
  cross-fade). LOD0, its door leaves and its furniture are spawned for lots within 170 m of the camera and freed past
  200 m, at most 8 lots a frame.
- **Cost**: 1,500 lots x (12 + 3) primitives is about 22,000 entities, nearly all culled by range; a town at LOD0
  draws about 20 x 20 primitives before batching. Measured on the owner's machine before it is called done.

### 7.6 Breaking

- The kit's rules (section 6), ported to `cb-core`: a piece stands while it is reached from the ground through `up`
  links and at most three `side` links; a house collapses when its alive wall area falls below half.
- Shots, blasts and the mech's body break every piece within their radius, through the same calls that break the
  city's buildings (`weapons`, `blast`, the mech's step). A striding mech breaks the pieces its legs and body
  overlap (a house is about a tenth of its height).
- A broken piece becomes chunks for `debris-physics` (`Chunk`), boxes of about 1.5 m, panels whole, thin pieces as
  dust. A damaged house keeps its own copy of its LOD0 meshes with the broken pieces' triangles removed, and stays
  at LOD0.

### 7.7 Checks

- `the_same_seed_places_the_same_lots`, `lots_keep_clear_of_water_roads_and_each_other`, `every_lot_is_flat`.
- `a_pilot_walks_through_the_front_door_into_the_living_room`, `a_shut_door_is_a_wall`,
  `the_colonials_stairs_reach_the_upper_floor`.
- `a_piece_with_nothing_under_it_falls`, `a_house_losing_half_its_walls_collapses`.
- `every_kit_loads_and_its_furniture_is_real` (each block item has a real model within 5 cm of its box).
- Headless stills in a cloud session: a suburb from the cockpit, a town's main street, walking into a house, a
  house broken by the mech.

## Open questions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| S1 | The types | all sixteen (the A-frame dropped, owner 2026-10-26); ranch, diner, gas station and motel first (they read best from a cockpit) | proposed (survey S1) |
| S2 | How dense are towns between the cities | 3-8 small towns of 20-60 buildings, a roadside stop every 3-4 km of highway | proposed (survey S2) |
| S3 | Do interiors come with every building near you | yes, within 150 m or when opened by damage, as `building-interiors` does for the city | proposed (survey S3) |
| S4 | Which houses stand in montane forest | the bungalow and the split-level (the A-frame was its house) | provisional (survey S4) |
| S5 | How near real furniture is drawn for a pilot on foot | within 60 m, the blocks from 60 to 100 m; in the mech only blocks, within 100 m | provisional (survey S5) |
| S6 | Do suburbs get their own streets | not in the first build: houses face the city with their drives; streets and cul-de-sacs as their own change | provisional (survey S6) |
