# Proposal: small-buildings

## Why

Owner, 2026-10-24: "using your knowledge and research of cool buildings can you make houses and
small business buildings? Have it be added to the island session". The island (`island-biomes`)
has 24 cities of towers and offices, but nothing between them: no suburbs, roadside strips, beach
towns, desert outposts or jungle villages. Missions flown low over the island need small places
that read at a glance as somewhere people live and work, and that a 24 m mech can stride through
and break.

The buildings that exist were found wanting (owner, 2026-10-24: "No window transparency. No
elaborate textures windows, doors layouts / Or carried building materials like wood or bricks or
concrete / No varied"). These types answer that: every window is a framed hole with a sill and
glass you see a furnished room through, and every type has its own material (brick, clapboard,
board and batten, stucco, adobe, block, corrugated steel, stainless).

## What changes

Sixteen building types of the alternate-history 1980s (survey G1), each a recipe over one kit
of parts (the A-frame cabin is dropped; owner, 2026-10-26: "the one house I dont like is the one with the single
big triangle roof, all others look good"):
- **Houses (8):** ranch house, colonial, craftsman bungalow, split-level, Pueblo
  Revival house, stilt house, beach cottage, single-wide trailer.
- **Small businesses (8):** streamline diner, gas station, corner store, auto repair, motor court
  motel, strip mall (laundromat, video rental, pizzeria, salon), burger drive-in, Quonset
  workshop.

**The kit** (`docs/mockups/small_buildings_kit.js` now, then `cb-core::world::small_buildings` and
`cb-app`):
- walls with real openings: windows, picture windows, ribbons, doors, shop fronts, garage and
  roll-up doors, each with frame, sill, lintel, shutters or an air conditioner;
- five roof families (hip, gable either way, flat with parapet, shed and swept, barrel), porches
  with railings and steps;
- interiors with furniture by room;
- neon and painted signs;
- cars.

**Placement by biome** (`island-biomes`): the island session places them in towns and along roads:
- suburbs of ranch, colonial, split-level and bungalow round the temperate cities;
- adobe and trailers in the desert;
- stilt houses in jungle and swamp;
- beach cottages and motels on the coast;
- bungalows and split-levels in the montane forest, in place of the A-frame (provisional, survey S4);
- diners, gas stations, drive-ins and Quonsets on the highways between cities.

**Destruction** uses `building-destruction` (and `structural-destruction` if it lands). A house's
walls and roof break into the same debris as the city's.

## Presented by

The mockup `docs/mockups/small_buildings.html`: the sixteen on one street, walkable at the
pilot's eye and the mech's, by day, dusk and night, with a cutaway to the floor plans.

## Out of scope

City towers and offices (`building-generation`, `building-variety`), real Material Maker sets for
these materials (a task here, done in a cloud session), people and animals.
