## ADDED Requirements

### Requirement: Trees are not entities
Trees SHALL be drawn from GPU instance buffers culled by a compute pass, with no ECS entity per tree, lit, fogged and casting shadows like any mesh.

#### Scenario: A wood in view
- **WHEN** 50,000 trees are within the draw radius
- **THEN** the entity count does not grow with them, and they draw in one indirect draw per species per view

### Requirement: A felled tree updates only its own row
Felling or a falling tree SHALL change only its own instance row on the GPU.

#### Scenario: A mech strides through a wood
- **WHEN** a mech snaps 12 trees in one step
- **THEN** those 12 rows, and no others, are written to the instance buffer that frame
