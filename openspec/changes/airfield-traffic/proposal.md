# Proposal: airfield-traffic

## Why

The owner, 2026-10-30: "Civilian airports should have some helicopters too. The player can steal those and planes as
well. ... All traffic on all these sites will be fully dynamic." And on the bases (plan questions, 2026-10-30):
"Patrol, scramble if hostile".

Today no aircraft taxis: a departure starts already rolling on the runway, a landing parks on the runway and vanishes,
routes run only between military bases, helicopters are orbiting points, and the airport's 21 aircraft are fixed
scenery. Military units appear in mid-air during defence waves.

## What changes

- **Taxi networks** at every airport and base, from the site templates' stands, taxiways, holding points and runway
  ends.
- **A full turnaround**: push back, taxi, hold short, line up, take off, climb, route, approach, land, roll out, vacate,
  taxi to a free stand, turn round, go again. The parked aircraft become the traffic's starting state.
- **Civil routes** between the two civil airports and off-map (arrivals and departures over the sea).
- **Helicopters at the civil airports**: pads, vertical take-off and landing, routes to city helipads and orbits.
- **Ground support vehicles**: tugs, fuel trucks and baggage trains following the taxi network.
- **Military traffic at the three bases**: fighter patrol circuits, cargo runs between bases, helicopter patrols, all
  from and to their runways and pads.
- **Scramble**: a base attacked, or robbed of an aircraft, turns hostile and launches two fighters and a gunship
  against the player from its alert apron.
- **Every aircraft on the ground can be stolen** (the flying is `fly-stolen-aircraft`).

## Impact

- `cb_core::air_traffic` (phases, a taxi graph, stands, pads), `cb_core::units` (launched from bases),
  `data/world/air_traffic.ron`, `data/world/units.ron`, site templates' marks, `cb-app::{air_traffic, airliners,
  units}`.
- Needs the models from `military-base` for the military kinds and the civil helicopter.
