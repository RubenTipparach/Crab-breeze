# Mech arms clear of their legs, by pose

## Why

Owner, 2026-10-29, with a screenshot of the Harrow in the game: "mech poses need to be updated so their arms don't go
into their legs. Update mech artifact first so I can verify". Then, when the first attempt changed the models: "You
weren't supposed to modify the model. This is why I asked for a mockup".

Measured on the Crab Breeze Mechs page (`MOCKUP.clearance`, triangle against triangle), three of the eight detailed
mechs put an arm into a leg: the Harrow, the Roustabout and the Mule. The other five keep a gap.

## What changes

- The models do not change.
- The pose layer gains an arm hold (`armHold` in `tools/mockups/mech_pose.js`): per mech, the shoulders turned out from
  the body and, where needed, swung forward with the elbows bent back to keep a weapon pointing ahead. It is laid over
  every move.
- The Mechs page shows it, with the holds in `ARM_HOLD` (`tools/mockups/gen_mech_designs.py`) and the measuring
  instrument `MOCKUP.clearance`.
- Once the owner approves, the game holds the arms the same way, the numbers in each mech's data.
