# Mech arms clear of their legs

## Why

Owner, 2026-10-29, with a screenshot of the Harrow in the game: "mech poses need to be updated so their arms don't go
into their legs. Update mech artifact first so I can verify".

Measured on the Crab Breeze Mechs page (`MOCKUP.clearance`, triangle against triangle), three of the eight detailed
mechs put an arm into a leg, standing still as well as walking: the Harrow (gun pods into the thigh armour), the
Roustabout (fist and forearm) and the Mule (forearm). The other five keep a gap.

## What changes

- `tools/blender/build_mechs.py`: the Harrow, Roustabout and Mule hang their arms further out and higher, on a strut
  from the chest, and turn them a little out from the body (`*_PUSH_M`, `*_LIFT_M`, `*_SPLAY`, `walker_arm`'s
  `splay` and `mount_m`).
- The rebuilt glbs are shown on the Mechs page first, built into a staging folder (`CB_MECH_STAGE`), and replace the
  game's `assets/models/mech_*.glb` only once the owner approves.
- The Mechs page carries the measuring instrument, `MOCKUP.clearance(id)`.
