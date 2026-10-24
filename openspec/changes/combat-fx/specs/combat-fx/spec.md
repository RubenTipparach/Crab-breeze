## ADDED Requirements

### Requirement: Effects are recipes in data
Every combat and world effect SHALL be a named recipe in `data/fx/fx.ron`, and the game SHALL stop at startup when a recipe it asks for is missing or a recipe has an unknown key or a non-finite number.

#### Scenario: A recipe is missing
- **WHEN** `fx.ron` lacks `explosion_ground`
- **THEN** startup stops with the file's path and the missing recipe's name

#### Scenario: A blast scales its recipe
- **WHEN** a 6 MJ explosion asks for `explosion_ground`
- **THEN** its fireball radius is the recipe's radius times the cube root of 6

### Requirement: One tracer per shot
Each shot in flight SHALL draw as one streak along its velocity, and the streak SHALL be gone the frame its shot stops.

#### Scenario: A shell hits a wall
- **WHEN** a 120 mm shell stops at a building
- **THEN** its tracer is despawned in the same frame and an `impact_building` effect plays where it stopped

### Requirement: Impacts look like what was hit
An impact SHALL play the recipe for the surface the simulation says the shot struck: a mech, a building, the ground or water.

#### Scenario: Autocannon on armour
- **WHEN** a 35 mm round strikes a mech part
- **THEN** the `impact_mech` recipe plays (sparks and a flash), not dust

### Requirement: Effects are capped
Live particles SHALL stay at or below 5000 and effect point lights at or below 24; an effect that does not fit SHALL be cut, never queued.

#### Scenario: A barrage
- **WHEN** 40 shells explode in one second
- **THEN** no more than 5000 particles and 24 effect lights are alive in any frame
