## ADDED Requirements

### Requirement: The island has two civil airports, two seaports and three military bases
World generation SHALL place `sites.airports` civil airports (2), then `sites.seaports` seaports (2), then
`bases.count` military bases (3), each keeping clear of every city, highway and site placed before it.

#### Scenario: The shipped seed fits every site
- **WHEN** the shipped world is generated
- **THEN** it holds two airports, two seaports and three bases, and no two footprints overlap

#### Scenario: Ports are on different coasts
- **WHEN** the seaports are placed
- **THEN** their shores' outward normals are at least `port_coast_apart_deg` apart

### Requirement: Runway approaches are clear
Each runway end SHALL have an approach cone 3,000 m long at 3 degrees that no other site, runway or building rises
into.

#### Scenario: Nothing in the cone
- **WHEN** the shipped world is generated
- **THEN** no building top, footprint or other runway lies inside any approach cone

### Requirement: Every site has a road
Every site SHALL have an access road from its gate to the highway network, in the lane graph.

#### Scenario: A car can drive to every site
- **WHEN** a route is asked from a city street to each site's gate
- **THEN** the lane graph returns one
