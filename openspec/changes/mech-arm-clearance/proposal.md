# Mech arms clear of their legs when kneeling

## Why

Owner, 2026-10-29, with a screenshot of the Harrow kneeling in the game: "mech poses need to be updated so their arms
don't go into their legs. Update mech artifact first so I can verify". The first two attempts were wrong and are
undone: one changed the models ("You weren't supposed to modify the model"), the next held the arms out in every pose
("I asked top change only the pose when kneeling, but you modified all poses! and you modified all mech models! ...
ONLY MODDIFYING THE KNEELING POSE").

Measured on the Crab Breeze Mechs page, triangle against triangle at 75% and 100% kneel: the Foreman, the Harrow, the
Roustabout and the Mule drive a forearm into a thigh as they kneel. The Driller (0.66 m), Ironhorse (0.35 m), Picket and
Sapper stay clear.

Then, 2026-10-29, of the Foreman, Ironhorse, Driller and Mule: "mechs arms arent attached, was this how it was before?".
It was: every mech's shoulder joint sits several metres off its torso's side with nothing between, in the models as they
were before this session.

## What changes

- Only the kneel. Standing, walking, turning, skating, the melee swing and every model stay as they are.
- The kneel gains a per mech arm offset (`KNEEL_ARMS` in `tools/mockups/mech_pose.js`), radians on top of the kneel's own
  arm pose and scaled by the kneel, so it is zero standing and full when sat.
- Once the owner approves, the game's kneel (`SitPose` in `cb_core::boarding`) carries the same three numbers, written per
  mech into `data/mechs/<id>/boarding.ron`.
- The shoulders gain an axle housing (`contract_arm`'s `axle` in `tools/blender/build_mechs.py`): a horizontal cylinder
  in the frame material from inside the torso out to the shoulder joint, on the Foreman, the Ironhorse and the six
  walkers. Nothing else in a model changes. It is built only with `CB_SHOULDER_AXLES=1` until the owner approves; the
  Mechs page shows it, patched into the page's model files by `tools/mockups/patch_mech_axles.py` so the page changes by
  the axles alone.
