# Proposal: building-lod

## Why

Owner, 2026-10-24: "Lod for buildings?", then "Ok let's do it. I would prefer 250 m for furniture".
Once a building is damaged it draws every floor's rooms and furniture at any distance, in the main
pass, the prepass and four shadow cascades, though from the street nobody sees a desk 1 km away or
its shadow on the floor of a dark office.

## What changes

- A building's rooms and furniture (partitions, core walls, every furniture box) are their own
  mesh, apart from its frame and skin, for damaged and dressed buildings alike.
- That mesh is drawn only within `rooms_radius_m` (250 m) of the pilot's eye and hidden beyond a
  20 m margin (`rooms_margin_m`), so a building on the edge does not flicker.
- Rooms and furniture cast no shadows (`NotShadowCaster`); they still receive them.

## Out of scope

Far damaged buildings drawn as frame only (step 3), far boxes merged per block (step 4): later
changes, measured on their own.
