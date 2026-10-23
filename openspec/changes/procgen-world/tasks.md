# Tasks: procgen-world

## 1. Core (`cb-core::world`)
- [ ] 1.1 `WorldConfig` in `data/world/world.ron`, validated (test: `the_shipped_world_data_loads`)
- [ ] 1.2 Base terrain and `height_at` on the mesh's triangles (test: `height_at_matches_the_grid_at_every_sample`)
- [ ] 1.3 Cities, spacing and open ground (test: `cities_keep_their_minimum_spacing`)
- [ ] 1.4 Highways by minimum spanning tree, graded terrain (test: `the_ground_under_a_highway_is_the_road_grade`)
- [ ] 1.5 Streets, blocks, lots, buildings (tests: `streets_are_wide_enough_for_a_mech`, `buildings_run_from_two_storeys_to_forty_and_more`, `no_building_stands_on_a_street`)
- [ ] 1.6 Golden seed (test: `seed_42_builds_the_golden_world`) and determinism (test: `the_same_seed_builds_the_same_world`)
- [ ] 1.7 `push_out_of_buildings` (test: `a_body_inside_a_building_is_pushed_to_its_wall`)

## 2. App (`cb-app::world`)
- [ ] 2.1 Terrain chunks, roads, buildings, sun and fog
- [ ] 2.2 `--seed <n>`

## 3. Proof
- [ ] 3.1 Screenshots of the city from the street and from the cockpit in `docs/screenshots/`
- [ ] 3.2 A playtest take striding a street (needs `playtest-pipeline`, or the owner playing)
