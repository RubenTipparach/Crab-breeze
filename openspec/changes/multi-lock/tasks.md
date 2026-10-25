# Tasks: multi-lock

- [ ] 1.1 `Seeker` on `WeaponSpec`; lock fields moved out of `TargetingConfig` into the shoulder missile's seeker, data migrated (test: `the_shoulder_missile_keeps_its_lock_numbers`)
- [ ] 1.2 `Locks`: build while held, drop on range or 1.5 x cone, fewest-locks-then-nearest candidate in stable order (tests: `six_locks_take_two_point_one_seconds`, `a_lone_target_takes_every_lock`, `a_target_leaving_the_cone_loses_its_locks`)
- [ ] 1.3 Release fires one missile per lock, 0.08 s apart; one-lock seekers fire on press when locked (test: `a_one_lock_seeker_plays_as_before`)
- [ ] 2.1 Cockpit: `xN` on boxes, the pip ring and sweep arc
- [ ] 2.2 AI release rule
- [ ] 3.1 Playtest: a six-missile salvo into a crab wave and onto Big Boy's weak points, recorded and uploaded
