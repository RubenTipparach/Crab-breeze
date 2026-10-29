# Design: mech arms clear of their legs, by pose

## The measurement

`MOCKUP.clearance(id)` on the Mechs page poses a mech with the pose layer the game mirrors (`tools/mockups/mech_pose.js`),
arm hold included:
- standing (rest and idle);
- 16 phases of the walk and of the turn;
- skating;
- two crouches as deep as the game's leg solver goes (skating and landing).

At each pose it takes every arm mesh (under `shoulder_L` and `shoulder_R`) against every leg mesh (under `thigh_L` and
`thigh_R`) with three-mesh-bvh, which the measuring run loads into the page. A pair that intersects counts as a hit;
otherwise the nearest distance is the gap. It reports the least gap, in metres, and where. `MOCKUP.setHold(id, h)` tries
a hold.

A box test was tried first and misled: a bone's parts are merged per material, so a thigh's decals on both its faces make
one box as wide as the thigh.

## The holds

`armHold` adds to the shoulders `(fwd, 0, out)` (the right shoulder `-out`) and to the elbows `elb`, radians. Each forearm's
lower edge met the top of its thigh's armour. Turning the arm out keeps a forward-pointing weapon pointing ahead; a
forward swing tips it up unless the elbow takes it back, and combined with a turn out it swings the weapon sideways, so the
Harrow and the Roustabout use the turn alone.

| Mech | Before | Hold | After |
|---|---|---|---|
| Harrow | intersecting standing (634 hits) | out 0.8 | 0.38 m, mid-stride |
| Roustabout | intersecting standing (380 hits) | out 0.55 | 0.71 m, mid-stride |
| Mule | intersecting standing (833 hits) | out 0.6, fwd -0.3 (fists up in a guard) | 0.53 m, standing |
| Foreman, Ironhorse, Driller, Picket, Sapper | 0.22 to 1.5 m | none | unchanged |

## In the game (after approval)

The game drives the legs with its own solver (`cb_core::mech::gait`) and leaves the arms at their rest, except during a
melee swing (`melee::arm_rotations`). The hold becomes a per-mech arm rest in `data/mechs/<id>.ron`, applied to the
shoulder and forearm bones under the walk, and the melee swing adds to it rather than replacing it.

## Open questions

| Id | Question | Provisional |
|---|---|---|
| AC1 | Foreman keeps 0.22 m mid-stride; hold it out too? | No: it does not touch |
