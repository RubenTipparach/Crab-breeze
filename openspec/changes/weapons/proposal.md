# Proposal: weapons

## Why

Owner, 2026-10-24: "epic mech fights and carnage". Mechs need guns. Ace Combat (G4) asks for lock-on
missiles later; the first weapons are the arm guns already modelled on every chassis.

## What changes

- `cb-core::weapons`: weapons in the mech's data (one per arm at `MUZZLE_L/R`): an autocannon
  (fast, many rounds) and a heavy cannon (slow, big blast). Fire groups on mouse buttons; the
  scripted pilot and the AI write the same `fire` actions.
- Projectiles are simulated in the fixed step with gravity and no drag (ballistics), as a pure
  function; a hit is resolved against, in order: mech parts (capsules on the rig), buildings
  (`building-destruction`), trees, then the ground and water. Hits on mechs go through
  `mech-damage`; shells explode (`terrain-deformation`).
- Losing an arm loses its weapon (mech-damage). Weapons heat the mech later (G2); a reload time now.
- Tracers, muzzle flash and shell casings are effects.

## Out of scope

Missiles and lock-on (G4, their own change), ammunition limits, a weapon heat model.
