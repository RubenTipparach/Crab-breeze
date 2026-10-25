## ADDED Requirements

### Requirement: Clouds are lit blobs, one mesh per tile
Each cloud puff SHALL be drawn as a lumpy blob merged with the other puffs of its 2 km tile into one mesh, lit by the scene's sun and ambient light, with a flat base at the cloud base.

#### Scenario: A tile of puffs
- **WHEN** a cloud tile's mesh is built from its puffs
- **THEN** it holds every puff and no vertex lies below the cloud base

### Requirement: Cloud cost can be measured
The clouds SHALL be switchable off with `CB_NO_CLOUDS=1` so the benchmark can measure their cost against the same take.

#### Scenario: Clouds off
- **WHEN** the game starts with `CB_NO_CLOUDS=1`
- **THEN** no cloud tile is drawn
