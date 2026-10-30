# Proposal: island-sites-layout

## Why

The owner, 2026-10-30: "We need to add 2 civilian airports. And 3 militaryn ases. ... Place civilians airports first.
Remove 2 military bases. ... Add ship ports. I want at least 2."

Today the island has one civil airport, one seaport and five military air bases (survey J5). Each site placer returns
one site and is placed after the bases, so an airport takes whatever room the bases left.

## What changes

- **Two civil airports, placed first**, before the military bases, each on its own city's ring, their runways and
  approach cones clear of each other and of every city.
- **Two seaports** (at least), placed after the airports on different coasts, each its own harbour.
- **Three military bases** (was five), placed last, in the room the civil sites left. This replaces survey J5's five.
- **Counts in data**: `sites: (airports: 2, seaports: 2)` and `bases: (count: 3)` in `data/world/world.ron`.
- **An access road** from every site's gate to the nearest highway, joined into the lane graph so road traffic and
  the player's car reach every site.
- **A mockup**: an island map page with every site, runway, approach cone and access road at true scale.

## Impact

- `cb_core::world::{generate, sites, bases, lanes}`, `data/world/world.ron`, `cb-app` site drawing (it loops over
  `plan.sites` already).
- Tests that hard-code five bases read the config instead.
- `airfield-traffic` and `ship-traffic` build on the sites placed here.
