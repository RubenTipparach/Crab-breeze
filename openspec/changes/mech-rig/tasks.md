# Tasks: mech-rig

## 1. Core (`cb-core::mech`)
- [ ] 1.1 Contract names in one list (test: `every_contract_bone_has_a_parent_in_the_contract`)
- [ ] 1.2 `MechConfig` and the stand-in in `data/mechs/placeholder.ron` (test: `the_placeholder_follows_the_contract`)
- [ ] 1.3 Locomotion step (tests: `full_throttle_reaches_top_speed`, `torso_twist_is_limited_and_independent_of_the_legs`, `the_mech_cannot_walk_into_a_building`)
- [ ] 1.4 Gait (tests: `feet_settle_together_at_rest`, `the_feet_alternate`, `a_stance_foot_does_not_slide_at_steady_speed`)
- [ ] 1.5 Digitigrade solver (tests: `the_solver_reaches_a_reachable_target`, `the_solver_returns_the_rest_pose_for_the_rest_target`, `the_knee_bends_the_way_the_rest_pose_bends`)
- [ ] 1.6 Skate and jet refused (test: `asking_for_an_unbuilt_mode_keeps_walking`)

## 2. App (`cb-app::mech`)
- [ ] 2.1 Spawn the stand-in from data, or a glb named in data
- [ ] 2.2 Bind by name; a missing bone is reported by name
- [ ] 2.3 Drive bones from the pose

## 3. Proof
- [ ] 3.1 Screenshot from the cockpit in a street
- [ ] 3.2 The first Blender mech (`assets/models/mech_*.glb`) loaded by a data change only
- [ ] 3.3 A playtest take striding a street
