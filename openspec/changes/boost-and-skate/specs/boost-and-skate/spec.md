## ADDED Requirements

### Requirement: Boosters for 20 seconds
The mech SHALL fly on its boosters while the pilot holds boost, moving fast in the direction asked, for at most 20 s of burn from full, then lock out until recharged to 30 %.

#### Scenario: Running the boosters dry
- **WHEN** the pilot holds boost for 21 s from full
- **THEN** the boosters cut out after 20 s and the mech falls

### Requirement: Boosting flies where the pilot looks
While the pilot holds boost and a direction, the mech SHALL fly along the pilot's view, pitch included: level when the view is level, climbing when it looks up, diving when it looks down.

#### Scenario: A level dash
- **WHEN** the pilot holds boost and forward for 5 s looking level
- **THEN** the mech covers more than 150 m and stays within 10 m of the height it started the dash at

#### Scenario: Looking up
- **WHEN** the pilot holds boost and forward for 5 s looking 30 degrees up
- **THEN** the mech climbs more than 50 m

### Requirement: Skates for 40 seconds
The mech SHALL skate when the pilot turns skates on, for at most 40 s of use from full, then drop back to walking.

#### Scenario: Skating dry
- **WHEN** the pilot skates for 41 s from full
- **THEN** the mech walks again after 40 s
