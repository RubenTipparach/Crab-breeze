## ADDED Requirements

### Requirement: Grass is built on the GPU
Grass SHALL be drawn with no entity, transform or CPU instance data per blade: each blade SHALL be built in the vertex shader from its index, and the only per-vertex data SHALL be the blade's index, the vertex's place in the blade and the level of detail.

#### Scenario: A tile's mesh carries no blade positions
- **WHEN** the blade mesh of any level is built
- **THEN** its only vertex attribute is (blade index, vertex in blade, level), and its vertex and triangle counts are those in the design (16,384, 4,096 and 1,024 blades a 32 m tile; 9, 7 and 5 vertices; 7, 5 and 3 triangles a blade)

### Requirement: Grass is drawn in tiles with levels of detail
Grass SHALL be drawn on square tiles within `radius_m` of the eye, each tile at the level its nearest point's distance calls for, nearer levels drawing more, narrower blades with more segments, a tile changing level only past a hysteresis band, and all tiles of a level SHALL share one mesh and one material so they batch into one draw.

#### Scenario: The eye on open ground
- **WHEN** the eye stands on the ground
- **THEN** the tiles within 300 m are listed nearest first, the tile under the eye is drawn at the finest level, and a tile 28 m away that was drawn at the finest level stays there

#### Scenario: The eye high above
- **WHEN** the eye is 400 m above the ground
- **THEN** no grass tile is drawn

### Requirement: Grass stands on the one ground
A blade's root SHALL stand on the ground's height as the terrain's one height query gives it, read from the terrain's lattice by the same cell rule, with no terrain noise evaluated in a shader.

#### Scenario: Heights in a tile
- **WHEN** a tile is baked
- **THEN** its height at any point equals `Heightfield::height_at` there to a millimetre

### Requirement: Grass grows only where it should
Grass SHALL NOT grow on roads, bridges, buildings, a city's paved ground, the beach or in water, SHALL thin on steep ground, and SHALL be cleared where a crater changed the ground.

#### Scenario: A city, a highway, a river
- **WHEN** a tile over a city's centre, a point on a highway, and a point in a river are baked
- **THEN** the city tile is mostly bare, and the highway and the river have no grass

#### Scenario: A crater
- **WHEN** a blast digs a crater in a grassy tile
- **THEN** the tile has no grass in the bowl and keeps it 20 m away

### Requirement: Grass can be switched off for measurement
The environment variable `CB_NO_GRASS=1` SHALL leave grass out of the game entirely, so the benchmark can measure what it costs.

#### Scenario: A benchmark variant
- **WHEN** the game runs with `CB_NO_GRASS=1`
- **THEN** no grass plugin, material, buffer or tile entity exists
