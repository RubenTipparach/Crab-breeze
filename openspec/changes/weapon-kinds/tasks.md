# Tasks: weapon-kinds

Built 2026-10-26 on the owner's "Approve all" and "Go and implement" (survey G22 and G23 decided). What is built is
tested in `cb-core`; nothing here has been seen in a playtest yet (task 5.1).

- [x] 1.1 `ShotKind` in `WeaponSpec`, `rocket: bool` migrated to `kind: Rocket` in every data file; validation per kind (tests: `a_spec_with_an_unknown_kind_field_is_refused`, `old_rocket_data_loads_as_kind_rocket`)
- [x] 1.2 `launch`: pellets from the forked RNG, the slug's charge and cancel, the beam's per-step trace (tests: `a_shotgun_puts_all_pellets_on_a_torso_at_100_m`, `a_shotgun_mostly_misses_at_400_m`, `releasing_a_charging_railgun_fires_nothing`, `a_railgun_fires_one_second_after_the_press`, `the_same_pull_throws_the_same_pattern`)
- [x] 1.3 The beam trace: exact ray-capsule for parts, a 4 m march with refine for the world; its timing test (0.1 ms a step) (tests: `a_ray_enters_a_capsule_at_its_surface`, `a_beam_stops_on_the_first_part_and_passes_its_own`, `a_beam_stops_on_the_ground_before_a_part_behind_it`; the instrument `a_beam_trace_costs_about_a_tenth_of_a_millisecond`, run in release: 0.113 ms on an Apple M4 Max, 30 m over a city, against the 0.1 ms budget)
- [x] 1.4 Plasma: straight flight, burst at `burst_m` or on contact through the blast path (test: `plasma_bursts_at_650_m_in_open_air`)
- [x] 2.1 `HeatBank` shared by the boosters and the weapon bank: the boosters' `Budget`, one implementation, a bank of its own for the weapons (G23) (tests: `the_laser_fills_the_bank_in_five_seconds`, `a_beam_cuts_out_when_the_bank_fills`, `a_rail_shot_into_a_hot_bank_locks_every_heat_weapon`, `the_weapon_bank_refills_in_twenty_seconds`, `two_seconds_of_laser_on_a_torso_takes_120`)
- [x] 2.2 Fire group 3 and `fire_back` in the bindings (G and Mouse 4)
- [x] 3.1 AI fire rules per kind (test: `each_kind_keeps_to_its_own_fire_rule`)
- [x] 3.2 Loadouts in the combat mechs' and crabs' data (after G22): the raiders, the first fight, the hangar's parts
- [ ] 4.1 Weapons MFD page (built: READY, CHARGE, HOT and the bank's HEAT from `Guns::state`); effects recipes (the beam is a stand-in streak); sound cue names handed to `audio` (not yet)
- [ ] 5.1 Playtest: a fight against a shotgun Hatchet, a railgun Brigand and a laser crab, recorded and uploaded
