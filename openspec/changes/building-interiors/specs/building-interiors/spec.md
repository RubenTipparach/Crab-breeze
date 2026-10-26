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

### Requirement: Every building has a street entrance
Every building SHALL have a door in its ground-floor facade on the side that faces the street, or, for a lot with no street frontage, on the side facing the widest alley; the door SHALL open into the corridor or into a lobby whose corridor door lines up with it, and a route SHALL lead from outside the door to the stair core's landing with no furniture or column on it.

#### Scenario: The door faces the street
- **WHEN** the shipped world is generated with seed 42
- **THEN** every building has one entrance, on its street side, and the point a metre outside it can be reached on foot from the block's pavement

#### Scenario: Nothing stands in the way
- **WHEN** the ground storey of any shipped building is planned and furnished
- **THEN** no furniture box over the step height and no column footprint lies within the pilot's width of the entrance route

### Requirement: The stair climbs from the ground to the top storey
The stair core SHALL hold a stair from the ground floor to the top storey with treads no higher than 0.18 m and no shallower than 0.22 m, and every floor above the ground SHALL have the stair's flights cut out of it.

#### Scenario: A walkable stair
- **WHEN** any shipped building's stair is laid out
- **THEN** its rise is at most 0.18 m and its run at least 0.22 m, and it reaches the top storey

### Requirement: The pilot can walk into a building and climb it
While the pilot is on foot, every building near them, intact or breached, SHALL collide with the pilot through its standing facade, entrance, floors, partitions, stair and furniture, so that the pilot can walk in through the street door, or a breach, and up the stair; while the pilot is not on foot no building SHALL have these colliders.

#### Scenario: Walking in and up
- **WHEN** a pilot on the pavement walks the entrance route of a shipped building and then its stair
- **THEN** they pass the door, reach the stair landing, and stand on the top storey's floor

#### Scenario: Still solid elsewhere
- **WHEN** a pilot walks at a building's facade away from its door
- **THEN** the facade stops them

#### Scenario: Only on foot
- **WHEN** the pilot is seated in the mech, boarding or climbing out
- **THEN** no building near them has interior colliders, and once they stand on foot the buildings within 12 m have them before their first step

#### Scenario: A shut door is a wall
- **WHEN** a pilot walks at a shut street door and the door is not stepped
- **THEN** the leaf stops them outside

#### Scenario: A breach is a way in
- **WHEN** a facade bay of a building has been blown out
- **THEN** a pilot on foot walks in through it

### Requirement: The street door swings open as the pilot walks into it
Every building's street door SHALL be shut until a pilot on foot comes within the trigger distance of its leaf on either side, SHALL then swing away from the pilot (into the building from the street, out of it from inside), SHALL stay open while the pilot is in the doorway or the leaf's arc, and SHALL shut after a delay once the pilot has gone; a leaf SHALL never move into the pilot's body.

#### Scenario: In off the street
- **WHEN** a pilot walks the entrance route of a shipped building from the pavement with the door shut
- **THEN** the door swings into the building, the pilot passes through, and the door never begins to shut while they are in its trigger zone

#### Scenario: It shuts behind them
- **WHEN** the pilot has walked on past the door's trigger zone and the delay and the swing have run
- **THEN** the door is shut

#### Scenario: Out from the hall
- **WHEN** the pilot walks back out through the shut door from inside
- **THEN** it swings out of the building and the pilot reaches the street

#### Scenario: Never on the pilot
- **WHEN** the pilot stands in the doorway or in the leaf's arc
- **THEN** the door stays open, and a leaf made to shut stops short of their body
