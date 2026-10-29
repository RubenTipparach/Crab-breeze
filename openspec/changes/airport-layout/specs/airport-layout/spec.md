## ADDED Requirements

### Requirement: An air base is laid out round its runway
Every air base SHALL have a parallel taxiway along its runway's whole length, joined to the runway by links at both
ends and the middle, each link with a holding position, and joined to the site by apron entries.

#### Scenario: The taxiway is clear of the runway
- **WHEN** the airfield of any base on seed 42 is laid out
- **THEN** its parallel taxiway lies in the flattened strip and keeps at least 50 m from the runway's edge

### Requirement: An air base's buildings each have a place
Every air base SHALL have a terminal with stands and parked aircraft, a control tower, a fire station, a fuel farm,
a car park, a helicopter stand, a windsock and a fence, each inside the airfield and none overlapping a hangar, a
pad, the tower or each other.

#### Scenario: Nothing overlaps
- **WHEN** the airfield of any base on seed 42 is laid out
- **THEN** no two building footprints overlap, and every stand is clear of every building

### Requirement: A runway is marked
A runway SHALL carry designators read from its heading, threshold bars, touchdown zone and aiming point marks, a
centre line and edge lines, with blast pad chevrons, approach lights and PAPI lights at each end.

#### Scenario: Designators read the heading
- **WHEN** a runway runs from west to east
- **THEN** its `a` end reads 09 and its `b` end reads 27
