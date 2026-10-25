# Proposal: jet-cruise

## Why

Jobs (mission-jobs) are 3-25 km from a base, and the owner's flow is "Fly to a location and get in a
fight" and land at an air base after. The jets burn for 20 s and the mech covers about 1.5-2 km on a
burn: a ground machine that hops (survey M3, M5). A job needs a way to cross the island in the air
without making the mech an aircraft in a fight.

## What changes

- **Cruise:** a jet setting the pilot switches on above 150 m off the ground. In cruise the mech flies
  level and fast (about 180 m/s, 650 km/h) on a slow burn that heats the engines; the jets' normal
  budget does not drain.
- **Limits that keep it a ferry, not a dogfight mode:** turns are wide (6 degrees a second), the guns
  are locked, and taking a hit or dropping below 150 m ends cruise into normal jet flight.
- **Heat:** cruise heat rises over about 4 minutes (about 40 km); at the limit cruise ends and the
  engines need 60 s to cool before cruise is allowed again. The cockpit shows the heat the simulation
  uses (FLIGHT MFD).
- **Landing:** you come out of cruise near the site or the base and land on the jets as today.
