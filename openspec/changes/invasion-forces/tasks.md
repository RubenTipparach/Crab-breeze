# Tasks: invasion-forces

- [x] 1.1 `data/world/units.ron` and `cb-core::units`: tanks, helicopters, fighters, cargo planes, validated (airship parts with 2.2)
- [x] 1.2 Drops and parachutes (tests: `a_dropped_tank_lands_where_it_was_released_plus_drift`, `a_unit_killed_under_its_chute_never_lands`, `a_cargo_plane_drops_its_whole_load_near_the_drop_point_then_leaves`; `a_downed_cargo_plane_takes_its_cargo_with_it` still to do: a plane shot down before its drop loses its load because nothing is released from a dead plane, untested)
- [x] 1.3 Aircraft movement and each unit's brain, pure and tested (tank standoff, helicopter orbit and height band, fighter runs, firing cone and range)
- [x] 2.1 Placeholder models, hit shapes, targeting boxes for every unit (`cb-app::units`; targeting, the cockpit boxes and the scripted pilot read one `Hostiles` list of mechs and vehicles)
- [ ] 2.2 The crab airship: turrets, fighter launches, crab mech drops, engines that bring it down
- [x] 2.3 Defence waves as mixed forces from data (G16): `forces` in `data/missions/defence.ron`; a wave clears when its raiders and vehicles are all destroyed (`DefenceConfig::wave_size`)
- [ ] 3.1 Playtest and measure
