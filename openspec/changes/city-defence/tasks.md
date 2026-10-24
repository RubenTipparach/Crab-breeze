# Tasks: city-defence

- [x] 0.1 Measurement tool `cb-tools cities [seed]`; the cities' numbers in the design
- [x] 1.1 `data/missions/defence.ron`, loaded and validated
- [x] 1.2 `Defence` phases (tests: `clearing_a_wave_starts_the_next_after_the_lull`, `a_city_past_half_damage_loses_the_mission`, `a_wave_not_cleared_in_ten_minutes_loses`, `clearing_all_three_waves_wins`, `losing_the_mech_loses_the_mission`)
- [x] 1.3 `city_damage` (test: `city_damage_counts_cells_not_buildings`)
- [x] 1.4 Spawn points (test: `spawn_points_are_the_same_for_the_same_seed`, `spawn_points_are_dry_and_inside_the_world`)
- [x] 1.5 `raider_goal` (tests: `a_raider_far_from_the_pilot_shells_the_city`, `only_the_nearest_three_turn_on_the_pilot`, `a_raider_hit_turns_on_the_pilot`)
- [x] 2.1 `DefencePlugin`: waves spawned at runtime, goals fed to the enemies' brains, burst windows, logs
- [x] 2.2 The cockpit readout
- [x] 2.3 `--mission`, `--showcase`, `--bench defence`; the shared boarding beat
- [ ] 3.1 Playtest: the scripted pilot's winning run recorded with OBS and uploaded to Drive; report in `docs/playtests/`
- [ ] 3.2 Retune the siege bursts from the run's `MISSION status` log
- [ ] 3.3 Benchmark `--bench defence` on the RTX 3070 machine; report in `docs/benchmarks/`
