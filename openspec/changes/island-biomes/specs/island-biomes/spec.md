## ADDED Requirements

### Requirement: The island has a mountain range and distinct biomes
The island SHALL have a mountain range through its middle with peaks above the cloud base, a desert in the range's rain shadow with canyons and mesas, jungle and swamps on the wet side, beaches, sea cliffs and an archipelago, all generated from the seed and data alone.

#### Scenario: Seed 42
- **WHEN** the shipped island is generated with seed 42
- **THEN** its highest ground is above the cloud base, and every biome (alpine, montane forest, jungle, swamp, temperate, savanna, desert, canyon, beach, cliff) holds at least one point

### Requirement: Rivers run through lakes
Every river SHALL start at a tarn on the range, and each long river SHALL pass through a valley lake on its flattest lowland stretch; a lake's water SHALL sit inside a basin, and no highway or city SHALL stand in a lake.

#### Scenario: Seed 42
- **WHEN** the shipped island is generated
- **THEN** every river's source lies in a tarn, at least one valley lake exists, and no highway point lies inside a lake
