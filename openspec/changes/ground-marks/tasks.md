# Tasks: ground-marks

- [x] 1.1 Tuning in `data/fx/ground_marks.ron`, loaded and validated by `GameData` (`cb_core::ground_marks::GroundMarksConfig`) (tests: `the_shipped_file_loads_and_matches_the_defaults_it_documents`, `a_break_shorter_than_a_segment_and_an_unknown_key_are_refused`; `cargo run -p cb-tools -- validate data/`)
- [x] 1.2 The surface rule `WorldPlan::surface_at` on the shared cover rule, `mark_surface` with water first (tests: `a_road_gives_concrete_a_meadow_gives_dirt_and_water_gives_nothing`, `a_park_in_a_city_is_soil`)
- [x] 1.3 The one foot-strike rule `cb_core::mech::foot_strikes` and `stance_lead_m`, `mech.rs` and the pose moved onto them (tests: `the_left_foot_strikes_at_each_whole_cycle_and_the_right_half_a_cycle_later`, `a_settled_stride_or_a_lost_leg_strikes_nothing_but_a_landing_strikes_both`, `a_foot_comes_down_a_quarter_stride_ahead_of_its_rest_ankle`)
- [x] 1.4 Marks, the ring and the fade (tests: `past_its_capacity_the_ring_replaces_the_oldest_mark_first`, `a_mark_shows_fully_for_two_thirds_of_its_life_then_fades_to_nothing`, `a_mark_with_a_broken_position_is_refused_and_changes_nothing`, `an_empty_slot_packs_as_zeros_so_it_is_not_drawn`, `a_footprint_lies_on_the_slope_and_points_where_the_mech_walks`)
- [x] 1.5 Skate trails end to end (test: `trail_segments_butt_end_to_end`)
- [x] 1.6 The jolt (tests: `the_jolt_drops_fast_recovers_and_ends`, `the_jolt_moves_the_eye_and_never_turns_it`, `the_left_foot_is_on_the_mechs_left`)
- [x] 2.1 `assets/shaders/ground_marks.wesl`, Mining Mike's dent shader ported with the soil and paving looks; compiled with naga against Bevy 0.20 (test: `the_marks_shader_compiles_against_bevy_pbr_and_its_uniform_and_rows_match`); the mesh (test: `the_mesh_is_a_five_by_five_grid_a_slot_and_nothing_else`)
- [x] 2.2 `cb-app::ground_marks`: the `GroundContact` message from `mech.rs`, marks laid per surface, crater scorches, one entity and one draw, the buffer rewritten only while marks live, `ViewJolt` into `pilot::place_camera`, the prints into the grass (first as pressing spheres; section 10 replaced them with `PrintCrush`)
- [x] 2.3 Effects per surface in `data/fx/fx.ron` (`foot_strike_*`, `landing_*`, `skate_dirt`; `foot_dust` replaced) (test: `the_ground_picks_the_effect`)
- [x] 2.4 `CB_NO_GROUND_MARKS=1` lays and draws no mark
- [x] 2.5 Second pass (design section 10): bold paving prints (crushed sole, slabs, outline, long cracks, dust ring, 0.10 m), deep broken soil prints, the sole's size apart from the quad (test: `a_paving_print_is_deep_and_its_quad_reaches_past_the_sole`)
- [x] 2.6 Soil prints and grooves crush the grass through its density for their life (tests: `a_crushed_cell_packs_zero_and_the_rest_packs_as_baked`, `the_crush_version_moves_when_a_soil_mark_comes_or_goes`); `MAX_PUSHERS` back to 8
- [x] 2.7 No vertex stage reads `lights` (test: `no_game_shader_reads_the_view_lights_in_a_vertex_stage`)
- [ ] 3.1 Screenshots (main session): prints in a meadow and on a city street, a skate trail on both, a crater, from the pilot's eye on foot and from the cockpit; tune the look (open question 4)
- [ ] 3.2 Benchmark on the owner's hardware with and without `CB_NO_GROUND_MARKS=1`, interleaved; replace design section 8's estimates
- [ ] 3.3 Playtest: walk, skate and land on soil and on paving from the cockpit; judge the jolt (survey M11)
- [ ] 3.4 Move the requirements into `openspec/specs/ground-marks/spec.md` once the playtest and the benchmark pass
