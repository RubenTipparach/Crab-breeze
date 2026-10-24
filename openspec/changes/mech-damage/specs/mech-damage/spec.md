## ADDED Requirements

### Requirement: Parts are destroyed one by one
A mech SHALL be damaged per part, and a part whose structure reaches zero SHALL be cut off the rig while the rest of the mech keeps working.

#### Scenario: Losing an arm
- **WHEN** the left arm's structure reaches zero
- **THEN** the arm falls away, its weapons are gone, and the mech still walks and twists

### Requirement: A mech limps on one leg
A mech that has lost a leg SHALL keep moving on its good leg and its stump at under a quarter of its walking speed.

#### Scenario: Limping home
- **WHEN** a mech with one leg throttles forward
- **THEN** it moves forward with a hop-and-drag gait no faster than 25 % of its top walking speed
