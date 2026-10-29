# Proposal: seaport-layout

## Why

Owner, 2026-10-29, after the airport's second pass: "Aight good enough for now, we also need seaports". The harbours
change put a harbour by each coastal city, and the game draws it as one T-shaped pier on piles: a 30 m pier out to a
220 m head with three luffing cranes, three warehouses and an office on the apron
(`docs/screenshots/airport-layout/today-harbour.jpg`). That is a landing stage, not a port, the same "kinda simple"
the first airport was. The ships already sail in (`harbours`, built); they need somewhere a port would put them.

## What changes

A seaport laid out the way a 1980s port is, shown first as a 3D mockup at true size for approval (CLAUDE.md section 4),
then built in the game:

- **Shelter**: two rubble breakwaters enclosing the basin, a green and a red light on their heads, a buoyed approach
  channel with leading lights.
- **Container terminal**: a straight quay wall, three ship-to-shore gantry cranes on rails, a stacked container yard
  with straddle carriers, a container ship alongside.
- **General cargo**: a finger pier with the luffing cranes, transit sheds and rail along the quay, a freighter on
  each side.
- **Ferry terminal**: a linkspan, a passenger terminal and marshalling lanes, the ferry berthed stern-in.
- **Tanker jetty**: a trestle out to a T-head with loading arms and a pipe rack, the tank farm ashore in its bunds,
  a tanker alongside.
- **Shipyard**: a dry dock with a ship on its blocks, its gate and a goliath crane over it.
- **Fishing harbour**: an inner basin with floating pontoons, trawlers and launches, a fish market and a slipway.
- **Coast guard quay**: the gunboats' berth, a boathouse, and a mech slipway: a concrete ramp into the water where a
  mech wades in to skim (survey M4).
- **Port control**: the harbour office with its jobs terminal (as today), a port control tower, a pilot station and
  tugs, rail sidings, the port road, customs and a car park.
- **Every building breaks**, the airport's way: the small-buildings kit's support rule over each building's pieces
  (owner, 2026-10-29: "all buildings are destructible").

## Out of scope

Ship movement inside the port (berthing, tugs at work) beyond what `harbours` already does, mariners and dockers as
people (`civilians`), cargo handling as a job mechanic, a wake in the water (`water-ripples`).
