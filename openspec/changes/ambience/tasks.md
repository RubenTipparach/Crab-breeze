# Tasks: ambience

- [x] 1.1 Nine stereo beds cut from CC0 recordings by `tools/audio/cut_cc0.py` (spec `tools/audio/cc0_cues.json`) into `assets/audio/ambience/`, with `SOURCES.md`, on the Crab Breeze Sounds page
- [x] 1.2 `data/audio/ambience.ron`, loaded and validated; `cb-tools validate` checks every bed file (tests: `the_shipped_cue_file_loads`, `a_sound_effect_left_unplaced_is_refused`)
- [x] 2.1 `cb-core::audio::ambience_weights` (tests: `the_city_centre_is_all_city`, `weights_sum_to_one`, `the_seat_is_windier_than_the_street`, `night_beds_wait_for_a_time_of_day`, `warehouses_make_a_district_industrial`)
- [x] 2.2 The ambience player (`cb-app/src/ambience.rs`): equal-power gains, 4 s easing, stop under -40 dB, through the cockpit seal, beds started at a random point (built 2026-10-25; not yet heard in a playtest)
- [ ] 3.1 Playtest: walk from open ground into a city and board the mech; the beds fade between zones and the wind rises in the seat
