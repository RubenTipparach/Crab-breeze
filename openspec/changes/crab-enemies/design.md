# Design: crab-enemies

## 1. Data

`data/crabs/skitter.ron`, `snapper.ron`, `bigboy.ron`: `model`, `height_m`, `speed_max_m_s`
(Skitter 16, Snapper 10, Big Boy 4), `strafe_ratio` (Skitter 1.0, Snapper 0.6, Big Boy 0.3),
`turn_deg_s`, `parts` (name, joints, armour in mm, structure), `weapons` (the existing `WeaponSpec`,
on sockets such as `MUZZLE_gun_L`), `stride_m` and `step_height_m`. Validated like the mechs, and a
check loads each glb to prove every joint and socket the file names is in it (CLAUDE.md 6.5).

## 2. Gait (`cb-core::crab`, pure)

Legs are grouped by the rig's names: six legs as tripods (L1 R2 L3, then R1 L2 R3), four as
diagonals (L1 R2, then R1 L2). The gait phase advances by distance travelled over `stride_m`. A leg
in swing lifts by `step_height_m * sin(pi s)` and its tip moves to its rest tip plus half a stride
along the travel; a planted tip stays where it touched (`height_at`). Coxa, femur and tibia are
solved from the tip by two-bone IK in the leg's vertical plane (the rig's yaw and raise axes). The
body's height follows the mean of the planted tips, with a bob of `0.04 * height_m` at twice the
step rate.

Tests: `a_tripod_always_has_three_feet_down`, `planted_feet_do_not_slide`,
`the_ik_reaches_every_tip_in_range`, `a_crab_strafes_with_its_body_facing_the_target`.

## 3. Cost

Skitter 21 k, Snapper 25 k and Big Boy 29 k triangles (`assets/models/aliens.json`): ten crabs are
about 230 k triangles, fine on the RTX 3070 (CLAUDE.md 7). The gait is a few microseconds a crab a
step. Measured with `--bench defence` before and after.

## Open questions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| G14 | What the defence waves are | Wave 1: 10 Skitters; wave 2: 6 Skitters and 4 Snappers; wave 3: 6 Snappers and Big Boy | provisional (survey G14) |
