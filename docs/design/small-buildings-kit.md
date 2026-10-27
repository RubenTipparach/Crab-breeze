# Small buildings kit: how to load, break and optimise it

This is for the agent who brings the houses and small businesses into the game. The design is in
`openspec/changes/small-buildings`, the look is on the Main Street mockup
(https://claude.ai/artifact/NsAB4HQMjoPd8Nazq9vTcW), and this page says how to use the exported
kits. Nothing here is game code yet: the kits are assets with data, the rules below are what the
mockup does, and the game ports them (CLAUDE.md sections 4 and 6.1).

## 1. What is in `assets/models/small_buildings/`

| File | What it holds |
| --- | --- |
| `<id>.glb` | One building: three nodes, `LOD0`, `LOD1`, `LOD2`, each with one mesh of one primitive per surface |
| `<id>.pieces.json` | The destruction table for `LOD0`: every breakable piece, its triangles, bounds, what it is made of and what holds it up |
| `manifest.json` | Every type: name, kind, biomes, bounds, triangle counts and budgets per LOD, surfaces and sign texts |

There are 16 types and 8 variants, exported with seed 7 (the A-frame cabin was dropped, owner 2026-10-26). Triangle counts come from `manifest.json`.

| Type | id | LOD0 (with interior) | LOD1 | LOD2 | Pieces |
| --- | --- | --- | --- | --- | --- |
| Ranch house | `ranch` | 4,918 | 3,140 | 28 | 399 |
| Colonial | `colonial` | 6,301 | 4,164 | 24 | 339 |
| Craftsman bungalow | `bungalow` | 4,138 | 2,260 | 24 | 303 |
| Split-level | `split` | 5,778 | 3,612 | 28 | 353 |
| Pueblo Revival | `adobe` | 4,456 | 2,972 | 20 | 228 |
| Stilt house | `stilt` | 4,462 | 2,104 | 14 | 400 |
| Beach cottage | `beach` | 4,386 | 2,496 | 14 | 341 |
| Single-wide trailer | `trailer` | 2,304 | 1,728 | 10 | 94 |
| Streamline diner | `diner` | 3,370 | 1,898 | 10 | 248 |
| Gas station | `gas` | 4,596 | 1,690 | 10 | 139 |
| Corner store | `corner` | 5,680 | 2,524 | 10 | 192 |
| Auto repair | `garage` | 4,070 | 2,784 | 10 | 208 |
| Motor court motel | `motel` | 13,570 | 7,760 | 20 | 895 |
| Strip mall | `strip` | 8,054 | 3,416 | 10 | 229 |
| Burger drive-in | `drivein` | 1,583 | 1,020 | 10 | 114 |
| Quonset workshop | `quonset` | 1,362 | 968 | 10 | 98 |

**Budgets:** LOD0 16,000, LOD1 7,000 (motel 8,500, with the reason in the manifest), LOD2 120. The
exporter refuses a type over budget. The folder is 11 MB.

### Variants, leaves, block furniture (2026-10-26)

- **Variants.** The four suburban types come in three variants each: `ranch`, `ranch_b`, `ranch_c` and so on (plan,
  walls and roof differ; owner: "we may need a few variations of the subrabn house"). A variant's manifest entry has
  `variant_of` and `variant`. 24 kits in all.
- **Nodes.** Besides `LOD0`-`LOD2` a glb has `FURN` (the block furniture, drawn within 100 m, no longer in LOD0) and
  one `LEAF<n>` per door leaf, its geometry relative to its hinge (the node's translation), so the game can swing it.
- **Piece table additions.** `furn_primitives` (FURN's surfaces); `leaves` (`piece`, `hinge`, `shut`, `inward`,
  `width_m`, `height_m`); `furniture` (`piece`, `item`, `at`, `rot` quarter turns, `tint`), one entry per block
  placed from `FURN`, so a modelled piece (`assets/models/furniture/`) can take its place. A piece's `node` says
  which node its `runs` point into (`LOD0` when absent); a piece made only of boxes lists them in `boxes` (a door
  frame is three boxes round its hole), for colliders.
- **Walkable.** Partitions stop short of a door, a porch's steps line up with the front door, and an upper floor
  has its stairwell cut out (the game's colliders walk all of it: `cb_core::world::small_buildings::walk`).

## 2. Conventions

- **Units and placement.** Metres. Each building's front faces +z, it is centred on x = 0 and it
  stands on y = 0, so a lot needs only a position and a heading. Ground under it must be flat to
  the foundation's depth (0.3 m below y = 0); level the lot or sink the building.
- **One primitive per surface.** Each primitive's material is named by its surface key (`brick`,
  `siding`, `shingle`, `glass`...) and carries `extras.surface`.
  - Bind your own material per key. One material serves every building, so a town draws in about
    45 materials however many buildings stand in it.
  - The glb's colours and roughness are stand-ins (`extras.standin: true`).
- **UVs are in metres** (world-scale, as the kit draws them). A surface's texture repeats every
  `extras.tile_m` metres: set the material's UV transform to scale by `1 / tile_m`. Brick courses,
  clapboard boards and corrugations then land at true size on every building. The full table is
  `SURFACES` in `docs/mockups/small_buildings_kit.js` (generator, `tile_m`, roughness, metallic,
  normal strength, stand-in colour).
- **Vertex colour is the paint.** `COLOR_0` (RGB) carries each building's paint and brick shade,
  and the material's base colour multiplies it. Do not drop the attribute.
- **No tangents are stored.** Generate them when real normal maps arrive (the game already does,
  with `with_generated_tangents`, `crates/cb-app/src/world.rs`).
- **Special surfaces.**

  | Surface | What it is | What the game does |
  | --- | --- | --- |
  | `glass`, `glassShop` | transparent (`alphaMode: BLEND`) | in LOD0 the furnished room shows through |
  | `glassFar` | LOD1's opaque dark glass | no room behind it |
  | `lamp` | emissive | brighter at dusk and night |
  | `sign:<TEXT>\|<bg>\|<fg>\|<neon>` | its text, colours and neon flag in `extras`, UVs 0..1 on the face | draw the text into a texture once per sign, emissive at night when `neon` is set |
  | `awning:<colour>` | a striped canvas | stripe colour in `extras`, double-sided |

- **Textures.** The 37 surfaces need Material Maker PBR sets: base colour, normal and ORM
  (CLAUDE.md section 10). That is task 1.1 of the change. Until then, use the stand-in colours, or
  the mockup's canvas look as a guide to what each should be. The canvas generators are mockup
  stand-ins, never game textures.

## 3. Loading in Bevy 0.20

Check every call against docs.rs for the pinned 0.20 (CLAUDE.md section 2). The mech loads
its model with `GltfAssetLabel::Scene(0).from_asset(path)` (`crates/cb-app/src/mech.rs`), and the
same works here for a quick look.

For the game, load meshes, not scenes:
1. **Meshes.** Load each LOD's primitives as mesh handles:
   - `GltfAssetLabel::Primitive { mesh, primitive }`, with mesh 0 = LOD0, 1 = LOD1, 2 = LOD2;
   - the primitive order is the surface order in `<id>.pieces.json` `primitives` for LOD0, and
     `extras.surface` on each primitive for all three.
2. **Materials.** Pair each mesh with your material for its surface key, not the glb's material.
3. **One handle per type.** Keep one mesh handle per (type, LOD, surface), shared by every
   placement, so Bevy batches identical mesh and material pairs across the town.
4. **Spawning.** Spawn a building as a parent transform with one child per primitive of the
   current LOD.
5. **Data.** Read `manifest.json` and `<id>.pieces.json` with serde at startup. They are plain
   JSON. Validate them on load as `data/` is (CLAUDE.md section 6.4).

Placement (which type stands where) belongs to the island session's `island-biomes` work:
- weights by biome and zone (suburb, roadside, village), from `manifest.json` `biomes`;
- `data/world/small_buildings.ron` per design section 5.

## 4. Levels of detail

| LOD | Use within (from the nearest body) | What it is |
| --- | --- | --- |
| LOD0 | 150 m, or any building that has been damaged | everything: frames, sills, see-through glass, interiors, furniture, cars, the piece table |
| LOD1 | 600 m | the shell: no interior, no piece under 4 litres (balusters, knobs, slats), glass opaque |
| LOD2 | 2 km | a box to the eaves and its roof, 10-28 triangles |
| beyond | | fold into the town's impostor or merged far tile |

- Switch with about 10 % hysteresis, so a building at the boundary does not flicker.
- A damaged building stays LOD0 for good: its piece state lives there.
- Interiors are only in LOD0, which is the owner's rule that far buildings carry no generated
  detail (2026-10-24).

## 5. Destruction

The table in `<id>.pieces.json`:

```json
{ "id": "ranch", "lod": "LOD0", "max_side_steps": 3, "collapse_wall_share": 0.5,
  "primitives": ["adobe", "brick", "concrete", ...],
  "pieces": [ { "id": 12, "lo": [x,y,z], "hi": [x,y,z],
               "runs": [[primitive, first_triangle, triangle_count], ...],
               "box":   { "mat": "brick", "tint": [r,g,b] },
               "panel": { "mat": "shingle", "tint": [r,g,b], "c": [x,y,z], "size": [w,t,l], "axes": [[u],[n],[v]] },
               "tag": "furniture" | "car" | "dressing", "interior": true,
               "wall_area_m2": 7.6,
               "up": [ids this piece holds from below], "side": [ids beside it that can hold it] } ] }
```

- **Runs** are triangle ranges in that LOD0 primitive's index buffer. To remove a piece, overwrite
  its index ranges with a repeated index (zero-area triangles) and upload. Its triangles then draw
  nothing, and nothing else moves. Batch the writes for a frame: one upload per touched primitive,
  not one per piece.
- **What stands** (the kit's `standing()`, the one rule):
  - a piece stands while it can be reached from a piece on the ground (`lo.y <= 0.05`) through
    `up` links, and at most `max_side_steps` `side` links;
  - after any damage, re-run it for that building, and drop what is no longer reached;
  - a shell holes a roof (roofs are 1.6 m panels), and a lost wall peels one side off, panel by
    panel.
- **Collapse:**
  - when the alive `wall_area_m2` of a building falls below `collapse_wall_share` of its total,
    everything above the floor slab falls (`lo.y >= 0.6` or `hi.y >= 1.2`);
  - each piece drops after `0.5 + lo.y x 0.04 + up to 0.15` s, under a dust cloud.
- **What a removed piece becomes:**
  - thin pieces (any side under 6 cm: plaster faces, glass, signs) become dust;
  - a `box` piece breaks into chunks of about 1.5 m (2.2 m in a collapse) of its `mat` and `tint`;
  - a `panel` falls as one tilted chunk with its `axes`, `c` and `size`;
  - anything else (a window with its glass, a sofa, a car) falls whole with its own triangles.
- **Creak.** Unsupported pieces wait 0.25-0.85 s, shaking and trickling dust, before they fall.
  That is survey W15's warning, read from the stress before the fall.
- **Debris and rubble:**
  - draw chunks instanced: a unit box per surface, scaled, turned and tinted per chunk, so one
    draw per surface;
  - a chunk that stops is rubble and just stops updating;
  - whole pieces bake into one static rubble mesh per surface when they rest, rebuilt at most
    twice a second, for that surface only;
  - piles: a 0.5 m height grid. A resting piece raises it by its thickness (0.5 m for a whole
    piece), and anything landing where the drop to a neighbour cell is steeper than 35 degrees
    slides off, so walls spread into heaps;
  - cap 1,500 falling things, and no fall longer than 6 s.
- **In `cb-core`:** port `supportLinks`, `standing` and the collapse rule to Rust over the piece
  table, with tests from this page's numbers. The mockup's `blast`, `settleBuilding`,
  `stepDebris` and `breakOff` in `docs/mockups/small_buildings.html` are the reference. Debris
  belongs with `debris-physics`, and the hit and damage path with `building-destruction`, so a
  house breaks through the same code as the city's buildings.

## 6. Optimising

Measured in the mockup (headless node, no GPU; frame time is for the owner's machine to measure,
CLAUDE.md section 7):
- the whole street of 17 is about 80,000 triangles at LOD0;
- physics for 500 falling things costs about 0.02 ms a frame;
- in a full test, 468-647 things fall and all rest within 7 s.

**Draw calls are the cost to watch.**
- A LOD0 building has 15-25 primitives, so 20 buildings within 150 m would be about 400 draws if
  drawn one by one.
  - Bevy batches identical mesh and material pairs, but each type's LOD0 is its own mesh, so the
    gain is per repeated type.
  - Merge undamaged LOD0 buildings of a town tile into one mesh per surface (about 45 draws for
    the tile).
  - Split a building out of the merge the moment it is first hit.
- LOD1 has about 12 primitives and LOD2 2 or 3. Merge every LOD2 building of a tile into one mesh
  per surface.

**Shadows:**
- interior pieces, glass, lamps and signs cast none;
- LOD2 casts none past 600 m;
- falling chunks cast shadows through their instanced draws only.

**Collision:**
- piece bounds for LOD0 near bodies (most pieces are boxes);
- one footprint box for LOD1 and LOD2;
- no collision for interior dressing unless the pilot is inside.

**Texture memory:** about 37 surfaces x 3 maps. At 2048 px BC7 with mips that is about 5.6 MB a
map, about 620 MB in all.
- Interior-only surfaces (plaster, floors, carpet, pine, counter, product) can be 1024 px.
- Concrete, asphalt and gravel are shared with `terrain-materials` and the city, and counted once.

**Variety:**
- re-export with other seeds (`--seed`) for paint, brick shade and brand changes;
- or tint per placement by multiplying the material colour;
- mirror a placement on x for a free variant (flip the winding, or use a negative scale with
  care).

**Interiors:** stream LOD0 interiors only within 150 m, or when a wall opens (survey S3). In the
LOD1 shell, the opaque `glassFar` hides that nothing is behind it.

## 7. Regenerating and checking

```sh
node tools/buildings/export_small_buildings.cjs            # writes assets/models/small_buildings/ (byte-reproducible)
node tools/buildings/check_small_buildings.cjs             # glb against manifest, budgets, piece runs and links
node tools/buildings/check_small_buildings.cjs assets/models/small_buildings "$(npm root -g)/three"   # also loads each glb with three.js
```

The geometry comes from `docs/mockups/small_buildings_kit.js`, the code the Main Street mockup
draws. Change a building there, look at it on the mockup, then re-export and check. The two cannot
drift, because there is only one kit.

## 8. In the game

`cb_core::world::small_buildings` reads these files (the kits, `furniture.json`), places the lots in world
generation, builds the colliders for a pilot on foot and breaks pieces; `crates/cb-app/src/small_buildings.rs`
draws them (openspec `small-buildings` design section 7).

## 9. Not done yet

- Material Maker sets for the 37 surfaces (stand-in colours now).
- Sign text rendering in the game.
- A frame-time measurement of a town at dusk on the owner's machine.
- LOD switching pops (no cross-fade).
