# Proposal: rockets

## Why

Owner, 2026-10-23: FX for "shots, explosions, missiles". There is nothing to draw a missile trail
on: every weapon is a gun. Lock-on missiles are Ace Combat's (G4) and need radar and a seeker, a
change of their own. A dumbfire rocket is the first step and the part a missile reuses: a
projectile with a motor, no drop, slow enough to see coming and dodge, leaving a smoke trail.

## What changes

- `WeaponSpec` gains `rocket: bool` (default false). A rocket flies straight at its launch speed
  (its motor holds it against gravity for its whole flight, up to `MAX_AGE_S`), so `fly` skips the
  gravity term and `lead` skips the drop term for it. Everything else (hit order, damage, blast)
  is the one projectile path.
- Enemies in data can carry a rocket pod (the defence waves do: `city-defence`).

## Out of scope

Seekers, lock-on, radar, flares, the player's own rockets (a cockpit weapon selector first).
