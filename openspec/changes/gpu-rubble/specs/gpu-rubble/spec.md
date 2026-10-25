## ADDED Requirements

### Requirement: Breaking buildings shed small GPU rubble
Every chunk that breaks off a building SHALL throw small 3D rubble pieces simulated and drawn on the GPU, their number growing with the chunk's volume up to a cap, falling under gravity and resting on the ground height where they were thrown until they fade.

#### Scenario: A storey falls far from the pilot
- **WHEN** a building 1 km away loses a storey
- **THEN** no rigid body is made, and small rubble pieces and dust are thrown where it broke
