## ADDED Requirements

### Requirement: Every combatant ships four levels of detail within budget
Every mech, crab mech and alien crab model SHALL carry LOD0 to LOD3 for each of its mesh parts, each level SHALL be
within its class's triangle budget, and the build SHALL refuse a model with a level over budget.

#### Scenario: A model over budget is refused
- **WHEN** a detailed mech's LOD1 comes out at 12,000 triangles against a budget of 10,000
- **THEN** the build fails naming the model, the level and both numbers

#### Scenario: The shipped models pass
- **WHEN** `cb-tools validate` runs on the shipped combatant glbs
- **THEN** each has all four levels and every level is within its budget

### Requirement: The level shown follows the size on screen
The game SHALL show each part's level by its distance to the camera in multiples of the model's height, crossfading
between levels, and SHALL keep the pilot's own mech at LOD0.

#### Scenario: A mech walks away
- **WHEN** an enemy 24 m tall moves from 100 m to 500 m from the camera
- **THEN** it shows LOD0, then LOD1 from 144 m, then LOD2 from 432 m, each change a dithered crossfade

#### Scenario: Two sizes, one rule
- **WHEN** a 6 m Skitter and a 30 m Big Boy stand at distances of the same multiple of their heights
- **THEN** both show the same level
