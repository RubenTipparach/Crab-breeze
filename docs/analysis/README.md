# Analysis

Investigations behind OpenSpec changes. Each note says what was looked at, what was measured and
what was taken.

| Note | What |
| --- | --- |
| `source-repos.md` | What Crab Breeze took from the owner's other repositories, at which revision |
| `buildings-research.md` | Building generation, interiors and furniture, destruction, rubble and building textures (changes `building-generation`, `building-interiors`, `structural-destruction`, `rubble`) |

## Sources for the buildings research (2026-10-24)

Every source used for `buildings-research.md`, the four building changes, the mockup
(`docs/mockups/buildings.html`) and the CPU measurement (`docs/benchmarks/2026-10-24-structure-cpu/`).

### Read on the web for this research

| Source | Used for |
| --- | --- |
| SideFX, "Making the procedural buildings of THE FINALS using Houdini" (2026-02): https://www.sidefx.com/community/making-the-procedural-buildings-of-the-finals-using-houdini/ | Procedural, fully destructible buildings from authored modules and offline fracture |
| Embark Studios (Måns Isaksson), "Engineering Mayhem: Technical Deep Dive into Environmental Destruction in THE FINALS", GDC 2024: https://gdcvault.com/play/1034280/Engineering-Mayhem-Technical-Deep-Dive (schedule: https://schedule.gdconf.com/session/engineering-mayhem-technical-deep-dive-into-environmental-destruction-in-the-finals/900179) | A connection graph for structures, debris state, replication and performance as the topics a shipped game had to solve (abstract only; the recording was not watched) |
| KitGuru, "Embark Studios reveals major destruction system update for THE FINALS": https://kitguru.net/gaming/joao-silva/embark-studios-reveals-major-destruction-system-update-for-the-finals | Gradual destruction: buildings that deform before collapsing, rather than all at once at a threshold; the creak before a member fails |
| NVIDIA, Blast 1.1 docs, Stress Solver (NvBlastExtStress): https://docs.nvidia.com/gameworks/content/gameworkslibrary/blast/1.1/api_docs/files/pageextstress.html | Bond forces turned into pressures over bond areas and compared with limits; cost linear in iterations |
| NVIDIA, Blast 1.1 docs, Introduction: https://docs.nvidia.com/gameworks/content/gameworkslibrary/blast/1.1/api_docs/files/pageintroduction.html | Chunk hierarchy, support chunks as an exact cover, bonds (centroid, normal, area), actors that split when bonds break; the shape of `structural-destruction`'s graph and islands |
| NVIDIA, Blast GameWorks overview: https://docs.nvidia.com/gameworks/content/gameworkslibrary/blast/blast.htm | As above |
| NVIDIA Omniverse, Blast Destruction extension: https://docs.omniverse.nvidia.com/extensions/latest/ext_blast.html | Damage unified into a stress model per destructible (omni.blast 0.11 and later) |
| Joost van Dongen, "Interior Mapping: rendering real rooms without geometry" (Game Developer): https://gamedeveloper.com/programming/interior-mapping-rendering-real-rooms-without-geometry (also on 80.lv: https://80.lv/articles/interior-mapping-rendering-real-rooms-without-geometry) | Fake rooms behind far windows, picked from an atlas; `building-interiors` section 3 |
| Automaton, "Are there really rooms behind every window in Marvel's Spider-Man?": https://automaton-media.com/en/news/20231201-23558/ | Interior mapping in a shipped open city (not confirmed by Insomniac) |
| Bevy examples, Decal rendering: https://bevy.org/examples/3d-rendering/decal/ | `ForwardDecal` with `ForwardDecalMaterial<StandardMaterial>`, which needs a depth prepass |
| Bevy API docs (a mirror at doc.qu1x.dev), `ClusteredDecal` and the clustered decal module: https://doc.qu1x.dev/bevy_trackball/bevy/light/struct.ClusteredDecal.html and https://doc.qu1x.dev/bevy_trackball/bevy/pbr/decal/clustered/index.html | Clustered decals project base colour, normal, metallic-roughness and emissive along -Z; not on macOS, iOS, WebGL 2 or WebGPU. Check docs.rs for Bevy 0.20 before writing code (CLAUDE.md section 2) |
| Bevy examples, Deferred rendering: https://bevy.org/examples-webgpu/3d-rendering/deferred-rendering/ | `StandardMaterial::depth_map` with `ParallaxMappingMethod::Relief` (needs tangents) for brick and concrete relief |
| Bevy PR 5928 (parallax mapping): https://github.com/bevyengine/bevy/pull/5928 | Depth map convention (white is bottom) |

### Cited from memory (not re-read for this research)

Named in `buildings-research.md` with "(From memory)". Titles and venues as remembered. Check them
before quoting a detail.

| Source | Used for |
| --- | --- |
| Peter Wonka, Michael Wimmer, François Sillion, William Ribarsky, "Instant Architecture", SIGGRAPH 2003 | Split grammars for facades |
| Pascal Müller, Peter Wonka, Simon Haegler, Andreas Ulmer, Luc Van Gool, "Procedural Modeling of Buildings", SIGGRAPH 2006 (the CGA grammar behind CityEngine) | Lot to mass to floors to facade to bays to tiles; the grammar in `building-generation` |
| Fernando Marson, Soraia Raupp Musse, "Automatic Real-Time Generation of Floor Plans Based on Squarified Treemaps Algorithm", 2010 | Floorplans by area; looked at, not taken |
| Ricardo Lopes, Tim Tutenel, Ruben Smelik, Klaas Jan de Kraker, Rafael Bidarra, "A Constrained Growth Method for Procedural Floor Plan Generation", 2010 | As above |
| Paul Merrell, Eric Schkufza, Vladlen Koltun, "Computer-Generated Residential Building Layouts", SIGGRAPH Asia 2010 | As above |
| Paul Merrell, Eric Schkufza, Zeyang Li, Maneesh Agrawala, Vladlen Koltun, "Interactive Furniture Layout Using Interior Design Guidelines", SIGGRAPH 2011 | Clearance, pairing, walls and groups as furniture rules |
| Lap-Fai Yu, Sai-Kit Yeung, Chi-Keung Tang, Demetri Terzopoulos, Tony F. Chan, Stanley Osher, "Make It Home: Automatic Optimization of Furniture Arrangement", SIGGRAPH 2011 | As above |
| Red Faction: Guerrilla (Volition, 2009), Geo-Mod 2.0 | Structural stress on authored buildings |
| Battlefield: Bad Company 2 and later (DICE, Frostbite) | Precomputed destruction states per building |
| Teardown (Tuxedo Labs, 2022) | Voxel destruction; looked at, not taken |
| Unreal Engine Chaos Destruction (Epic) | Geometry collections, Voronoi fracture, connectivity graph, strain thresholds |

### The owner's repositories, read locally

| Source | Revision | Used for |
| --- | --- | --- |
| Star-Crew-Bridge-Simulator: `.claude/skills/panel-textures/SKILL.md`, `tools/blender/build_wall_panels.py`, `tools/blender/hs_kit.py` (`bake_atlas`), `data/materials/panels.json`, `tools/materials/postprocess.py` | `09e8552` | Face-on Cycles renders of modelled panels, trim rows in one atlas, hash placement without repeating a neighbour; the method proposed for facade bays, trims and far facades |
| Star-Crew-Bridge-Simulator: `data/materials/prop_atlas.json`, `tools/blender/build_ship_exteriors.py` (`bake_lod`) | `09e8552` | Baked prop atlases and LOD atlases; roof props and far roofs |
| Star-Crew-Bridge-Simulator: `tools/materials/oneill_atlases.py` | `09e8552` | Macro plus tiling detail for large surfaces; blank party walls |
| Star-Crew-Bridge-Simulator: `.claude/skills/material-maker/`, `tools/materials/*.ptex` pipeline | `09e8552` | Material Maker for bulk materials (brick, concrete, roofing) |
| Star-Crew-Bridge-Simulator: `tools/decals/build_decals.py`, `openspec/changes/repairs-on-deck/design.md` | `09e8552` | Scorch decals |
| Star-Crew-Bridge-Simulator: `tools/crew_rooms.py`, `data/ships/tern/crew_rooms.json` | `09e8552` | Furniture placement checks (inside the room, clear of doors and sills, no overlaps); `building-interiors` section 2 |
| Star-Crew-Bridge-Simulator: `openspec/changes/hull-repair/design.md`, `data/ships/tern/damage.json` | `09e8552` | Damage stages before a break (scorched, buckled, torn, hole) by swapping variants |
| Star-Crew-Bridge-Simulator: `.claude/skills/blender-csg-levels/references/city-scale.md` | `09e8552` | Undercity's city pipeline (layout, shapely plan, Blender) |
| Star-Crew-Bridge-Simulator: `.claude/skills/light-baking/SKILL.md`, `docs/mockups/lib/lightbake.js` | `09e8552` | Read; not taken for buildings (Crab Breeze lights live) |

### Crab Breeze's own code and data

| Source | Used for |
| --- | --- |
| `crates/cb-core/src/world/destruct.rs`, `crates/cb-app/src/buildings.rs`, `openspec/changes/building-destruction/` | Today's 6 m cells, which `structural-destruction` replaces and whose grid it keeps |
| `openspec/changes/procgen-world/design.md`, `data/world/world.ron` | Lots, storeys (3.5 m), 1 226 buildings on seed 42 |
| `openspec/changes/terrain-deformation/design.md` | The crater rule `R = 2.2 m x E^(1/3)` reused for blasts; the 0.5 m height tiles reused for rubble |
| `openspec/changes/debris-physics/design.md` | Avian debris and the 600-body budget |
| `crates/cb-core/src/rng.rs` | SplitMix64 with labelled forks, mirrored in the mockup |
| `docs/benchmarks/2026-10-24-baseline/report.md` | The current frame times (world take p95 3.0 ms on the M4 Max) the building cost is set against |
