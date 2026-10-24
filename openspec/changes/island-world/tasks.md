# Tasks: island-world

## 1. Core (`cb-core::world`)
- [x] 1.1 Island, LOD and view tuning in `data/world/world.ron`, validated (test: `the_shipped_world_data_loads`)
- [x] 1.2 Heights as a function, the island's shape, past the edge too (tests: `height_at_matches_the_grid_at_every_sample`, `height_at_lies_on_the_drawn_triangles`, `the_ground_goes_on_past_the_worlds_edge`)
- [x] 1.3 Land in the middle, sea at the edges, floor 20 to 60 m, gentle shore (tests: `the_island_has_land_in_the_middle_and_sea_at_the_edges`, `the_open_sea_floor_is_between_twenty_and_sixty_metres_down`, `the_shore_shelves_gently_enough_to_walk_out_of_the_sea`)
- [x] 1.4 Cities on land, the capital at the centre, encounters on dry land (tests: `the_start_is_in_the_capital_near_the_islands_centre`, `every_encounter_starts_on_dry_land`, `no_city_square_is_under_water`)
- [x] 1.5 Rivers from inland to the sea; channels spare highways except under bridges (tests: `a_river_always_runs_downhill`, `the_ground_under_a_highway_is_the_road_grade`)
- [x] 1.6 Levels of detail, hysteresis, the quadtree, skirts (tests in `world::lod` and `a_skirt_hangs_below_every_edge_and_faces_out`, `the_lod_rings_keep_the_ground_to_a_few_hundred_thousand_triangles`)
- [x] 1.7 Trees grown by tile near the bodies (tests: `trees_grown_in_any_order_are_the_same_trees`, `a_felled_tree_stays_felled_when_its_tile_is_let_go_and_grown_again`, `growing_trees_is_bounded_by_the_bodies_not_the_island`)
- [x] 1.8 One cell index for buildings, rivers, flats and clouds (tests in `world::grid`, `the_indexed_density_is_the_density_over_every_puff`)
- [x] 1.9 Golden seed re-pinned (test: `seed_42_builds_the_golden_world`), determinism kept (test: `the_same_seed_builds_the_same_world`)

## 2. App (`cb-app`)
- [x] 2.1 `ground`: quadtree nodes built off the main thread, swapped without holes (tests in `ground`), colliders near the eye and loose bodies, the far sea floor
- [x] 2.2 The sea sheet following the eye to the far plane (geometry only; the look is another change's)
- [x] 2.3 Trees: `keep_trees` in `SimSet::World`, tiles drawn near the eye, shared meshes and material, shadows near only
- [x] 2.4 Clouds by tile near the eye (coverage 0 for now)
- [x] 2.5 Far plane and haze from `view` data

## 3. Proof
- [ ] 3.1 Measure frame time on foot, skating and flying (`perf-measure`), and read the GPU preprocessing log line, on the owner's go-ahead
- [ ] 3.2 Playtest: skate from the capital to the coast, out onto the shelf, and fly back over the island
