## ADDED Requirements

### Requirement: Every storey has a floorplan
Every storey that is not an industrial hall SHALL be divided into a corridor and rooms of types set by its use, with a stair or core that lines up on every storey.

#### Scenario: Every room can be reached
- **WHEN** any storey of the shipped block is planned with seed 42
- **THEN** every room has a door onto the corridor or into another room of the same flat

#### Scenario: The stairs line up
- **WHEN** a building with a core is planned
- **THEN** its core occupies the same cell on every storey

### Requirement: Furniture is placed by interior rules
Furniture SHALL be placed per room type, inside the room, clear of door zones and column footprints and of other items, and items taller than 1.2 m SHALL NOT stand against a facade wall.

#### Scenario: Doors stay clear
- **WHEN** the shipped block is furnished with seed 42
- **THEN** no item's footprint or clearance strip overlaps a door zone
