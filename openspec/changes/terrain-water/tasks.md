# Tasks: terrain-water

- [x] 1.1 `data/world/water.ron`, validated (test: `the_shipped_water_data_loads`)
- [x] 1.2 Sea level and lakes (test: `no_city_square_is_under_water`)
- [x] 1.3 Rivers by steepest descent, carved channel (tests: `a_river_always_runs_downhill`, `rivers_keep_clear_of_cities`)
- [x] 1.4 Bridges where highways cross (test: `every_highway_crossing_has_a_bridge_at_road_grade`)
- [x] 1.5 `water_at` and golden seed (test: `seed_42_builds_the_golden_world` extended)
- [x] 2.1 Wading drag, speed cap, refusal (tests: `wading_deeper_is_slower`, `water_over_the_cockpit_stops_the_mech`)
- [x] 2.2 Pilot stopped by deep water (test: `the_pilot_cannot_walk_into_deep_water`)
- [x] 3.1 Water meshes and material, interactive ripples, spray and steam
- [ ] 3.2 Measure in the cockpit, wading (perf-measure)
- [x] 3.3 Playtest: stride into a river, across, out
