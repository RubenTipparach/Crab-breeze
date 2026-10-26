## ADDED Requirements

### Requirement: Ground forces arrive by air
Tanks and enemy mechs SHALL arrive by cargo plane and descend on parachutes; crab mechs SHALL arrive by parachute from a crab airship.

#### Scenario: A cargo drop
- **WHEN** a cargo plane crosses the attacked city at 400 m and releases a tank
- **THEN** the tank falls, its chute opens after 2 s, it descends at 8 m/s and lands, and only then drives

#### Scenario: Shot under the chute
- **WHEN** a dropped tank is destroyed while still descending
- **THEN** it never lands and never fires
