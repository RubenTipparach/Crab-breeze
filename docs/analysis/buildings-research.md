# Buildings: generation, interiors, destruction and rubble (research, 2026-10-24)

Owner, 2026-10-24: "research detailed building generation, interiors and floorplans, furniture etc,
rubble, and fast and efficient breakage of structures. I want buildings to look nice, and be fully
destructible", and "before you put any code into the game, I want to make sure you are building it
right via js mockup".

This note is the investigation behind four OpenSpec changes: `building-generation`,
`building-interiors`, `structural-destruction` and `rubble`. The three.js mockup that tries the
design out is `docs/mockups/buildings.html`, with its shots in `docs/screenshots/buildings/`
(`tools/mockups/shoot_buildings.mjs`). No game code was changed.

## 1. Where the game is today

- `procgen-world` puts one box per lot: 1 226 buildings on seed 42, 2-60 storeys of 3.5 m, six
  concrete and glass tones, batched into a handful of draws.
- `building-destruction` (built) divides each box into cells of about 6 m x 6 m x one storey
  (`cb-core/src/world/destruct.rs`). A blast breaks the cells within reach. A cell with no cell under
  it falls. A storey with under 35 % of its cells standing brings everything above it down as storey
  chunks (Avian bodies, `debris-physics`). The mesh is rebuilt from the standing cells.
- What it can't do: no facades, interiors or furniture; a building reads as voxels once hit; collapse
  needs a whole storey to go; every hit remeshes the building; rubble is a pile of cuboids that
  vanishes past the 600-body budget and that nothing can walk on.

The 6 m cell is the right size and stays. The four changes keep it as the structural grid and
put real things on it.

## 2. Techniques looked at

### 2.1 Generating buildings

- **Shape and split grammars** (Wonka et al., "Instant Architecture", SIGGRAPH 2003; Müller et al.,
  "Procedural Modeling of Buildings", SIGGRAPH 2006, the CGA grammar behind CityEngine). A lot becomes
  a mass, then floors, facades, bays and tiles, each step a rule that splits a shape. This is the
  standard, and it maps straight onto data: one archetype is one rule set. *(From memory, not
  re-checked for this note.)*
- **Houdini building kits**: THE FINALS' fully destructible procedural buildings were built in
  Houdini ([SideFX, 2026-02](https://www.sidefx.com/community/making-the-procedural-buildings-of-the-finals-using-houdini/)).
  The same idea: modules authored once, assembled by rules, fractured offline.
- **Taken**: a split grammar in Rust over the 6 m structural grid. Lot to mass (setbacks, podium) to
  storeys to facade panels (one per structural bay per storey) to modules (window bays, piers,
  spandrels, storefronts, cornices) to roof props. The mockup has four archetypes on one grammar.

### 2.2 Floorplans

- **Squarified treemaps** (Marson and Musse, 2010) and **growth-based room placement** (Lopes et
  al., 2010) split a footprint into rooms by area. **Bayesian layouts** (Merrell, Schkufza and
  Koltun, "Computer-Generated Residential Building Layouts", SIGGRAPH Asia 2010) learn from real
  plans. *(From memory.)*
- **What real buildings do**: offices and flats are double-loaded corridors round a core (stairs,
  lifts, risers). Partitions land on mullions so no wall cuts a window. Rooms are a few facade bays
  wide.
- **Taken**: the architectural rule, not a general packer. Core in a cell every storey shares (so
  the stairs line up). A corridor through it along the long axis, single-loaded when the plan is under
  13 m deep. Rooms fill the bands on 3 m bay lines, typed by the storey's use (retail, lobby, office,
  flat, industrial), and flats subdivide into kitchen, bath, living room and bedroom. It is
  deterministic, cheap (the whole block plans in about 60 ms in a browser) and always gives a corridor
  to every door.

### 2.3 Furniture

- **Interior design guidelines as constraints** (Merrell et al., "Interactive Furniture Layout Using
  Interior Design Guidelines", SIGGRAPH 2011; Yu et al., "Make It Home", SIGGRAPH 2011). Clearance in
  front of things, pairing (desk and chair), against walls, conversation groups. *(From memory.)*
- **Star Crew** already does this for crew cabins: `tools/crew_rooms.py` writes
  `crew_rooms.json` (room, prop, back position, yaw) and checks every piece is inside its room, clear
  of door zones, not overlapping and clear of window sills.
- **Taken**: Star Crew's rule set, in Rust. Each room type lists items in order with a count range.
  Each item is anchored to a wall, the centre or a grid (desk pods, warehouse racks). It needs a
  clearance strip in front, keeps out of door zones and column footprints, and tall things never stand
  in front of windows. About 3 000 items on the mockup's block.

### 2.4 Seeing rooms through windows without building them all

- **Interior mapping** (van Dongen, 2008): the window's shader ray-casts into a fake box room
  ([Game Developer](https://gamedeveloper.com/programming/interior-mapping-rendering-real-rooms-without-geometry)).
  Marvel's Spider-Man is widely believed to use it, though Insomniac has not said
  ([Automaton](https://automaton-media.com/en/news/20231201-23558/)).
- **Taken**: real interiors near the player and wherever a building is breached; interior mapping
  for far windows. The fake room's atlas tile is picked from the same floorplan, so the room you see
  from a kilometre away is the room you find when you blow the wall open.

### 2.5 Destruction

- **NVIDIA Blast** ([docs](https://docs.nvidia.com/gameworks/content/gameworkslibrary/blast/1.1/api_docs/files/pageextstress.html)):
  chunks in a hierarchy, *support chunks* joined by *bonds* (centroid, normal, area). Bonded chunks
  stay together as one actor when the graph splits. The stress solver turns the forces on support
  chunks into bond pressures, compares them with limits and costs time linear in its iterations.
- **THE FINALS** (Embark, GDC 2024, "Engineering Mayhem": construction and fracture of structures, a
  *connection graph*, replication, debris state;
  [GDC](https://gdcvault.com/play/1034280/Engineering-Mayhem-Technical-Deep-Dive)). A later patch added
  gradual destruction so buildings deform before collapsing rather than all at once at a threshold
  ([KitGuru](https://kitguru.net/gaming/joao-silva/embark-studios-reveals-major-destruction-system-update-for-the-finals)).
- **Red Faction: Guerrilla** (Geo-Mod 2.0, structural stress on authored buildings), **Battlefield**
  (precomputed destruction states per building), **Teardown** (voxels), **Unreal Chaos** (geometry
  collections, Voronoi fracture, connectivity graph, strain). *(From memory.)*
- **Star Crew's** `hull-repair` design: walls split at panel bays, integrity stages (scorched,
  buckled, torn, hole) shown by swapping to damaged texture variants, no new geometry.
- **Taken**: Blast's shape at mech scale, with a load path instead of an iterative stress solver.
  - Elements: column, floor bay, wall panel, glass, partition, core, furniture, prop. Each rests on
    other elements or the ground.
  - Connectivity from the ground decides what falls, as islands that drop as one rigid cluster
    (Blast's actors).
  - A top-down load pass decides what is overloaded. A floor puts its weight on the columns at its
    corners whose column line still reaches the ground. A column passes its load down. Capacity is
    2 x the intact load. A floor held on one edge only is a cantilever at 45 % of its capacity. An
    overloaded member creaks for 0.5-1.6 s, then fails.
  - Falling clusters load the floors they land on (2 x their weight) and punch through, so storeys
    pancake.
  - One pass is O(elements). The worst building in the mockup solves in 0.7-2 ms *in JavaScript*.
    That is not a measurement of the game, and the Rust number is measured in `perf-bench` (CLAUDE.md
    section 7).

### 2.6 Fast breakage on the GPU

- The mockup draws each building with **one mesh per material** (7) and tags every vertex with its
  element. A per-building mask texture holds one RGBA texel per element: standing, stress, structure
  flag and storey. The vertex shader collapses a broken element's triangles to a point, and the
  shadow pass reads the same mask. Breaking something writes one texel: no remesh, no new draw.
- In Bevy this is an `ExtendedMaterial<StandardMaterial, _>` with a storage buffer of element bits
  and a custom `u32` vertex attribute, plus the same test in the prepass and shadow shaders. Check
  the 0.20 API on docs.rs before writing it (CLAUDE.md section 2).
- Pieces: the module's pre-fractured cells (Blender, at build time, in the module's glb) are spawned
  from a pool when it breaks. The mockup splits boxes along their longest axis as a stand-in.

### 2.7 Rubble

- Rubble in a game is a heightfield you can walk on with loose pieces over it. Pieces alone are
  expensive and can't be walked on. A heightfield alone looks like a hill.
- **Taken**:
  - The fines of each landed cluster (half its volume in the mockup) go into a heightfield that slumps
    to a 37 degree angle of repose. The game has that heightfield already: `terrain-deformation`'s
    0.5 m tiles, where craters subtract and rubble adds. `height_at` then puts the pilot and the mech
    on top of the pile with no new collision code.
  - Pieces settle onto it and freeze. Past the pool the oldest resting pieces become heap.
  - The core decides how much rubble lands where (deterministic). Avian only places the loose pieces.

### 2.8 Bevy features that matter

- **Decals**: `ForwardDecal` (needs a depth prepass) and `ClusteredDecal` (base colour, normal,
  metallic-roughness and emissive projected along -Z). The docs say clustered decals don't run on
  macOS, iOS, WebGL 2 or WebGPU ([Bevy example](https://bevy.org/examples/3d-rendering/decal/),
  [docs](https://doc.qu1x.dev/bevy_trackball/bevy/pbr/decal/clustered/index.html)). The owner
  playtests on Windows, but `./run.sh` on the Mac would lose scorch and crack decals, so forward decals
  are the fallback there.
- **Parallax**: `StandardMaterial::depth_map` with `ParallaxMappingMethod::Relief`. Brick courses
  and board-formed concrete get depth at no geometry cost. It needs tangents.
- **Batching**: identical mesh and material instances batch automatically, so furniture and roof
  props are instanced per module type.

## 3. Textures: Star Crew's render-to-texture, and Material Maker

What Star Crew does (`Star-Crew-Bridge-Simulator` at `09e8552`, read for this note):

- **Panels modelled and rendered face-on** (`tools/blender/build_wall_panels.py`, `hs_kit.py`,
  `data/materials/panels.json`).
  - A 2 m x 2 m module is carved with CSG (plates, bolts, slats, grilles, diamond plate) and rendered
    orthographic at 512 px/m in Cycles: fixed seed, 16 samples. Wear comes from shader AO, bevel-node
    edge wear, grime and rust.
  - Tiling comes from building strips at x offsets -2, 0 and 2 with periodic noise. Seven trim rows
    (side, baseboard, rib, beam, frame, cove) are stacked in one layer with edge-filled gaps so mips
    don't bleed.
  - Placement is a hash of `seed|room|edge|bay|level` against module weights, never the neighbour's
    module.
  - It is judged on a contact sheet, seam ratios and sha256-reproducible builds.
  - In Star Crew the light is baked into the albedo. Crab Breeze already changed that for the cockpit
    (`panel-textures`, section 13): full PBR maps.
- **Per-prop baked atlases**: a detailed version is baked selected-to-active onto the low-poly prop
  (`bake_atlas`), at 40-128 px/m. The same bake makes the ship exterior LODs (`bake_lod`).
- **Macro plus detail**: a 1024 px macro albedo wrapped once over a big object, times a near-white
  512 px detail albedo and normal tiling every 16 m (`tools/materials/oneill_atlases.py`).
- **Projected paint and decals**: livery and seams in metres projected onto the mesh;
  `tools/decals/build_decals.py` for scorch.

How each part maps to buildings:

| Surface | Source | Why |
| --- | --- | --- |
| Brick, concrete (board-formed), stucco, roofing felt, gravel, asphalt | **Material Maker** graphs, full PBR at 2048 px, with a height map for relief parallax | Bulk tiling materials are what Material Maker is for, and section 10 says so |
| Facade bays (a window with its sill, lintel, reveal and frame; a storefront; a loading door) | **Star Crew face-on render** of the modelled module: albedo, normal, ORM, height, emission | Gives far facades (LOD1 and LOD2) the near facade's detail on flat quads |
| Trims (cornices, copings, string courses, pilaster bases and capitals, sills) | **Star Crew trim rows**, one atlas | Long thin details, thousands of metres of them per block |
| Roof clutter (HVAC, tanks, vents, dishes, antennas) | **Blender props** on `hs_kit`, each with a baked atlas; far roofs get the clutter rendered into the roof cell's texture | Roofs are what a flying mech sees most |
| Signage, graffiti, stains, scorch, cracks | **Decal sheets** | Variety without new materials |
| Big walls (blank party walls, warehouse sides) | **Macro plus detail** | Breaks up a 30 m brick wall that would otherwise tile visibly |

Face-on renders for facades fall outside section 10's rule (Material Maker for every material).
Section 13 has an exception for cockpit panels only, so this needs the owner's word: survey `W14`.

## 4. What the mockup showed

Seed 42, one 108 m x 88 m block, seven buildings, 56 storeys. Counts are the page's own:

- 795 rooms, 2 951 furniture items, 64 roof props.
- 10 454 elements and 21 929 bonds.
- 527 k triangles with all interiors drawn (boxes, not the kit).
- 189 draw calls at rest, about 400 with debris and shadows.
- Generation took 63-75 ms in the browser. Solves were 0.7-2.1 ms for the worst building.

Things it showed that the write-up alone would have missed:

1. **Columns need a fallback support.** A column whose column below breaks must still stand on the
   floor below. Otherwise the whole column line above drops out of an intact building.
2. **A load path alone is too forgiving.** Taking out every ground column on one side left the floors
   above held by their inner columns. The cantilever rule (one edge held, 45 % capacity) peels the bay
   off, storey by storey, which is what reads as a real collapse.
3. **Partitions must be emitted once per line.** Two rooms emitting the same wall z-fights. Band
   boundaries are written once, from the corridor side nearest the spine.
4. **Trims are offset 2 cm** from the faces they sit on (sills, lintels, copings). Otherwise their
   faces share a plane with the brick (CLAUDE.md section 10: no z-fighting).
5. **The heap must come from the core**, not from where physics pieces land, or two machines grow
   different rubble.

## 5. Sources

Every source, with what it was used for, is listed in `docs/analysis/README.md`.
