# Proposal: ship-traffic

## Why

The owner, 2026-10-29 and 30: "Make the boats sail in and out of ports", "Add ship ports. I want at least 2. And add
in ship traffic", "All traffic on all these sites will be fully dynamic."

Today the lane ships shuttle between points and never moor (`dwell_s` is ignored), and the seaport's 17 ships are
static placements that never move.

## What changes

- **Every ship at a seaport is in the traffic**: it arrives through the harbour mouth, berths, stays, and sails out.
  The static placements become the timetable's starting state.
- **Both seaports** (from `island-sites-layout`) are harbours with berths, a mouth and an anchorage.
- **Lane ships moor**: a lane slot runs port to port, honouring `dwell_s` at each end.
- **Port ground traffic**: gantry cranes travel their rails and yard trucks drive loops between the berths and the
  container stacks.

## Impact

- `cb_core::ships::{timetable, harbour}`, `data/world/ships.ron`, `cb-app::boats`, the seaport template's marks.
