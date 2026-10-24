## ADDED Requirements

### Requirement: The world is built from a seed and data alone
The world generator SHALL build the whole world (terrain, cities, streets, lots, buildings and highways) from a seed and `data/world/world.ron`, with no other input.

#### Scenario: The same seed builds the same world
- **WHEN** the generator runs twice with seed 42 and the shipped data
- **THEN** both plans are identical, building for building

#### Scenario: Seed 42 is pinned
- **WHEN** the generator runs with seed 42 and the shipped data
- **THEN** the plan's fingerprint, city count and building count equal the golden values in the test

### Requirement: Cities at mech scale
Cities SHALL have streets at least 24 m wide and buildings from 2 storeys up to at least 40 storeys, at 3.5 m a storey.

#### Scenario: A mech fits down every street
- **WHEN** the shipped world is generated
- **THEN** every street is at least 24 m wide and no building footprint overlaps a street

#### Scenario: Buildings vary from low to high rise
- **WHEN** the shipped world is generated with seed 42
- **THEN** the lowest building has 2 storeys and the tallest at least 40

### Requirement: Open ground between cities
Cities SHALL be separated by open ground with no buildings, at least the configured spacing apart.

#### Scenario: Cities keep their distance
- **WHEN** the shipped world is generated
- **THEN** every pair of city squares is separated by open ground and their centres are at least 1100 m apart

### Requirement: One ground and one collision query
The terrain mesh, the pilot and the mech SHALL use the same ground height query, and the pilot and the mech SHALL use the same building collision query.

#### Scenario: The drawn ground is the walked ground
- **WHEN** `height_at` is evaluated at any grid sample
- **THEN** it equals the height the terrain mesh draws there

#### Scenario: Roads lie on the ground
- **WHEN** `height_at` is evaluated anywhere on a highway's surface
- **THEN** it equals the road's grade at that point
