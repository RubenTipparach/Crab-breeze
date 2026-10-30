# Tasks: targeting

- [x] 1.1 `cb-core::targeting`: pick, cycle, lock in range (tests: `the_target_nearest_the_crosshair_is_picked`, `next_cycles_by_distance_and_wraps`, `a_lock_builds_in_a_second_in_range`, `no_lock_out_of_range`, `looking_away_breaks_the_lock`)
- [x] 1.3 Missiles: seeker turn, homing on the locked id (tests: `a_missile_turns_toward_its_target_no_faster_than_its_seeker`, `an_unlocked_missile_flies_straight`); `fire_missile` binding; the scripted pilot fires on a lock
- [x] 1.2 `data/hud/targeting.ron`, loaded and validated; `target_next` binding
- [x] 2.1 Boxes, the selected box, the lead marker from `weapons::lead`, the off-screen arrow, `LOCK`
- [x] 2.2 Playtest: the defence take shows boxes, the lead marker and locks from the cockpit
- [x] Q picks by the crosshair (owner, 2026-10-29: "Q shuold first cycle nearest object...", "cool down 15 seconds for objects close to q"): the enemy nearest the crosshair in the pick cone not picked in the last 15 s, then the rest by priority (a unit shelling the city first, provisional) and distance, then the nearest the crosshair again (`targeting::cycle`, `cycle_cooldown_s`; test `q_takes_the_nearest_the_crosshair_then_the_rest_by_priority_and_distance_then_starts_over`). Not yet built or tested: the owner was playing
