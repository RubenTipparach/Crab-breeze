## ADDED Requirements

### Requirement: Cruise flies far and ends on heat, a hit or low height

The mech SHALL enter cruise only from jet flight at least 150 m above the ground, SHALL hold the guns
while cruising, and SHALL leave cruise when its heat reaches the limit, when it is hit, or when it
drops below 150 m.

#### Scenario: Hit in cruise
- **WHEN** a cruising mech takes a hit
- **THEN** it is in normal jet flight on the next step, keeping its speed
