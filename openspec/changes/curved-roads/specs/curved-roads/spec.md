## ADDED Requirements

### Requirement: Highways follow the land
A highway SHALL run along a routed centre line whose grade climbs no more than `highway_max_grade` between its level ends, and the ground under its paved width SHALL be its grade.

#### Scenario: The ground under a highway is its grade
- **WHEN** seed 42 is generated
- **THEN** every point on every highway's paved width, off bridges, is within 0.12 m of the road's grade

### Requirement: Cities stay flat beside roads
A city's square SHALL stay at its ground height wherever roads run near it.

#### Scenario: A city is flat at its ground height
- **WHEN** seed 42 is generated
- **THEN** every sampled point of every city's square is at that city's ground
