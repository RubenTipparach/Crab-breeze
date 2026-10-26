## ADDED Requirements

### Requirement: Cars drive the lanes and keep their gap

Traffic cars SHALL drive along the lane graph built from the streets and highways, SHALL keep at
least a 2 s gap to the car ahead, and SHALL choose turns from seeded randomness so a run repeats.

#### Scenario: A queue at a junction
- **WHEN** a car waits at a junction for a crossing car
- **THEN** the cars behind it stop with at least their following gap

### Requirement: Arterial junctions have traffic lights that cars obey

Where an arterial meets at least two other roads the junction SHALL have 1980s traffic lights whose two phases take
turns from the times in `data/world/traffic.ron`, never both open at once; a car SHALL stop at the line on red, go on
amber only when it cannot stop short of the line, and take its turn on green; every other junction SHALL keep taking
turns by arrival, and traffic SHALL never deadlock (survey W39, 2026-10-26: "Lights on the arterials").

#### Scenario: A red light on an arterial
- **WHEN** a car comes to a lit junction while its light is red
- **THEN** it stops at the line and crosses only after its light turns green

#### Scenario: Ten minutes of a lit town
- **WHEN** the town's traffic runs for ten minutes with its arterials lit
- **THEN** cars cross junctions every minute and no waiting car stands longer than three light cycles

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
