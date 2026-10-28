## ADDED Requirements

### Requirement: Houses and small businesses are built from one kit
The game SHALL build every house and small business type from one kit of parts (walls with openings, roofs, porches, interiors, signs) driven by data, with each window a hole in the wall holding a frame, a sill and transparent glass.

#### Scenario: A window you can see through
- **WHEN** a ranch house is built
- **THEN** each of its windows is an opening in the wall with a frame and glass, and the room behind it holds furniture

### Requirement: Towns read by biome
Small buildings SHALL be chosen by the biome and zone they stand in, so a desert town is adobe and trailers, a jungle village is stilt houses, and a beach town is cottages and motels.

#### Scenario: A desert town
- **WHEN** a town is placed where the desert biome is strongest
- **THEN** most of its houses are Pueblo Revival houses or trailers

### Requirement: The suburbs have variants of each house
Each suburban type (ranch, colonial, bungalow, split-level) SHALL come in three variants that differ in plan, walls or roof, not only in paint.

#### Scenario: A street of ranch houses
- **WHEN** a suburb is placed with ranch houses
- **THEN** it draws from three different ranch plans

### Requirement: A pilot on foot walks into a house
A house SHALL be walkable on foot: its doors swing open as the pilot reaches them, its walls, windows and furniture stop the pilot, its floors and stairs carry them.

#### Scenario: Through the front door
- **WHEN** the pilot on foot walks at a ranch house's front door
- **THEN** the door swings open and the pilot ends inside, standing on the living room floor

### Requirement: Real furniture only for a pilot on foot
Furniture SHALL be drawn as modelled furniture for a pilot on foot within 60 m, and as the kit's blocks otherwise within 100 m; a pilot in the mech never loads the modelled furniture.

#### Scenario: Climbing into the mech
- **WHEN** the pilot boards the mech beside a furnished house
- **THEN** the house's furniture is drawn as blocks
