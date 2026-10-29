# Design: mech arms clear of their legs when kneeling

## The measurement

A measuring run on the Mechs page poses a mech through its kneel (the page's `sitPose` at 25, 50, 75 and 100%, blended
with the standing idle as the page blends them) and takes every arm mesh (under `shoulder_L` and `shoulder_R`) against
every leg mesh (under `thigh_L` and `thigh_R`) with three-mesh-bvh, loaded into the page by the run only. A pair that
intersects is a hit; otherwise the nearest distance is the gap.

| Mech | Standing | 75% kneel | 100% kneel |
|---|---|---|---|
| Foreman | 0.36 m | intersecting | intersecting |
| Harrow | intersecting | intersecting | intersecting |
| Roustabout | intersecting | intersecting | intersecting |
| Mule | intersecting | intersecting | intersecting |
| Driller | 0.94 m | 0.84 m | 0.66 m |
| Ironhorse | 1.40 m | 0.82 m | 0.35 m |
| Picket, Sapper | 1.2 m or more | 1.5 m or more | 1.5 m or more |

The Harrow, Roustabout and Mule already touch standing: a forearm's lower edge rests on the top of its thigh armour.
That is left as it is, as the owner asked; the kneel is what drives the forearms deep into the thighs, as in the owner's
screenshot.

## The kneel offset

The kneel already swings both shoulders forward 0.5 rad and bends the elbows 0.6 rad while the thighs fold up toward the
chest. `KNEEL_ARMS` adds, per mech, `fwd` to the shoulders' swing, `out` to their turn outward (the right shoulder
`-out`) and `elb` to the elbows (negative bends further), all times the kneel's own ease, so standing is untouched and the
offset is whole when sat. A grid search took the smallest change that leaves at least 0.2 m at 75% and 100%:

| Mech | Offset | Gap at 75% and 100% | What it looks like |
|---|---|---|---|
| Foreman | elb -0.3 | 0.73 m | the elbows fold a little more, the tools lift off the knees |
| Harrow | elb -0.6 | 0.23 m | the elbows fold, the guns rise and point ahead |
| Roustabout | out 0.6, fwd -0.25 | 0.37 m | the arms brace out past the knees |
| Mule | out 0.6, fwd -0.5 | 0.30 m | the fists brace out past the knees |

## The shoulder axles

The shoulder joint (`contract_arm`) stands off the torso by 3 to 5.5 kit units on every mech, and the arm's parts are all
on the shoulder's side of it, so the arm floats beside the torso. `axle` adds a cylinder on the chest along x, from 0.55 of
the joint's offset (inside the torso) out to the joint: radius 3.2 kit units on the Foreman, 3.0 on the Ironhorse, 2.0 x
the arm's scale on the walkers (0.64 to 0.93 m). It turns nothing and moves nothing; the shoulder still pivots at the joint,
which is now the axle's end. 136 triangles a mech. Built only with `CB_SHOULDER_AXLES=1` until approved.

## In the game

The game's kneel is `SitPose` (`cb_core::boarding`), blended by the leg solver (`cb_core::mech::gait::pose`, `sit_amount`,
eased as the page's `e`). It moved the legs and bowed the torso and left the arms at rest, so the page's measured
clearances did not hold in the game. It now carries three angles, `arm_out_deg`, `arm_forward_deg` and `elbow_deg`
(default 0), and they are the page's whole kneel arm pose, not only the clearance offset: the arm pose every mech takes in
`sitPose` (shoulders 0.5 rad about their side axis and 0.1 rad outward, elbows -0.6 rad) plus the mech's `KNEEL_ARMS` row,
summed per axis as the page adds them. `tools/mechs/gen_mech_data.py` reads both out of `tools/mockups/mech_pose.js` (it
refuses a `sitPose` whose arm lines it cannot read, or whose arms are not mirrors) and writes the sum into every rostered
`data/mechs/<id>/boarding.ron`, the way it already writes the shared torso bow: there is no shared mech data file, so the
base is written into each mech's line rather than kept in a second place. At full sit that is out 5.7, forward 28.6, elbow
-34.4 degrees on the Driller, Ironhorse, Picket and Sapper; Foreman elbow -51.6, Harrow elbow -68.8, Roustabout out 40.1
and forward 14.3, Mule out 40.1 and forward 0.0 (all with the base's other angles). `gait::kneel_arm_offsets` turns each
shoulder forward then out (the page's Euler order), the right the mirror way, and bends each forearm, times `sit_amount`;
`cb-app` applies each turn in the bone's own rest axes. Standing the arms are untouched. Cost: a few quaternion
multiplies per arm bone per frame, only while kneeling. Proven by unit tests on the real data; not yet seen in the running
game (playtest 1.6).

## Open questions

| Id | Question | Provisional |
|---|---|---|
| AC1 | Harrow, Roustabout and Mule touch their thighs standing too; leave it? | Yes, as the owner asked: only the kneel changes |
