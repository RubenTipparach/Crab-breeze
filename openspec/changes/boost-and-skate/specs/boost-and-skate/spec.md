## ADDED Requirements

### Requirement: Boosters for 20 seconds
The mech SHALL fly on its boosters while the pilot holds boost, moving fast in the direction asked, for at most 20 s of burn from full, then lock out until recharged to 30 %.

#### Scenario: Running the boosters dry
- **WHEN** the pilot holds boost for 21 s from full
- **THEN** the boosters cut out after 20 s and the mech falls

### Requirement: Skates for 40 seconds
The mech SHALL skate when the pilot turns skates on, for at most 40 s of use from full, then drop back to walking.

#### Scenario: Skating dry
- **WHEN** the pilot skates for 41 s from full
- **THEN** the mech walks again after 40 s
