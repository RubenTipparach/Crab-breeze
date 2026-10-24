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

- Each chassis has a bay in its chest (`tools/blender/mech_boarding.py`): a lined cavity sized to its
  interior's envelope plus 0.1 m, the `cockpit` node on the tub floor under the eye, the sockets
  `EYE_pilot`, `SEAT_pilot`, `HATCH_entry`. The interior (`build_cockpit.py --variant wall | jockey |
  harness`) mounts at `cockpit`.
- Three interiors: wall (seated, twin grips, a flat screen wall; Foreman), jockey (leaning forward on a
  saddle, low wide screens; Ironhorse), harness (standing, forearm controls, a flat octagon of screens;
  Stevedore). Every screen is flat (the owner: curved screens are "just fish eye pov").
- The screens that look outside share the material `SCREEN_allview`. In the game it is drawn from a cube
  render of the mech's sensor head (`SENSOR_head`) sampled by direction from the eye, so the view is
  undistorted wherever the head turns. Scanlines, a phosphor grade, the HUD drawn ahead.
- Cost: six extra scene renders a frame at the cube size. That is the largest cost in this design
  and is measured on the owner's hardware before it is accepted (CLAUDE.md 7); fallbacks are a
  lower cube size, fewer faces (the rear faces at a third of the rate) or one wide render.
- Power: the screens power up tile by tile over 2.5 s with a flicker, and down in 0.6 s.

## 2. Sitting down

The legs fold (`sitPose`: thigh forward, knee down, the pelvis lowered, the torso bowed), 3.0 s in,
3.0 s out, eased. Steam from the leg pistons as they compress.

## 3. The chest opens (opening; closing is the same in reverse)

Times are seconds after the chest starts (3.2 s into the sequence); every plate is a node with its
hinge at the node's origin, driven by its track; `RAM_<n>` rams re-aim at their tips every frame.

| Chassis | Plates and motions | Rams | Then |
| --- | --- | --- | --- |
| Foreman (excavator cab) | `CHEST_grille`, the bar grille, lifts 100 degrees on its top hinge (0.5-1.9 s); `CHEST_shutter`, the armour shutter behind it, drops into a housing in the belly (1.6-3.2 s) | two on the grille | the interior's inner tracks, the ladder |
| Stevedore (container) | `CHEST_lock_L1/L2/R1/R2`, the lock rods, turn out 86 degrees (0.0-0.7 s); `CHEST_door_L/R` swing wide 112 degrees (1.1-2.6 s) | | |
| Ironhorse (smokebox) | `CHEST_dog_1..8`, the clamps round the door, swing off 69 degrees (0.0-0.5 s); `CHEST_door`, the round smokebox door, swings 115 degrees on its side hinge (0.8-2.4 s) | | |

Then the interior opens (its manifest's tracks, from 80 % of the chest's time) and the chain ladder
(`LADDER`, `LADDER_1..10`, 0.8 m a segment) drops from under the sill one segment every 0.12 s, as
many as reach the ground. Vents are `VENT_<stage>_<n>` empties with `kind` (steam or mist), `stage`
(seam: pressure equalising as the chest starts; open: the hinges while plates move; sill: mist rolling
out once open), `rate` and `dir`. Mist is a heavier, slower particle that falls and spreads.

## 4. The pilot's path

With the ladder down, Use within 3 m of the ladder's foot walks the pilot's eyes up the ladder,
onto the sill (`HATCH_entry`), through the opening, turns them to face out of the bay and settles
them at `EYE_pilot`: about 6 s, look kept free until the last second. The sequence then
closes (section 3 in reverse, 6 s), and the screens power up. Disembarking: screens down, open,
the reverse path, the mech may stand when the pilot is clear (3 m).

## 5. Cost

A boarding step and the timeline a fixed step: microseconds. Particles: up to 1500 billboards
while it plays, one draw call. The all-view screen: section 1.

## Open questions

| Id | Question | Provisional answer |
| --- | --- | --- |
| C1 | How the pilot sees out | Decided by the owner, 2026-10-24: sealed, all-view screens |
| C6 | How the pilot reaches the chest | A chain ladder that drops from under the sill (survey C6) |
| C8 | Which cockpit first | Wall, in Foreman (survey C8, M6) |
| C7 | All-view screen: seams and tint, or as clear as glass | Visible tile seams and a light phosphor tint (survey C7) |
