## ADDED Requirements

### Requirement: Only a ram or a weapon damages a building
A mech moving into a building SHALL break it only when it was moving at or above its ram speed and the building stopped it; weapons break buildings through their blasts as before.

#### Scenario: Walking into a wall
- **WHEN** the placeholder mech walks into a building at its full 12 m/s walk
- **THEN** it is stopped and the building loses nothing

#### Scenario: Skating into a wall
- **WHEN** the mech skates into a building at 55 m/s and is stopped by it
- **THEN** a hole of radius about 8.8 m is broken where it struck, and it smashes on through at 70 % of its speed

### Requirement: The hole grows with the impact and is capped
The ram's hole SHALL have the ram radius at the ram speed, grow as the speed ratio to the two-thirds power, and never exceed the largest radius.

#### Scenario: Flat out
- **WHEN** the mech rams at 120 m/s
- **THEN** the hole's radius is the 9 m cap
