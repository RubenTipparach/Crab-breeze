# Design: mech arms clear of their legs

## The measurement

`MOCKUP.clearance(id)` on the Mechs page poses a mech with the pose layer the game mirrors (`tools/mockups/mech_pose.js`):
- standing (rest and idle);
- 16 phases of the walk and of the turn;
- skating;
- two crouches as deep as the game's leg solver goes (skating and landing).

At each pose it takes every arm mesh (under `shoulder_L` and `shoulder_R`) against every leg mesh (under `thigh_L` and
`thigh_R`) with three-mesh-bvh, which the measuring run loads into the page. A pair that intersects counts as a hit;
otherwise the nearest distance is the gap. It reports the least gap, in metres, and where.

A box test was tried first and misled: a bone's parts are merged per material, so a thigh's decals on both its faces make
one box as wide as the thigh.

| Mech | Before | After |
|---|---|---|
| Harrow | intersecting standing (515 hits) | 0.35 m, mid-stride |
| Roustabout | intersecting standing (71 hits) | 0.70 m, mid-stride |
| Mule | intersecting standing (475 hits) | 0.83 m, mid-stride |
| Foreman | 0.22 m | unchanged |
| Ironhorse | 0.54 m | unchanged |
| Driller | 0.46 m | unchanged |
| Picket | 1.5 m or more | unchanged |
| Sapper | 1.09 m | unchanged |

## The fix

Each forearm's lower edge met the top of its thigh's armour, which flares up to hip height. Turning the upper arm out
alone (`splay`) moved a hand 0.5 m for 0.16 rad, and not the forward-pointing gun pods at all. So the shoulders move:

| Mech | Out | Up | Splay |
|---|---|---|---|
| Harrow | 1.6 m | 1.0 m | 0.12 rad |
| Roustabout | 1.3 m | 1.2 m | 0.12 rad |
| Mule | 1.6 m | 1.5 m | 0.12 rad |

A strut on the chest, `PUSH + 0.6` m long, carries each shoulder out, so no arm hangs in the air.

## In the game

The game drives the legs with its own solver (`cb_core::mech::gait`) and leaves the upper arm at its rest, except during
a melee swing (`melee::arm_rotations`), which writes the upper arm's whole rotation while the arm is thrown forward. The
splay is a rest angle and returns when the swing ends. Nothing in `cb-app` or `cb-core` changes; the glbs do.

## Open questions

| Id | Question | Provisional |
|---|---|---|
| AC1 | Foreman keeps 0.22 m mid-stride; widen it too? | No: it does not touch |
