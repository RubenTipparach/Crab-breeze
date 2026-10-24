# Proposal: crab-enemies

## Why

Owner, 2026-10-24: "enemy mechs should also be crabs". The raiders are tinted copies of the player's
stand-in. The invaders are alien crabs (`docs/design/alien-rig.md`): the Skitter (6 m, four legs,
twin SMGs and missile pods), the Snapper (heavy cannon and a sword) and Big Boy (a 30 m siege crab).
Their models are built (`assets/models/alien_*.glb`), but the game cannot drive a crab: the mech
code knows only the two-legged rig.

## What changes

- **Crabs in the game**: a crab is a body on its legs from data (`data/crabs/<name>.ron`: model,
  speeds, armour per part, weapons on its `MUZZLE_*` sockets), posed by one gait for every crab (the
  rig's promise: four joints a leg, whatever the shape), feet planted through the one ground query.
- **The crab gait** (`cb-core::crab`): alternating tripods for six legs and diagonal pairs for four,
  stride from speed, sideways scuttling (a crab's heading and its travel need not agree), body bob
  and sway; pure and tested.
- **Damage and hits**: hit capsules from the rig (body, each leg, each arm); lost legs slow it, the
  body destroyed kills it, a lost weapon arm loses its gun; lock on to Big Boy's `WEAK_*` points.
- **The AI pilot drives crabs too**: the same brain (`enemy-mechs`), with a crab's controls (it
  strafes at full speed, so its orbit runs sideways).
- **The defence waves become crabs**: Skitters and Snappers, with Big Boy as the last wave's set
  piece (survey G14).

## Out of scope

Crab boarding (no cockpit), crabs climbing buildings, crabs beyond the three built.
