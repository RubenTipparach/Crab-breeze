# Proposal: lot-access

## Why

Owner, 2026-10-27, on the houses stills: "driveways need to connect with roads for it to be realistic, same with
businesses they need parking lots that connect with road". The first build of `small-buildings` (design section 7)
stands each house in a field facing the city, its drive ending in grass a few metres out, and stands each business
4 m off the highway's shoulder with nowhere to park. Up close it reads as models set on a lawn, not a place people
drive to. This also answers survey S6 ("Streets next?"): the suburbs get streets.

## What changes

- **Suburb streets.** Each suburb row gets a residential street along its front, 8 m paved, levelled into the
  ground the way the highways are, and every street joins a city street or a highway through connecting streets.
  A house whose street can't be joined is not built.
- **Driveways.** Every house kit records where its drive meets the front of its lot (the concrete slab under the
  garage or carport, or a new gravel or concrete drive on kits that have none). The game paves a drive from there
  to the street's edge, with a curb cut. A drive steeper than 12 percent drops the lot.
- **Parking lots.** Every business in a town or at a roadside stop stands back from the highway far enough for a
  row of striped stalls and a two-way aisle in front of it (12.5 m), paved to the road with one curb cut, two on a
  frontage over 30 m. The gas station's apron joins the road at both ends. Town houses on the highway get a drive
  to it.
- **One paving mesh.** Streets, drives, parking and stripes draw as one mesh with the roads, in one draw, with
  the existing asphalt and the house concrete sets.

Not in this change: traffic on the suburb streets, parked cars in the business lots (survey S9), cul-de-sacs.

## Capabilities

- New: `lot-access` (streets, drives and parking that join every built lot to the road network).
- Touches: `small-buildings` (placement leaves room for streets and parking; kits record their drives).

## Impact

- `docs/mockups/small_buildings_kit.js`, the exporter and checker: a `drive` record per house kit, drives added
  to kits without one.
- `cb_core::world::small_buildings::lots`: streets placed with the rows, the business setback, lots dropped when
  they can't be reached.
- New `cb_core::world::paving`: the streets, drives and parking as data, and the ground levelled under them.
- `cb-app::world` `spawn_roads`: draws the paving into the roads mesh.
- `data/world/world.ron` `small_buildings:` gains the street, drive and parking keys.
