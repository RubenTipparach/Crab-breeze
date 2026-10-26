# Tasks: civil-air-traffic

## 1. Write-up

- [x] 1.1 Proposal, design and spec (this change); `openspec validate --all`
- [ ] 1.2 Survey questions W45-W49 on `docs/design/survey.html` (handed to the session that owns the survey)

## 2. Simulation (`cb-core::air_traffic`, `data/world/air_traffic.ron`)

- [x] 2.1 Kinds in data, validated (tests: `the_shipped_kinds_are_the_four_the_owner_asked_for_at_true_size`, `a_route_aircraft_that_cannot_hold_the_glide_path_from_cruise_is_refused`)
- [x] 2.2 Routes: roll, climb, cruise, approach, landing on a base's strip (tests: `an_airliner_takes_off_cruises_and_lands_on_the_far_bases_strip`, `on_the_shipped_island_a_flight_from_every_base_lands_at_the_next_over_the_real_ground`, `an_airliner_too_high_over_the_threshold_goes_round_again`)
- [x] 2.3 City circling (test: `a_helicopter_circles_its_city_in_its_band_and_leaves_when_the_city_is_attacked`)
- [x] 2.4 Near the pilot only, out of sight, capped, seeded (tests: `traffic_appears_only_where_the_pilot_cannot_see_it_appear`, `traffic_lives_near_the_pilot_only_and_never_past_each_kinds_cap`, `the_same_seed_flies_the_same_traffic_and_another_does_not`, `hidden_means_far_or_behind`)
- [x] 2.5 Keeping clear of a fight (tests: `a_route_bends_round_a_fight_and_never_enters_it`, `a_flight_whose_base_comes_under_attack_diverts_to_the_nearest_quiet_one`, `no_flight_starts_in_a_fight_or_at_a_base_in_one`, `steering_clear_takes_the_near_tangent_and_leaves_a_zone_it_is_in`)
- [x] 2.6 Hits through the one armour rule, the fall and the wreck (tests: `a_shot_that_does_not_beat_the_armour_does_nothing_and_one_that_does_brings_it_down`, `a_downed_airliner_falls_with_its_momentum_and_crashes`, `the_hit_capsules_cover_the_nose_and_the_wing_tips`, `a_wreck_is_the_same_pieces_every_time_and_lies_round_the_crash`)
- [x] 2.7 Crabs' shots of opportunity (test: `units::a_unit_takes_a_shot_of_opportunity_only_when_its_target_is_not_its_shot`)
- [x] 2.8 The standing hook: nearest base and per-base tally (test: `the_pilots_hits_are_tallied_against_the_nearest_base`)
- [x] 2.9 The career loop's shape: `via` points reported as passed, `Traffic::launch` (test: `a_freighter_overflies_its_via_point_and_says_so_then_lands`)
- [x] 2.10 Neutral targets never picked or cycled to by themselves, selected on purpose (tests: `targeting::a_civil_aircraft_is_never_picked_or_cycled_to_by_itself`, `targeting::a_civil_aircraft_is_selected_only_with_the_crosshair_held_on_it`)
- [x] 2.11 Measure the step (`measure_traffic_step`, design section 8)

## 3. The game (`cb-app::air_traffic`)

- [x] 3.1 Step the traffic, zones from the hostiles, crashes as explosions and debris wrecks
- [x] 3.2 Civil capsules in the shot sweep; `CivilAirHit` and `CivilStanding`; crabs' opportunity fire
- [x] 3.3 Block-out stand-ins drawn from the shape table; rotors and propellers turn
- [x] 3.4 Neutral boxes (`CIV`), the TARGET page's name, the lock boxes on a deliberately selected one
- [x] 3.5 Jet engine loop (stand-in cue `airliner_jet`)
- [x] 3.6 The mockup page `docs/mockups/civil_aircraft.html` from `tools/mockups/gen_civil_aircraft.py`
- [x] 3.10 The pilot's hits charged to the nearest base's standing through `cb_core::civil_standing`, once per
  aircraft however many rounds hit it (survey W34); tested
- [x] 3.7 The owner approves the stand-ins' shapes on the mockup page (2026-10-26: "Approved all of the stand ins"); the freighter's size
  stays open (W48): approving the stand-in is not a choice of size
- [ ] 3.8 Blender models with LODs and PBR sets, after 3.7
- [ ] 3.9 Recorded engine, propeller and rotor sounds (W49)

## 4. Proof

- [ ] 4.1 Playtest: watch an airliner land at the start base, a helicopter circle a city, traffic turn away from a defence wave, and shoot one down from the cockpit (when the owner says Claude may playtest)
- [ ] 4.2 Frame time on the 3070 in the cockpit at the start and over a city, old against new
- [ ] 4.3 Move the requirements into `openspec/specs/civil-air-traffic/spec.md` once 4.1 shows them
