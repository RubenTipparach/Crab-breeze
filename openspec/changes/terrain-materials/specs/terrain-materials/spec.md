## ADDED Requirements

### Requirement: Terrain looks are chosen by rules in data
The terrain SHALL be drawn with PBR layers whose weights come from rules in data (slope, height, water, city ground, scorch), and the weights at every vertex SHALL sum to one.

#### Scenario: A cliff is rock
- **WHEN** the ground slope at a vertex is over 35 degrees
- **THEN** the rock layer carries the most weight there
