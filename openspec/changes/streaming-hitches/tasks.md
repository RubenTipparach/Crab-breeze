# Tasks: streaming-hitches

- [x] 1.1 Instruments: every slow frame named by its systems (`--features hitch_spans`), `CB_HITCH_MS`,
      `CB_NATIVE_1080`, the `SkateTo` and `JetTo` beats, the streaming route, `tools/perf/hitch_causes.py`
- [x] 1.2 Baseline on the M4 Max, frames over 16.7 ms by cause (report)
- [x] 2.1 Car transforms written only when they move; measured: transforms written a frame
- [x] 2.2 Trees: `must_radius_m` and `urgent_tiles_per_step`; test `a_body_new_to_a_place_grows_its_near_trees_at_once_and_the_rest_a_few_a_step`
- [x] 2.3 Ambience beds skip to their start on the audio thread; tests `a_bed_plays_from_its_start_after_a_silent_catch_up_as_bevys_start_position_did`, `a_bed_goes_round_again_from_the_recordings_beginning`
- [ ] 3.1 The same route on the RTX 3070 (`--features hitch_spans`), old against new, and the free-roam-fight (trees in a fight)
- [ ] 3.2 Music cues still start through Bevy's own path (no start position, so no skip): measure them on the 3070
