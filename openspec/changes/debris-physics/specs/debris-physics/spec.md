## ADDED Requirements

### Requirement: Debris falls under physics
Building chunks, mech wrecks and felled trees SHALL fall, tumble and come to rest as rigid bodies on the terrain and on each other, within a budget of 600 moving bodies.

#### Scenario: A storey collapses
- **WHEN** a building's storeys above a destroyed ground floor fall
- **THEN** their chunks fall onto the ground and come to rest there

### Requirement: Debris is physical only near the pilot
A chunk SHALL become a rigid body only when it breaks off within the physics radius of the pilot's eye; farther, it SHALL play a dust and rubble puff instead, and a moving body that ends up beyond 1.2 times the radius SHALL be made static where it is. The cell simulation SHALL run at every distance.

#### Scenario: Raiders shell the far side of a city
- **WHEN** a building 1 km from the pilot loses chunks
- **THEN** no rigid body is made for them, a dust puff plays where they broke, and the building still breaks and falls by the cell simulation
