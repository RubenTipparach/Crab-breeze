## ADDED Requirements

### Requirement: A harbour is a sheltered port
Every harbour SHALL have two breakwaters enclosing its basin with an entrance at least 150 m wide, a light on each
breakwater head, and a buoyed channel from the entrance out to its anchorage.

#### Scenario: Ships can get in
- **WHEN** the port of any harbour on seed 42 is laid out
- **THEN** its entrance is at least 150 m wide and at least 12 m deep, and the channel to it crosses no land

### Requirement: Each kind of ship has its berth
Every port SHALL have a container quay, a general cargo pier, a ferry berth with a linkspan, a tanker jetty, a fishing
harbour and a coast guard quay, each berth at least as long as the largest ship it takes and as deep as its draught.

#### Scenario: The tanker fits its jetty
- **WHEN** the port of any harbour on seed 42 is laid out
- **THEN** the tanker jetty's berth is at least 150 m long and 10 m deep

### Requirement: A port's buildings break
Every port building and crane SHALL be made of breakable pieces held by the small buildings' support rule, so a piece
that loses what holds it falls.

#### Scenario: A gantry falls
- **WHEN** the legs on both rails of a container gantry are destroyed
- **THEN** its girder and boom fall
