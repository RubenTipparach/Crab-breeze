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

## 2b. The new crabs, and losing limbs

The crab mechs 23 to 28 (`build_mech_blockouts.py -- crabs`; owner, 2026-10-25: "Can we shoot off each leg or arm")
stand on one, two or three pairs of hind legs, their claws carrying the front. Each leg and each claw arm is one node
in the glb, at its root joint: `LEG_<pair><L|R>` (pair 0 forward) and `ARM_L`, `ARM_R`, every piece of the limb under
it. The manifest lists them (`mech_crabs.json`, `limbs`).

- **Gait**: the claws walk as the front pair (knuckle down, a short stride), so a one-pair crab moves diagonals of claw
  and hind leg, a two-pair crab tripods of claw, hind and hind, and a three-pair crab alternating tripods of hind legs
  with the claws planted between steps.
- **Each limb is a part** with its own armour (`parts`, as the mechs' arms and legs are) and a hit capsule along it.
- **At zero it comes off**: the game unparents the limb's node and hands it to `debris-physics` as one body, with the
  hit's impulse, a spark burst and smoke from the stump; it stays in the world as a wreck (cover, `debris-physics`).
- **What a lost limb costs**, from the sim, not faked (CLAUDE.md 6.1):

| Lost | Effect |
| --- | --- |
| A hind leg, of two or three pairs | top speed times the share of hind legs left; the gait drops to the legs it has |
| A hind leg, of one pair | it falls onto that claw and crawls on the claws and the other leg at a quarter speed |
| Both hind legs on one side | it cannot walk; it turns in place on its claws |
| A claw arm | its gun is gone; the crab leans onto the other claw (a slower, lopsided walk) |
| Both claws | it cannot fire its arm guns; back units only |

The mockup shows it now: on the Mechs page, Scene, Crabs, **Shoot off a limb** blows a random limb of the crab on
screen off whole, with sparks, and **Repair** puts them back.

## 3. Cost

Skitter 21 k, Snapper 25 k and Big Boy 29 k triangles (`assets/models/aliens.json`): ten crabs are
about 230 k triangles, fine on the RTX 3070 (CLAUDE.md 7). The gait is a few microseconds a crab a
step. Measured with `--bench defence` before and after.

## Open questions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| C1 | The new crabs (23 to 28) replace Skitter, Snapper and Big Boy, or join them? | Join: the new six as the heavy waves | provisional |
| C2 | Lost-limb effects as in the table above? | Yes | provisional |
| C3 | Shot-off limbs stay as wrecks in the world? | Yes, as cover, until the mission ends | provisional |
| G14 | What the defence waves are | Wave 1: 10 Skitters; wave 2: 6 Skitters and 4 Snappers; wave 3: 6 Snappers and Big Boy | provisional (survey G14) |
