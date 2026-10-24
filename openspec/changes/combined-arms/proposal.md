# Proposal: combined-arms

## Why

Owner, 2026-10-24: "There should be all kind of enemies, tanks, helicopter, cars, rocket launchers,
infantry." A mech fight against three copies of the same mech is not a battlefield. Ace Combat (G4)
is set pieces against mixed forces; a 12 m mech should wade through tanks and infantry while
helicopters and rocket teams make it work for every step.

## What changes

- `cb-core::units`: ground and air units from data (`data/world/units.ron`), each with speed,
  armour, hit points, a hit size and a weapon:
  - **tank**: tracked, 12 m/s, 120 mm turret gun (shell 900 m/s), 200 mm front armour; stops to fire.
  - **helicopter**: 70-110 m up, circles at 300 m, rocket pods (salvos of 4 rockets, 220 m/s).
  - **technical**: a pickup with a heavy machine gun, 22 m/s, circles close and fast; no armour.
  - **rocket team**: two soldiers and a launcher tube; a slow rocket (160 m/s) every 6 s, from cover.
  - **infantry squad**: five soldiers with rifles; sparks off armour; crushed under a mech's foot.
- Each kind has its own small brain (approach to its range, keep moving or hold, fire when on
  target), seeded and deterministic, reading only what a soldier could see.
- Rockets are slow and visible (a smoke trail), so a pilot can see them coming and dodge.
- Units are hit by the same projectile code as mechs (their hit shapes are capsules), destroyed
  units leave burning wrecks; infantry and technicals are crushed by mech feet.

## Out of scope

Aircraft (jets), squad tactics, vehicles on road graphs, units entering buildings.
