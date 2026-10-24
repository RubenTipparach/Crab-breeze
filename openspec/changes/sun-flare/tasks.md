# Tasks: sun-flare

## 1. Data and rules (cb-core)

- [x] 1.1 `data/world/sun_flare.ron` and `cb_core::sun_flare::SunLook`, loaded by `GameData.sun` (tests: `the_data_file_loads_and_validates`, `bad_flare_data_is_refused_by_field`)
- [x] 1.2 The sun's screen position (tests: `a_sun_behind_the_eye_has_no_place_on_screen`, `a_sun_straight_ahead_is_at_the_centre`, `a_sun_at_the_edge_of_the_view_is_at_the_edge_of_ndc`, `the_sun_follows_the_view_not_the_world`, `screen_heights_keep_pixels_square`; scenario "The sun behind the pilot")
- [x] 1.3 The occlusion taps and the fade by depth (tests: `every_tap_sky_is_full_and_every_tap_hidden_is_none`, `an_edge_sliding_over_the_sun_fades_the_flare_in_small_steps`, `taps_off_screen_count_as_clear`, `tap_uvs_centre_on_the_sun_and_mark_the_ones_off_screen`)
- [x] 1.4 Off screen, cloud, water, smoothing, strength, the starburst's turn (tests: `the_flare_fades_smoothly_off_the_edge`, `water_and_cloud_at_the_eye_put_the_flare_out`, `the_fade_follows_without_a_jump`, `strength_scales_with_the_sun_and_the_exposure`, `the_starburst_turns_with_the_view`; scenario "A brighter exposure")

## 2. Shader and passes (cb-app)

- [x] 2.1 `assets/shaders/sun_flare.wesl`: the flare and the cockpit's taps, compiled and validated with naga (tests: `the_flare_shader_compiles_with_and_without_msaa`, `the_flare_uniform_matches_the_wesl_struct`, `the_shader_has_room_for_every_tap_and_ghost`)
- [x] 2.2 `sun_flare::SunFlarePlugin`: bloom on the eyes from data, the eyes' flare pass, the cockpit's taps pass, the medium at the eye, `CB_NO_FLARE=1` and `CB_NO_BLOOM=1` (tests: `a_sun_ahead_is_drawn_and_one_behind_is_not`, `under_water_or_in_cloud_nothing_is_drawn_but_the_cockpit_still_tests`, `the_uniform_carries_the_look_and_the_sun`, `the_bloom_is_the_data_s`, `the_cockpit_s_share_starts_clear`)
- [x] 2.3 `water_render::samplable_depth` generic over the camera marker; `WaterPasses` set for ordering

## 3. Proof

- [ ] 3.1 Screenshots (main session, with the owner's permission): on foot facing the sun; the sun half behind a building's edge; seated with the sun in the front screen; seated with a pillar over the sun; the sun just off the edge; under water facing up toward the sun; in cloud
- [ ] 3.2 Measure (`perf-measure`): default against `CB_NO_FLARE=1` and `CB_NO_BLOOM=1`, seated, facing the sun
- [ ] 3.3 Settle the open questions in the survey; move the requirements into `openspec/specs/sun-flare/spec.md` once a playtest shows them
