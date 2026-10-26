# Tasks: multi-lock

Built 2026-10-26 with the advanced weapons (owner, "Approve all", then "Go and implement"). Tested in `cb-core`; not
yet playtested.

- [x] 1.1 `Seeker` on `WeaponSpec`; the shoulder missile read as a one-lock seeker from `data/hud/targeting.ron` (`TargetingConfig::seeker`) rather than migrated into every mech's data, see design "Built" (test: `the_shoulder_missile_keeps_its_lock_numbers`)
- [x] 1.2 `Locks`: build while held, drop on range or 1.5 x cone, fewest-locks-then-nearest candidate in stable order (tests: `six_locks_take_two_point_one_seconds`, `a_lone_target_takes_every_lock`, `a_target_leaving_the_cone_loses_its_locks`, `a_pod_never_locks_a_civil_aircraft`)
- [x] 1.3 Release fires one missile per lock, 0.08 s apart; one-lock seekers fire on press when locked (tests: `a_salvo_leaves_a_missile_per_lock_and_reloads_for_eight_seconds`, `a_pod_fires_nothing_on_the_pull`, `a_lost_station_drops_its_queued_shots`; the one-lock missile is today's `Lock`, unchanged: `a_lock_builds_in_a_second_in_range`)
- [ ] 2.1 Cockpit: `xN` on boxes (built), `LOCKS n/m` on the WEAPONS page (built); the pip ring and sweep arc (not yet)
- [x] 2.2 AI release rule (test: `an_ai_pod_lets_go_at_three_locks_on_a_group_and_all_six_on_one`); wired for enemies carrying a pod
- [ ] 3.1 Playtest: a six-missile salvo into a crab wave and onto Big Boy's weak points, recorded and uploaded
