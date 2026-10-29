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

## In the game (after approval)

The game's kneel is `SitPose` (`cb_core::boarding`), blended by the leg solver (`cb_core::mech::gait::pose`, `sit_amount`);
it moves the legs and bows the torso and leaves the arms at rest. It gains three angles, `arm_out_deg`, `arm_forward_deg`
and `elbow_deg` (default 0), written per mech by `tools/mechs/gen_mech_data.py` into `data/mechs/<id>/boarding.ron` from the
same table, and applied to the shoulder and forearm bones times `sit_amount`. Cost: three multiplies per arm bone per frame
while kneeling.

## Open questions

| Id | Question | Provisional |
|---|---|---|
| AC1 | Harrow, Roustabout and Mule touch their thighs standing too; leave it? | Yes, as the owner asked: only the kneel changes |
