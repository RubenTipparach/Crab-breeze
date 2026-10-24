# Tasks: mech-rig

## 1. Core (`cb-core::mech`)
- [x] 1.1 Contract names in one list (test: `every_contract_bone_has_a_parent_in_the_contract`)
- [x] 1.2 `MechConfig` and the stand-in in `data/mechs/placeholder.ron` (test: `the_placeholder_follows_the_contract`)
- [x] 1.3 Locomotion step (tests: `full_throttle_reaches_top_speed`, `torso_twist_is_limited_and_independent_of_the_legs`, `the_mech_cannot_walk_into_a_building`)
- [x] 1.4 Gait (tests: `feet_settle_together_at_rest`, `the_feet_alternate`, `a_stance_foot_does_not_slide_at_steady_speed`)
- [x] 1.5 Digitigrade solver (tests: `the_solver_reaches_a_reachable_target`, `the_solver_returns_the_rest_pose_for_the_rest_target`, `the_knee_bends_the_way_the_rest_pose_bends`)
- [x] 1.6 Skate and jet refused (test: `asking_for_an_unbuilt_mode_keeps_walking`)

## 2. App (`cb-app::mech`)
- [x] 2.1 Spawn the stand-in from data, or a glb named in data
- [x] 2.2 Bind by name; a missing bone is reported by name
- [x] 2.3 Drive bones from the pose

## 3. Proof
- [x] 3.1 Screenshot from the cockpit in a street
- [x] 3.2 The first Blender mech (`assets/models/mech_breeze.glb`) loaded by a data change only (`data/mechs/breeze.ron`)
- [ ] 3.3 A playtest take striding a street
- [ ] 3.4 Port Mining Mike's pose layer (idle, walk, turn step, skate pose, hit) from `tools/mockups/gen_mech_designs.py`, reading the `root` extras, so the game and the review page move the same way (one implementation); its own change
