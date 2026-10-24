# Tasks: targeting

- [x] 1.1 `cb-core::targeting`: pick, cycle, lock in range (tests: `the_target_nearest_the_crosshair_is_picked`, `next_cycles_by_distance_and_wraps`, `a_lock_builds_in_a_second_in_range`, `no_lock_out_of_range`, `looking_away_breaks_the_lock`)
- [x] 1.3 Missiles: seeker turn, homing on the locked id (tests: `a_missile_turns_toward_its_target_no_faster_than_its_seeker`, `an_unlocked_missile_flies_straight`); `fire_missile` binding; the scripted pilot fires on a lock
- [x] 1.2 `data/hud/targeting.ron`, loaded and validated; `target_next` binding
- [x] 2.1 Boxes, the selected box, the lead marker from `weapons::lead`, the off-screen arrow, `LOCK`
- [x] 2.2 Playtest: the defence take shows boxes, the lead marker and locks from the cockpit
