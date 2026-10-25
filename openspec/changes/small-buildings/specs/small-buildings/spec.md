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
