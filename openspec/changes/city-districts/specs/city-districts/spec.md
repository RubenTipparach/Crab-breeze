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

### Requirement: A downtown takes one of three shapes

Each city's downtown SHALL be compact (round its centre), a main street (a band through the centre along the street axis
toward the highway that comes in nearest) or two cores (two round cores either side of the centre), drawn per city from
its own stream, the capital compact; its towers SHALL fall from its heart, where the lot nearest the heart stands the
downtown's full height, to its edge.

#### Scenario: The cities of one island
- **WHEN** the island is generated
- **THEN** its cities' downtowns take more than one shape, and every downtown block but the one nearest the heart lies within its shape

#### Scenario: The heart of a downtown
- **WHEN** a downtown's buildings are placed
- **THEN** the tallest stands on the lot nearest its heart

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

### Requirement: A highway meets its city at exits

A city's highway SHALL meet its arterials at interchanges with real ramps (a SPUI, diamonds, a single quadrant, a partial
cloverleaf, a cloverleaf, or a trumpet where a spur meets a bypass), and every other street that crosses it SHALL cross on
a bridge over it or under its viaduct, never at grade and never in a tunnel; no building SHALL stand inside an
interchange's footprint.

#### Scenario: Leaving the highway
- **WHEN** a car on the highway reaches an arterial
- **THEN** a ramp leads it off the highway onto the arterial, at the arterial's own height

#### Scenario: A street that has no exit
- **WHEN** a street with houses on both sides meets a highway with overpasses
- **THEN** it crosses over the highway on a bridge with an embankment each side and carries on into the next block

### Requirement: Every road can be driven

A city's road network SHALL be drivable end to end: every road reachable by car from the downtown, every ramp joined to
a road at both ends, every road that passes over another clearing it by 4.5 m, no grade over 8 %, no highway turn
tighter than 400 m in radius nor ramp turn tighter than 25 m, and no pier in a lane. A driving check SHALL prove it,
with a bot that drives trips through the network.

#### Scenario: The owner's ramp
- **WHEN** a ramp stops short of the street it leads to
- **THEN** the drive check fails and names the ramp end left in the air

#### Scenario: A bot trip
- **WHEN** the bot drives from a random street to the far end of the highway
- **THEN** it arrives, by streets, a ramp and the highway

### Requirement: A city's blocks are used the way a town's are

Every city block SHALL have a use: a downtown of towers at the city's middle, flats round it, the strip of businesses
and car parks where a highway reaches the city, parks, schools, and houses everywhere else. A house block SHALL hold
rows of houses facing its streets, each with a drive to the street; a strip block SHALL hold businesses, each with a
striped car park out to its street; no house or business SHALL stand on a street.

#### Scenario: The capital's house blocks
- **WHEN** the seed-42 island is built
- **THEN** its house blocks hold more than six houses each on average, its strips at least ten businesses, and every city lot stands inside a house or strip block, off every street, its drive or car park reaching a street (`a_citys_house_blocks_are_full_of_houses_and_its_strip_of_businesses`, `lots_keep_clear_of_water_roads_and_each_other`, `every_house_has_a_drive_to_a_street_or_a_highway`)

#### Scenario: Car parks, streets and highways look apart
- **WHEN** the city is drawn
- **THEN** its streets are on Asphalt027C, its highways on Concrete047A and its car parks on the worn asphalt set, paler than the streets (`Surface::Lot`)

