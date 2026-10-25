## ADDED Requirements

### Requirement: Terrain looks are chosen by biome and slope in data
The terrain SHALL be drawn with PBR layers chosen per pixel from the two strongest biomes at that point and the slope there, by a table in data, and the layer weights at every pixel SHALL sum to one.

#### Scenario: A cliff is rock
- **WHEN** the ground slope at a point is over 35 degrees
- **THEN** the cliff layer of the strongest biome carries the most weight there

#### Scenario: Canyon walls are banded
- **WHEN** two points on a canyon wall are at the same height
- **THEN** they take the same strata band
