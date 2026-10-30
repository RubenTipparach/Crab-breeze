## ADDED Requirements

### Requirement: Aircraft taxi between stands and runways
Every aircraft at an airport or base SHALL taxi on its site's taxi graph from its stand to a holding point and back,
and SHALL take off and land only on its site's runways, one aircraft on a runway at a time.

#### Scenario: A full turnaround
- **WHEN** an aircraft's schedule runs from its stand through a departure and its next arrival
- **THEN** it pushes back, taxis, holds, takes off, lands, vacates and taxis to a free stand

#### Scenario: One on the runway
- **WHEN** an arrival is inside 6 km on final
- **THEN** no departure lines up on that runway

### Requirement: Helicopters use pads
A helicopter SHALL take off and land vertically on a pad at its airport or a city helipad.

#### Scenario: Pad to pad
- **WHEN** a civil helicopter completes a trip
- **THEN** it lifted from a pad and settled on a pad

### Requirement: Bases patrol and scramble
Military bases SHALL fly fighter patrols, cargo runs and helicopter patrols from their runways and pads, and a base
the player attacks or robs SHALL launch fighters and a gunship against the player from its runway.

#### Scenario: Attacked base scrambles
- **WHEN** the player damages a building on a base
- **THEN** within 60 s two fighters and a gunship take off from that base and attack the player

### Requirement: Grounded aircraft can be stolen
Any aircraft standing at a stand or pad SHALL be offered to the pilot to take, and SHALL leave the traffic when taken.

#### Scenario: Taken from the stand
- **WHEN** the pilot takes an aircraft from a stand
- **THEN** it is no longer in the traffic and its stand is not given to another aircraft until it is free
