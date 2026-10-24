## ADDED Requirements

### Requirement: The pilot sees the legs and the budgets
Seated, the pilot SHALL see the legs' heading against the torso and, for boosters and skates, the heat, the seconds left and the seconds to recharge, all read from the simulation.

#### Scenario: Torso twisted
- **WHEN** the torso is twisted 60 degrees left of the legs
- **THEN** the leg compass's arrow points 60 degrees right of straight up

### Requirement: The pilot can find their way
Seated, the pilot SHALL see the compass heading they look along and the distance and bearing to the nearest city.

#### Scenario: Looking north
- **WHEN** the pilot looks along -z
- **THEN** the heading reads 000 N
