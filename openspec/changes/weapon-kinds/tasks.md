# Tasks: weapon-kinds

- [ ] 1.1 `ShotKind` in `WeaponSpec`, `rocket: bool` migrated to `kind: Rocket` in every data file; validation per kind (tests: `a_spec_with_an_unknown_kind_field_is_refused`, `old_rocket_data_loads_as_kind_rocket`)
- [ ] 1.2 `launch`: pellets from the forked RNG, the slug's charge and cancel, the beam's per-step trace (tests: `a_shotgun_puts_all_pellets_on_a_torso_at_100_m`, `a_shotgun_mostly_misses_at_400_m`, `releasing_a_charging_railgun_fires_nothing`)
- [ ] 1.3 The beam trace: exact ray-capsule for parts, a 4 m march with refine for the world; its timing test (0.1 ms a step)
- [ ] 1.4 Plasma: straight flight, burst at `burst_m` or on contact through the blast path (test: `plasma_bursts_at_650_m_in_open_air`)
- [ ] 2.1 `HeatBank` shared by the boosters and the weapon bank (tests: `the_laser_fills_the_bank_in_five_seconds`, `a_beam_cuts_out_when_the_bank_fills`)
- [ ] 2.2 Fire group 3 and `fire_back` in the bindings
- [ ] 3.1 AI fire rules per kind
- [ ] 3.2 Loadouts in the combat mechs' and crabs' data (after G22)
- [ ] 4.1 Weapons MFD page; effects recipes; sound cue names handed to `audio`
- [ ] 5.1 Playtest: a fight against a shotgun Hatchet, a railgun Brigand and a laser crab, recorded and uploaded
