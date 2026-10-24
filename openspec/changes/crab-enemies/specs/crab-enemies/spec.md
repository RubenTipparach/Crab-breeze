## ADDED Requirements

### Requirement: Crabs walk on their own legs
A crab SHALL move on its legs with a gait that keeps at least half its legs planted and its planted feet still on the ground.

#### Scenario: A Skitter scuttles sideways
- **WHEN** a Skitter strafes left at full speed for 5 s
- **THEN** at every step at least two of its four feet are on the ground, and a planted foot does not move until it lifts

### Requirement: The defence raiders are crabs
The defence mission's waves SHALL be alien crabs from `data/crabs/`, not copies of the player's mech.

#### Scenario: Wave 1 appears
- **WHEN** wave 1 starts
- **THEN** its 10 raiders are Skitters
