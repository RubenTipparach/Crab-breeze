# Proposal: military-base

## Why

The owner, 2026-10-30: "Military airfields still need a lot of the same infrastructure it's just have military
vehicles and facilities. So you could mock that up", then "Work on military base mockup. Vehicles: helicopter, cargo
plane, fighter jets. Facility: dealers choice, but we need same fidelity as civilian airport".

Today a base is a 700 x 500 m apron with hangar bays, a tower and three outlined pads, beside a 2 km runway, and no
aircraft; military units are box placeholders.

## What changes

- **A military airfield site template** at the civil airport's fidelity, built the airport's way (a site page on the
  small-buildings kit, exported to breakable kits; night lights as data), replacing the block-out base. It keeps the base's mech hangar bays and terminals.
- **Four aircraft** on the airliner pipeline (`aircraft_kit.py`: zones, rig, lamps, LODs, manifest), original 1980s
  industrial designs, no real type's name or markings:
  - `fighter`: a twin-engine, twin-tail air superiority fighter;
  - `military_cargo`: a high-wing four-turboprop transport with a rear ramp;
  - `military_helicopter`: a utility gunship with stub wings;
  - `civil_helicopter`: a light twin-engine helicopter for the civil airports.
- **A mockup page** `docs/mockups/military-base.html`: the base and the aircraft at true scale beside the pilot and
  a mech, day and night, gear and rotors moving, taxi paths drawn.

## Impact

- New `tools/blender/build_military_aircraft.py`, `tools/mockups/military_base_src.html`,
  `tools/mockups/airfield_kit.js` (the airport page's airfield functions, shared), `assets/models/sites/military_base/`, `assets/models/aircraft/{fighter,military_cargo,military_helicopter,
  civil_helicopter}.*`, the mockup generator and page. Game code is `airfield-traffic`.
