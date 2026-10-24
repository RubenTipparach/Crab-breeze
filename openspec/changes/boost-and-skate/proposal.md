# Proposal: boost-and-skate

## Why

Owner, 2026-10-24: "some rockets to jump and some skates to slide around ... boosters will let me
fly and move me very fast in any direction. boosters last for about 20 seconds, skates last for
about 40." Survey M3 (heat limits flight, fuel ignored for now), M4 (wheels in the feet, Armored
Core style) and M5 (a limited mode the pilot turns on) are decided.

## What changes

- **Boosters** replace the jet mode switch: hold Space and they fire. Space lifts; WASD thrusts
  in any direction relative to where the pilot looks (up to 45 m/s); let go and the mech falls and
  lands walking. 20 s of burn from full; recharge 8 s from empty after a 1.5 s pause; empty, they
  lock until 30 % has come back.
- **Skates**: Shift toggles them. The legs brace, the wheels drive: up to 30 m/s, quick turns, low
  side grip so the mech drifts through corners. 40 s of use; recharge 10 s after a 1 s pause; empty,
  the mech drops back to walking.
- One velocity for the mech in every mode, so walking, skating and flying carry momentum into each
  other.

## Out of scope

Fuel (M3: ignored for now), skating on water, wall-running.
