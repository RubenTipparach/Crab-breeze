## ADDED Requirements

### Requirement: Mixed enemy forces
The game SHALL field tanks, helicopters, technicals, rocket teams and infantry squads from data, each moving and firing by its own brain through the same projectile simulation as the mechs.

#### Scenario: A tank engages
- **WHEN** a tank is 800 m from the player's mech with a clear line
- **THEN** it drives in to its range, stops, turns its turret onto the mech and fires

### Requirement: Infantry can be crushed
Infantry and technicals under a mech's foot SHALL be destroyed.

#### Scenario: Stepping on a squad
- **WHEN** a mech's foot plants within a soldier's radius
- **THEN** that soldier is killed
