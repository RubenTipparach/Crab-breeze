# Tasks: ambience

- [x] 1.1 Nine stereo beds cut from CC0 recordings by `tools/audio/cut_cc0.py` (spec `tools/audio/cc0_cues.json`) into `assets/audio/ambience/`, with `SOURCES.md`, on the Crab Breeze Sounds page
- [ ] 1.2 `data/audio/ambience.ron`, loaded and validated (tests: `a_bed_without_its_file_stops_startup`, `beds_are_the_only_unplaced_effects`)
- [ ] 2.1 `cb-core::audio::ambience_weights` (tests: `the_city_centre_is_all_city`, `weights_sum_to_one`, `the_seat_is_windier_than_the_street`, `night_beds_wait_for_a_time_of_day`)
- [ ] 2.2 The ambience player in `cb-app::audio`: equal-power gains, 4 s easing, stop under -40 dB, through the cockpit seal
- [ ] 3.1 Playtest: walk from open ground into a city and board the mech; the beds fade between zones and the wind rises in the seat
