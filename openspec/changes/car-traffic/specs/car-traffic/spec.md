## ADDED Requirements

### Requirement: Cars drive the lanes and keep their gap

Traffic cars SHALL drive along the lane graph built from the streets and highways, SHALL keep at
least a 2 s gap to the car ahead, and SHALL choose turns from seeded randomness so a run repeats.

#### Scenario: A queue at a junction
- **WHEN** a car waits at a junction for a crossing car
- **THEN** the cars behind it stop with at least their following gap

### Requirement: A mech's foot crushes a car

A mech's foot strike whose foot covers a car SHALL crush that car.

#### Scenario: Stepping on a sedan
- **WHEN** the Foreman's left foot comes down on a sedan
- **THEN** the sedan is crushed and stays as a wreck

### Requirement: Traffic exists only near the pilot

Cars SHALL be spawned only within 1.5 km of the pilot and out of view, and dropped beyond 2 km.

#### Scenario: Flying away
- **WHEN** the pilot cruises 3 km from a city
- **THEN** none of that city's cars remain
