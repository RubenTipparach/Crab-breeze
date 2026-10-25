## ADDED Requirements

### Requirement: Buildings are generated from their lot, seed and data
The generator SHALL build every building (storeys, facade, roof and props) from its lot, the world seed forked by the building's id and `data/world/buildings.ron`, with no other input.

#### Scenario: The same seed builds the same building
- **WHEN** a building is generated twice from the same lot, seed and data
- **THEN** both have the same elements, modules and props in the same order

### Requirement: Buildings stand on a 6 m structural grid
Columns SHALL stand on the nodes of a grid of cells about 6 m square, floors SHALL span one cell, and facade bays SHALL be half a structural bay, so partitions meet piers or mullions and never cross a window.

#### Scenario: A partition never cuts a window
- **WHEN** any shipped archetype is generated
- **THEN** no partition's line passes through a window opening

### Requirement: Roofs carry props clear of their edges
Roof props SHALL stand on roof cells of one level, at least 1.0 m clear of the roof edge and clear of each other by their radii.

#### Scenario: Props keep clear of the parapet
- **WHEN** the shipped block is generated with seed 42
- **THEN** every roof prop's footprint lies on roof cells of one storey and 1.0 m or more inside the roof edge
