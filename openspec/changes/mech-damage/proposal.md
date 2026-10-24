# Proposal: mech-damage

## Why

Owner, 2026-10-24: "Mech parts destruction? A mech can still function if arms or head get blown
off. It could limp around if a leg is blown off." Damage that changes how the mech fights is the
heart of a mech sim: losing an arm loses its weapon, losing the head blinds the main cameras,
losing a leg turns the stride into a limp.

## What changes

- Parts on the rig contract's bones, each with armour and structure in data: head (the sensor
  head; it carries the all-view cameras), torso, each arm, each leg, cockpit. Hits are resolved per
  part (`cb-core::damage`, pure).
- When a part's structure reaches zero it is destroyed: its bones and their children are cut off
  the rig, spawned as a falling Avian wreck with sparks, smoke and fire, and the stump shows.
  - **Arm**: its weapons and its hand are gone; the torso still twists.
  - **Head**: the main sensor cameras are gone; the all-view screens fall back to the backup
    cameras on the torso: lower resolution, a narrower view, static (`mech-boarding` screen).
  - **Leg**: the mech falls to one knee, then limps: a hop-and-drag gait on the good leg and the
    stump, top speed under 25 %, a heavy lurch in the cockpit, jets can still lift it (M3).
  - **Torso or cockpit**: the mech is destroyed; the pilot ejects or dies (a later change).
- The cockpit shows it: damage lamps per part, the screens' camera source, alarms.
- A debug key deals damage to a named part until weapons exist.

## Out of scope

Weapons and ballistics themselves, repair, ejection, critical hits to internal systems (heat,
power), damage to buildings.
