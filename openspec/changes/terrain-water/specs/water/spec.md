## ADDED Requirements

### Requirement: Rivers and lakes from the seed
The world generator SHALL carve rivers that run downhill and fill basins below sea level as lakes, deterministically from the seed, and SHALL keep every city square dry.

#### Scenario: A river runs downhill
- **WHEN** the shipped world is generated with seed 42
- **THEN** every river's water surface falls or stays level from source to mouth

### Requirement: One water query
The pilot, the mech, splashes and drawing SHALL read water through one query giving surface height, depth and flow at a point.

#### Scenario: Ground under water
- **WHEN** `water_at` reports water at a point
- **THEN** its surface is above `height_at` there by its depth

### Requirement: Mechs wade
A mech in water SHALL be slowed by drag in proportion to its submerged leg length and SHALL NOT walk into water deeper than its cockpit floor.

#### Scenario: Deep water slows the stride
- **WHEN** the mech walks at full throttle into water at half its hip height
- **THEN** its ground speed falls to the wading cap within 2 s
