# Proposal: mech-ramming

## Why

Owner, 2026-10-24: "the mech has to hit a biulding with a weapon or ram into it at a significant
speed to actually caause damage".

Today a mech that walks into a wall faster than 2.5 m/s and is slowed by it (to under 60 % of its
speed) smashes a 4.5 m sphere out of the building (`crates/cb-app/src/mech.rs`, numbers inline).
The placeholder mech walks at up to 12 m/s, so simply walking into a building tears holes in it.

## What changes

- **Walking into a building does no damage**: the mech is stopped by it, as by any wall.
- **A ram does**: only at or above `ram.speed_m_s` (20 m/s, above the 12 m/s walk, so skating,
  about 1.7 s from a standstill, or jets), and only when the wall stopped it (its speed fell below
  60 % in one step). The hole grows with the impact: radius 4.5 m at the ram speed, scaled by
  (speed / ram speed)^(2/3) (as the energy's cube root over the threshold's), up to 9 m. The mech
  keeps 70 % of its speed and smashes through, as today.
- **Weapons are unchanged**: shells, rockets and the cannon break buildings through their blasts.
- The rule moves into `cb-core` (`mech::ram`), its numbers into each mech's data (`ram:` in
  `data/mechs/<name>.ron`).

## Out of scope

Damage to the mech from ramming (the owner: ramming is free, survey W23), ramming other mechs.
