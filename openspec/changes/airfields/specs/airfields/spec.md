## ADDED Requirements

### Requirement: Airfields stand on the island
The world generator SHALL place airfields with a level runway beside a city, joined to it by a highway.

#### Scenario: Seed 42 has airfields
- **WHEN** seed 42 is generated
- **THEN** it has at least one airfield, each runway level and dry

### Requirement: The Foreman is never lost
Destroying the Foreman SHALL return it to the roster; destroying any other owned mech SHALL remove it for good.

#### Scenario: A bought mech is destroyed
- **WHEN** an owned mech other than the Foreman is destroyed
- **THEN** it is no longer in the roster and must be bought again
