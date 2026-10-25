## ADDED Requirements

### Requirement: Hits leave marks on mechs
Every hit that lands on a mech part SHALL leave a hit-mark decal on that part, sized by the shot's calibre, moving with the part, and a mech SHALL carry at most 48, the oldest going first.

#### Scenario: Autocannon on an arm
- **WHEN** a 35 mm round strikes an enemy's right arm
- **THEN** a puncture mark is placed on that arm where it struck, and it swings with the arm

### Requirement: Damaged mechs burn and smoke
A mech part that is lost SHALL burn at its stump and on the ground where it lands, a part under a quarter of its health SHALL smoke, and a mech under a quarter of its health SHALL burn from the torso.

#### Scenario: A leg is shot off
- **WHEN** an enemy loses its left leg
- **THEN** fire and smoke play at the hip the leg hung from, and the thrown leg burns for 30 s

### Requirement: Falling things kick up dust
A thrown limb, a wreck or a mech that comes down SHALL throw dust where it first touches the ground, once.

#### Scenario: A thrown arm lands
- **WHEN** a thrown arm first touches the ground
- **THEN** one `fall_dust` plays there, scaled by the arm's mass
