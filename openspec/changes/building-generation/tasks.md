# Tasks: building-generation

## 0. Mockup (done before any game code, owner 2026-10-24)
- [x] 0.1 `docs/mockups/buildings.html`: the grammar, four archetypes, roof props, true scale beside the pilot and mech
- [x] 0.2 Shots in `docs/screenshots/buildings/` (`tools/mockups/shoot_buildings.mjs`)
- [ ] 0.3 Owner looks at the mockup and answers W10 and W14

## 1. Core (`cb-core::world::building`)
- [ ] 1.1 `BuildingsConfig` from `data/world/buildings.ron`, validated (test: `the_shipped_building_data_loads`)
- [ ] 1.2 Mass, storeys and setbacks on the 6 m grid (test: `a_tower_steps_in_above_its_podium`)
- [ ] 1.3 Columns, floor bays, wall panels, parapets as elements with their supports (test: `every_element_but_the_ground_floor_rests_on_another`)
- [ ] 1.4 Facade modules per skin (test: `no_partition_line_crosses_a_window`)
- [ ] 1.5 Roof props clear of the parapet and each other (test: `roof_props_keep_clear_of_the_edge`)
- [ ] 1.6 Golden seed (test: `seed_42_builds_the_golden_block`)

## 2. Art
- [ ] 2.1 `tools/blender/build_building_kit.py`: modules with LODs, `FRAC_` cells, manifest budgets
- [ ] 2.2 Material Maker graphs for brick, concrete, stucco, roofing, asphalt; contact sheet looked at
- [ ] 2.3 Face-on bay renders and the trim atlas; contact sheet looked at

## 3. App (`cb-app::buildings`)
- [ ] 3.1 LOD0 meshes per material with the `element` attribute; LOD1, LOD2
- [ ] 3.2 Z-fighting check on the generated meshes
- [ ] 3.3 Measured in `perf-bench` (downtown, cockpit) on the owner's machine
- [ ] 3.4 Playtest: walk the block on foot and stride it in the mech, on Drive
