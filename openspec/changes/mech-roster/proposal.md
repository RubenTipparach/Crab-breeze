# Proposal: mech-roster

## Why

The owner, 2026-10-24, on the Mechs page: "Stevedore sucks..all these other mechs 10/10 let's get them
integrated with the game".

The eight detailed mechs (Foreman, Ironhorse, Harrow, Roustabout, Driller, Mule, Picket, Sapper) exist only on the
mockup page. The game can fly one Blender mech today, the 12 m Breeze, and four things stand between it and the new
ones:

- **The rig contract.** None of the eight has the `LADDER_base` socket, and the Picket and the Sapper have no
  `MUZZLE_L`. The binder refuses a model missing any contract name, and a refused mech cannot be boarded.
- **The cockpit.** Each mech's interior (Panorama, Titan, Excavator cab, Canopy) is a separate glTF that the page hangs
  in the chest. The game loads no interior, so the pilot would sit in an empty bay.
- **Their size.** They stand 22 to 24 m. Every size the game reads is tuned for 12 m: stride, step height, hip
  height, wading depth, body radius, mass, the pilot's eye at 9.6 m for aiming and targeting, the hit capsules, the
  body column the look ray stops on, and the wreck's fall.
- **Boarding.** Each mech needs a boarding timeline (`data/mechs/<name>/boarding.ron`) naming its own chest pieces,
  and a sit that folds its legs.

## What changes

- **The Stevedore is retired** (off the build list and the Mechs page; its generator stays in git history).
- **The glbs meet the contract:** every one carries `LADDER_base` on the ground in front of its hatch, both muzzles,
  and its jets on the torso. A check opens every shipped mech glb and refuses a missing contract name or a wrong
  parent.
- **The interior goes into the game with the mech:** the mech's data names its interior glTF and which interior part
  rides which chest joint (the page's `rides`). The game hangs the interior on the `cockpit` bone, moves each riding
  part onto its chest joint so the two open as one body, and draws the cabin from the seat. The screen-wall meshes are
  left out while sealed, so the pilot sees the world through them, as the page's screens show it.
- **Sizes come from data, per mech:** a new `body` section holds the pilot's eye height, the hit capsules, the wreck's
  centre and the smash height, with defaults equal to today's numbers. Every place that hard-codes them for 12 m reads
  the piloted mech's own.
- **Each mech gets its data, generated from its glb:** `tools/mechs/gen_mech_data.py` measures the glb (hip, eye,
  height, width, the hatch) and reads its boarding tracks and cockpit rides from the glb's extras, and writes
  `data/mechs/<name>.ron` and `data/mechs/<name>/boarding.ron`. Nothing is typed twice.
- **Playable:** `./run.sh --mech mechs/foreman.ron` (and each of the others).

## Out of scope

- Enemy mechs stay the stand-in (`enemy-mechs`).
- The default mech stays the stand-in until a playtest on the owner's machine shows the new ones walking, boarding and
  fighting. Switching the default is one line and an open question.
- The chest's hydraulic rams do not aim at their tips in the game yet (the page's `aimRams`).
- Sound, damage decals and LODs for the new mechs: their own changes.
