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
- [x] 1.5 Roof props clear of the parapet and each other (test: `roof_props_keep_clear_of_the_roof_edge`, in `cb-core::world::dressing`)
- [ ] 1.6 Golden seed (test: `seed_42_builds_the_golden_block`)

## 2. Art
- [ ] 2.1 `tools/blender/build_building_kit.py`: modules with LODs, `FRAC_` cells, manifest budgets
- [ ] 2.2 Material Maker graphs for brick, concrete, stucco, roofing, asphalt; contact sheet looked at
- [ ] 2.3 Face-on bay renders and the trim atlas; contact sheet looked at

## 3. App (`cb-app::buildings`)
- [x] 3.0 Dressing on the plan's boxes (`cb-core::world::dressing`): pilasters on the column lines, a string course or floor band per floor, parapet and coping, roof props (timber water tanks on legs, HVAC, ducts, vents, chimneys, aerials, masts, a stair bulkhead), styled walk-up, office or tower by height; each piece anchored to its frame element and dropped when that element falls (tests: `the_same_building_is_dressed_the_same`, `every_cut_piece_names_an_element_on_the_grid`, `a_whole_building_takes_far_fewer_pieces_than_a_cut_one`, `dressing_drops_the_pieces_whose_element_fell`). Streamed within 450 m of the camera, 6 buildings a frame. A whole 60-storey tower: 320 pieces, about 4 300 triangles, 0.03 ms to generate; cut after damage: about 43 000 triangles, 0.06 ms (release, M4 Max, indicative)
- [ ] 3.1 LOD0 meshes per material with the `element` attribute; LOD1, LOD2
- [ ] 3.2 Z-fighting check on the generated meshes
- [ ] 3.3 Measured in `perf-bench` (downtown, cockpit) on the owner's machine
- [ ] 3.4 Playtest: walk the block on foot and stride it in the mech, on Drive
