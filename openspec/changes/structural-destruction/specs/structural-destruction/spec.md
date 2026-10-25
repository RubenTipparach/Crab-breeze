## ADDED Requirements

### Requirement: Buildings are structural graphs
Every piece of a building SHALL be an element that rests on the ground or on other elements, and an element with no standing support SHALL fall, grouped with the elements bonded to it.

#### Scenario: A storey with nothing under it falls
- **WHEN** every column and wall holding up part of a building is destroyed
- **THEN** the elements above that part fall together as one island and leave the graph

### Requirement: Load decides what gives
Columns and floors SHALL carry load down to the ground, and a member loaded past its capacity SHALL fail after a creak, so damage spreads by load and not by distance alone.

#### Scenario: One column is not enough
- **WHEN** a single ground-floor column of an intact building is destroyed
- **THEN** no other element fails

#### Scenario: A whole side brings its bays down
- **WHEN** every ground-floor column along one side of a building is destroyed
- **THEN** the floor bays above that side become cantilevers, are overloaded and fall within the creak time

### Requirement: Breaking costs no remesh
Breaking an element SHALL change only that building's element mask, never its meshes, and SHALL NOT add a draw call.

#### Scenario: A hole in a wall
- **WHEN** a wall panel is destroyed
- **THEN** the building's meshes and draw count are unchanged and the panel is no longer drawn
