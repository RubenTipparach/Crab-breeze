# Tasks: small-buildings

## 0. Mockup
- [x] 0.1 `docs/mockups/small_buildings.html` and `small_buildings_kit.js`: seventeen types on one street, walk, dusk, cutaway
- [x] 0.1b Destruction in the mockup: four weapons, chunks, support by touch, pancake collapse, rubble piles
- [ ] 0.2 Shots in a cloud container (lavapipe), looked at; owner answers S1-S3

## 0b. Kits for the game
- [x] 0.3 Exported kits: `assets/models/small_buildings/` (glb with LOD0-2, piece tables, manifest with budgets) by `tools/buildings/export_small_buildings.cjs`, checked by `check_small_buildings.cjs`; how to use them in `docs/design/small-buildings-kit.md`

## 1. Art (cloud session)
- [ ] 1.1 Material Maker graphs for the 25 surface sets; contact sheet looked at
- [ ] 1.2 Exported to `assets/textures/buildings/`, listed in the material table

## 2. Core (`cb-core::world::small_buildings`)
- [ ] 2.1 Types and options in `data/world/small_buildings.ron`, validated (test: `every_type_builds_with_no_shared_faces`)
- [ ] 2.2 Parts from the seed (tests: `the_same_seed_builds_the_same_house`, `every_window_is_a_hole_with_glass`)
- [ ] 2.3 LOD 1 and 2 from LOD 0 (test: triangle budgets per LOD)

## 3. App and island
- [ ] 3.1 Meshes per material, signs, night emissive
- [ ] 3.2 Placement on the island by biome and zone (the island session)
- [ ] 3.3 Breakable pieces for `building-destruction`
- [ ] 3.4 Measured in a town at dusk; playtest striding through a suburb and a roadside stop, on Drive
- [ ] Each type bakes its far look and massing recipe (`building-type-lods`; `every_building_type_has_its_far_look` passes)
