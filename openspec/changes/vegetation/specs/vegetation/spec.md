## ADDED Requirements

### Requirement: Trees from the seed
The world SHALL scatter trees from the seed and rules in data, never on roads, in water or on city blocks other than parks.

#### Scenario: The same seed grows the same trees
- **WHEN** the world is generated twice with seed 42
- **THEN** the same trees stand in the same places with the same ids

### Requirement: Mechs knock trees down
A tree hit with an impulse over its break impulse SHALL snap at the base, fall under physics and stay fallen.

#### Scenario: Striding through a wood
- **WHEN** a walking mech's leg strikes a pine
- **THEN** the pine snaps, falls away from the leg and comes to rest on the ground
