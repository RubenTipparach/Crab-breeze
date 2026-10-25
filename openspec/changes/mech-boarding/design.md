# Design: mech-boarding

The reference is the Crab Breeze Mechs page (`docs/mockups/mech_designs.html`, built by
`tools/mockups/gen_mech_designs.py` from `assets/models/mech_<id>.glb` and the cockpit interiors
`assets/models/cockpit_<variant>/`). The owner, 2026-10-24: "Breeze me h sucks", "Follow design
philosophy of boss mechs from mining mike. The cockpit is part of the mech, inside of it with the armor
opening up at the chest". So the cockpit is not a pod bolted to the mech: it is an interior in a bay in
the chest, and the mech's own chest armour opens to board. The timeline below is each mech's own, written
by its generator into the glb's root extras (`boarding`), and the interior's inner tracks come from its
manifest (`assets/models/cockpit_<variant>.json`).

## 1. The bay, the interior and the screen wall

- Each chassis's torso is lofted round its interior (`tools/blender/mech_boarding.py` `pod()`, the outline
  measured off the interior's glTF by `tools/blender/cockpit_hull.py`; owner, 2026-10-24: "The geometry of
  the cockpit must match the interiors that we just designed", then "I would want a mech more mech shape").
  Inside, a dark lining 0.05 m off the interior in three bands (under the screens, the screen belt, the
  roof); outside, the larger of the chassis's own torso silhouette (broad shoulders behind the cut, a waist,
  a collar for the head) and the lining grown by 0.30 m of armour (0.36 m round the screen belt). So the
  prow of the chest is the interior's shape: the Foreman's chamfered cab, the Stevedore's octagon, the
  Ironhorse's hexagonal screen bank. The `cockpit` node sits on the tub floor under the eye, with the
  sockets `EYE_pilot`, `SEAT_pilot`, `HATCH_entry`; the interior (`build_cockpit.py --variant panorama | cab |
  titan | canopy | harness`) mounts at `cockpit`.
- What opens: on a chest built in mode `inner` (the Foreman, the Ironhorse), the armour in front of each of the
  interior's opening parts, cut round the part by a box (Exact booleans), riding that part's own stages; the part
  itself rides the last chest joint (`cockpit.rides` in the root extras: interior node to chest joint), so the
  chest and the cockpit open as one body (owner, 2026-10-24, on the Foreman: "it's cockpit has a problem when
  opening. It needs to follow the titan style or panorama style"). On the older modes (the Stevedore's doors),
  everything ahead of the cut plane (`z_cut`) from just under the lining's floor to just over its roof.
- Interiors, all seated but the harness: Panorama (the Foreman), the Excavator cab (the Ironhorse; owner, 2026-10-24:
  "Iron horse has no seat still"), Titan and Canopy (the new Roustabout and Harrow, block-outs first), harness
  (standing, the Stevedore, until it gets a seated one). Every screen is flat (the owner: curved screens are "just
  fish eye pov").
- The screens that look outside share the material `SCREEN_allview`. They show the world as the mech's sensor head
  (`SENSOR_head`) sees it, from the eye's direction, so the view is undistorted wherever the head turns. Scanlines, a
  phosphor grade, the HUD drawn ahead. The cube map round the eye is not how they are drawn: it is only for the
  cabin's light and reflections (owner, 2026-10-24: "Cube map is only for cockpit reflections";
  `crates/cb-app/src/screens_cube.rs`). Provisional: one wide render from the head for the front screens and one for
  each side, measured on the owner's hardware before it is accepted (CLAUDE.md 7).
- Power: the screens power up tile by tile over 2.5 s with a flicker, and down in 0.6 s.

## 2. Kneeling down

The legs bend (`sitPose`: thigh forward, knee down, the pelvis lowered, the torso bowed), 3.0 s in,
3.0 s out, eased. Steam from the leg pistons as they compress.

The mech kneels only as far as its legs physically can, as the mockup shows, and never folds flat
onto the ground (owner, 2026-10-24: "the mech nealt down way too far", "dont fold the legs all the way
down like that, how it was mocked up in the artifact made sense, it kneels as far as it 'physically
can'", and "CRITICAL, the mech knealt TOO FAR"). The game's sit pose is the mockup's, computed by
`tools/mechs/gen_mech_data.py` from each glb's leg segments with the mockup's own `bendLegs` at 1.25:

| Mech | Hips standing | Hips knelt | Hock back from straight down | Torso bow |
| --- | --- | --- | --- | --- |
| Foreman | 13.10 m | 8.54 m | 19.3 degrees | 22.9 degrees |
| Others | their hip | 0.63-0.76 of it | 19-22 degrees (two-segment legs: no hock turn) | 22.9 degrees |

The old pose (pelvis at 0.29 of the hip, hock laid flat at 90 degrees) sat the torso on the road.

## 3. The chest opens (opening; closing is the same in reverse)

Times are seconds after the chest starts (3.2 s into the sequence); every plate is a node with its
hinge at the node's origin, driven by its track; `RAM_<n>` rams re-aim at their tips every frame.

| Chassis | Plates and motions | Rams | Then |
| --- | --- | --- | --- |
| Foreman (excavator) | Mode `inner` on the Panorama: `CHEST_lower` pops 0.04 m (0.35-0.65 s), then `CHEST_lower_1` folds forward and down 155 degrees on its foot, so it hangs below the sill with its top flush with the floor (0.6-2.0 s), the cockpit's lower front and the floor's lip on it; `CHEST_upper` pops (0.35-0.7 s), then `CHEST_upper_1` swings up and over 158 degrees on its top front edge, its lowest edge 2.1 m over the floor (1.4-2.9 s), the cockpit's upper front, dash and the roof's lip on it | two on the upper plate | |
| Stevedore (dock crane) | `CHEST_lock_L1/L2/R1/R2`, the lock rods, turn out 86 degrees (0.0-0.7 s); `CHEST_door_L/R`, the prow split down the middle, swing out 92 degrees on hinges at the cut (1.1-2.6 s) | | |
| Ironhorse (locomotive) | `CHEST_dog_1..7`, the clamps round the seam, swing off 69 degrees (0.0-0.5 s); mode `inner` on the Excavator cab: `CHEST_ramp` pops 0.04 m (0.35-0.65 s), then `CHEST_ramp_1`, the smokebox front with the cab's front on it, folds forward and down 152 degrees, hanging below the sill with its top flush with the floor (0.75-2.75 s) | | |

Then the interior opens what is left of it (its manifest's tracks for parts no chest joint carries, from 80 % of
the chest's time). The Foreman and the Ironhorse carry no ladder (owner, 2026-10-24: "They shouldnt have a ...
ladder. They shouldnt have these pieces of geometry that block the pilot from exiting, that part shouldve opened
WITH the dooor"): everything in front of the doorway, the floor's and the roof's lips included, goes with the parts
that open, and from the seat nothing stands between the floor and 2.1 m (checked from the pilot's eye with the
page's `MOCKUP.pick`). The Stevedore keeps its chain ladder (`LADDER`, `LADDER_1..10`, 0.8 m a segment), which
drops from under the sill one segment every 0.12 s, as many as reach the ground. Vents are `VENT_<stage>_<n>` empties with `kind` (steam or mist), `stage`
(seam: pressure equalising as the chest starts; open: the hinges while plates move; sill: mist rolling
out once open), `rate` and `dir`. Mist is a heavier, slower particle that falls and spreads.

## 4. The pilot's path: the rope

Knelt only as far as the legs go, the Foreman's sill is about 13 m up, so the chest lowers a rope
(owner, 2026-10-24: "having a rope lowered, the camera then follows the rope up to the mech", "there
should be a rope dangling down for me the player to climb up"):
- **The rope** drops from a winch 0.4 m above the sill, 0.8 m out in front of it, once the chest is
  open, paying out at 9 m/s to the ground, and reels in at the same rate when the hatch closes. It
  hangs straight with a slow sway of under half a degree (`cb-app::rope`, 4 cm thick).
- **Its foot** (`MechAccess::ladder_base`) is on the ground 1.2 m out in front of the sill, at the
  mech's own ground height, so the climbing pilot is just outside the rope.
- **The climb:** Use within reach of the rope's foot carries the eyes to the foot, straight up the
  rope to the sill's height, onto the sill (`HATCH_entry`), and settles them at `EYE_pilot` facing
  out of the bay, at the climbing speed in the pilot data (7 m/s: about 2 s up the rope, as if
  winched). Climbing out runs the same path backwards and stands the pilot at the rope's foot. The sequence then
closes (section 3 in reverse, 6 s), and the screens power up. Disembarking: screens down, open,
the reverse path, the mech may stand when the pilot is clear (3 m).

## 5. Cost

A boarding step and the timeline a fixed step: microseconds. Particles: up to 1500 billboards
while it plays, one draw call. The all-view screen: section 1.

## Open questions

| Id | Question | Provisional answer |
| --- | --- | --- |
| C1 | How the pilot sees out | Decided by the owner, 2026-10-24: sealed, all-view screens |
| C6 | How the pilot reaches the chest | The mech sits; no ladder on the Foreman or the Ironhorse (owner, 2026-10-24). How the pilot climbs to a sill about 3 m up is open: hand holds on the folded flap are the provisional answer |
| C8 | Which cockpit first | Wall, in Foreman (survey C8, M6) |
| C7 | All-view screen: seams and tint, or as clear as glass | Visible tile seams and a light phosphor tint (survey C7) |
