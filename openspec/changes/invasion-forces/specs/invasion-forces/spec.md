## ADDED Requirements

### Requirement: Ground forces arrive by air
Tanks and enemy mechs SHALL arrive by cargo plane and descend on parachutes; crab mechs SHALL arrive by parachute from a crab airship.

#### Scenario: A cargo drop
- **WHEN** a cargo plane crosses the attacked city at 400 m and releases a tank
- **THEN** the tank falls, its chute opens after 2 s, it descends at 8 m/s and lands, and only then drives

#### Scenario: Shot under the chute
- **WHEN** a dropped tank is destroyed while still descending
- **THEN** it never lands and never fires

### Requirement: The crab airship is a carrier
A crab airship SHALL launch fighters every 20 s and drop crab mechs every 45 s while it has its launch deck and bays, and SHALL fall when its four engines are destroyed.

#### Scenario: Its engines are destroyed
- **WHEN** the last of an airship's four engines is destroyed
- **THEN** it stops launching, loses height and falls
