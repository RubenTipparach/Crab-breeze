## ADDED Requirements

### Requirement: Rockets and jets burn with a plume
A rocket in flight and every firing jet nozzle SHALL draw an exhaust plume, and a jet's plume SHALL be longer the harder it fires.

#### Scenario: A rocket in flight
- **WHEN** a rocket is in flight
- **THEN** a 4 m plume is drawn behind it, pointing back along its flight

#### Scenario: A jet fires
- **WHEN** a mech's jets fire
- **THEN** a plume is drawn under each nozzle, and it is gone when they stop
