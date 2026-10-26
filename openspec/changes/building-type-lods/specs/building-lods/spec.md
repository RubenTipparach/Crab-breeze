## ADDED Requirements

### Requirement: Every building type has its own far look
A building farther than the dressing radius SHALL be drawn with its own type's mesh and baked facade, in its own wall and glass colours, never a texture shared by every type.

#### Scenario: A brick walk-up beside a glass tower, 800 m off
- **WHEN** the pilot looks at a brick walk-up and a curtain-wall tower 800 m away
- **THEN** the walk-up shows brick with punched windows and a cornice, and the tower shows a glass curtain wall with mullions, each in the colours its near skin has

### Requirement: The near and far looks agree
A building's far look SHALL use the same type and colours as its dressed skin, so coming closer changes its detail but not its colour or window rhythm.

#### Scenario: Skating toward a city
- **WHEN** the pilot skates toward a city and buildings cross 1.5 km and 250 m
- **THEN** no building changes colour or window rhythm as its detail changes, and none flickers on a boundary

### Requirement: A building type is not done without its far look
Every building type SHALL have a baked far facade and a massing recipe; a type without them SHALL fail the build's tests.

#### Scenario: A new type without a bake
- **WHEN** a building type is added with no baked far facade
- **THEN** `every_building_type_has_its_far_look` fails and names the type

### Requirement: The far shape gives way to a plain box at 1.5 km
A building past the dressing radius and within 1.5 km SHALL show its massing (parapet, cornice, roof props) over walls in its type's far facade; beyond 1.5 km it SHALL be a plain box in that facade. A detail already showing SHALL be kept to 1.15 times its radius.

#### Scenario: Crossing 1.5 km
- **WHEN** a building showing its massing moves from 1.4 km to 1.6 km away
- **THEN** it keeps its massing, and drops to the plain box only past 1.725 km
