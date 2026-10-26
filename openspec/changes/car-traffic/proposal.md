# Proposal: car-traffic

## Why

The owner (2026-10-24): "We need car traffic and the ability for the player to control cars and
drive around." The cities and highways are empty. Traffic makes them read as places people live, and
makes a 24 m mech feel 24 m tall. Driving (car-driving) is a separate change built on the same cars.

## What changes

- **Cars drive the roads:**
  - city streets and arterials at 40-60 km/h;
  - highways at 90-110 km/h;
  - they keep to their lane, follow the car ahead, stop at junctions and turn.
- **Traffic exists only near you.** Cars are spawned on lanes out of sight within 1 km of the pilot
  and removed beyond 1.2 km, so the whole island never has to be simulated.
- **They react to the fight:**
  - cars near shelling or a mech brake hard and stop;
  - a car under a mech's foot is crushed flat;
  - a car hit by a shot or a blast burns and its wreck stays put.
- **1980s civilian cars,** not real brands: sedans, station wagons, pickups, vans, taxis, buses and
  semi trucks. Each is modelled in Blender the CSG way with full PBR and LODs.
