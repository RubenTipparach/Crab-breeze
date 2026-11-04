## ADDED Requirements

### Requirement: A city is a town round a highway

A generated city SHALL have a highway (through the city beside its downtown, or round it with a spur avenue in) with its
commerce along it, a downtown beside it, and house blocks on grids that drift a few degrees between neighbourhoods, fading
at a ragged edge into farmland, with no paved slab round it.

#### Scenario: Driving into a city
- **WHEN** the pilot follows the highway into a city
- **THEN** stores, car lots and gas stations line it before the downtown, and house blocks spread out on either side

#### Scenario: A bypass city
- **WHEN** a city's highway is a bypass
- **THEN** no highway crosses its downtown, and a spur avenue leads from the bypass to it

### Requirement: Housing grows with the downtown

A city with a large downtown SHALL be wider than one with a small downtown, with apartment blocks near its centre.

#### Scenario: The capital
- **WHEN** the capital has a large downtown
- **THEN** its house blocks reach farther out than a small city's, and apartment blocks stand within 700 m of the downtown

### Requirement: Water only where the island has it

A city SHALL have no river, and the sea and a harbour only where it stands on the coast.

#### Scenario: An inland city
- **WHEN** a city stands inland
- **THEN** no water runs through it and no quay cranes stand in it

### Requirement: One connected street network

Every street of a city SHALL join the rest of its network, and the network SHALL join the highway (through frontage roads
and interchanges, or through section roads to a bypass); no land inside a city's edge SHALL be left unbuilt between its
parts.

#### Scenario: Driving from a house to the highway
- **WHEN** a car leaves any house block
- **THEN** it can reach the highway on streets alone, without crossing a kerb or open ground
