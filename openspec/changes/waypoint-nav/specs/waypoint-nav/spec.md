## ADDED Requirements

### Requirement: The pause menu has a map of the island
The pause menu SHALL have a MAP page showing the whole island from above, with its water, roads, cities, air bases,
airport and seaport, the pilot's position and heading, and an accepted mission's waypoint, zoomed by the wheel and panned
by dragging.

#### Scenario: Opening the map at the start
- **WHEN** the player opens the pause menu at the free roam start on seed 42 and picks MAP
- **THEN** the map shows the island with the pilot's arrow in city 0, and the airport and the seaport labelled north of it

### Requirement: The player puts down waypoints on the map
A left click on the map SHALL put down a waypoint, numbered in order, up to the data's maximum; a right click on one SHALL
remove it, and CLEAR SHALL remove them all.

#### Scenario: A route to the seaport
- **WHEN** the player clicks on the seaport and then on the airport
- **THEN** waypoints 1 and 2 stand there, a line runs from the pilot through 1 to 2, and the list gives each distance

### Requirement: The active waypoint guides the pilot
The first waypoint not yet reached SHALL be marked in the world with its number and distance, with an arrow at the
screen's edge when it is off screen, and SHALL be taken off once the pilot comes within the reach radius, making the next
one active.

#### Scenario: Reaching a waypoint
- **WHEN** the pilot comes within 150 m of waypoint 1
- **THEN** waypoint 1 is taken off with a chime, and waypoint 2's marker shows with its distance
