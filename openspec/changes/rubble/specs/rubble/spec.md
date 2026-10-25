## ADDED Requirements

### Requirement: Rubble is ground
Rubble from a collapse SHALL raise the ground height that the pilot, the mech and debris stand on, through the same height layer craters change.

#### Scenario: Climbing a pile
- **WHEN** a building's storeys have collapsed and landed
- **THEN** the ground height under the landing is higher by the deposited rubble, and the pilot walking there stands on it

### Requirement: Heaps keep their angle of repose
A rubble heap SHALL NOT stand steeper than its angle of repose between neighbouring height samples.

#### Scenario: A big deposit slumps
- **WHEN** a large volume of rubble lands on a small area
- **THEN** after the deposit no sample stands more than `repose_slope x spacing` above its neighbour

### Requirement: Rubble is deterministic
The amount and place of rubble SHALL be decided by the simulation core from the same hits, not by where physics debris comes to rest.

#### Scenario: The same demolition, the same heap
- **WHEN** the same demolition runs twice from seed 42
- **THEN** both heaps are identical, sample for sample
