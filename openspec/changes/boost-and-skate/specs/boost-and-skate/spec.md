## ADDED Requirements

### Requirement: Rockets lift for 20 seconds
The mech SHALL lift on its rockets while the pilot holds boost, vertically only, for at most 20 s of burn from full, then lock out until recharged to 30 %.

#### Scenario: Running the rockets dry
- **WHEN** the pilot holds boost for 21 s from full
- **THEN** the rockets cut out after 20 s and the mech falls

#### Scenario: Rockets never push sideways
- **WHEN** the pilot holds boost and strafe right for 3 s with the skates off
- **THEN** the mech climbs and moves less than 1 m sideways

### Requirement: Skates drive all costly horizontal movement
The skate key SHALL toggle the skates on the ground and in the air. On the ground the skates SHALL drive along the legs' heading up to 120 m/s with an acceleration that fades toward top speed; in the air, with skates on, WASD SHALL push the mech relative to the view and burn the skate budget. Walking SHALL burn nothing.

#### Scenario: Building speed takes time
- **WHEN** the pilot skates at full throttle from a stand
- **THEN** the mech is between 40 and 70 m/s after 5 s and above 100 m/s after 15 s

#### Scenario: Strafing in the air on skates
- **WHEN** the pilot, airborne with skates on, holds strafe right for 3 s
- **THEN** the mech moves right more than 30 m and the skate budget, not the rockets', is used

### Requirement: Skates for 40 seconds
The mech SHALL skate when the pilot turns skates on, for at most 40 s of use from full, then drop back to walking.

#### Scenario: Skating dry
- **WHEN** the pilot skates for 41 s from full
- **THEN** the mech walks again after 40 s

### Requirement: Every mech has rockets and skates with stats of its own

Every roster mech SHALL have the rockets and the skates, run by the one simulation, with its lift, climb cap, burn
and recharge, and its skates' top speed, drive, grip, turn rate, use and recharge read from its own data file; no two
roster mechs SHALL share the same set (survey C15, owner, 2026-10-26: "every mech has dash/skate, just diferent stats
for them").

#### Scenario: The light Picket out-skates the hauling Mule
- **WHEN** the Picket and the Mule skate from a stand at full throttle for 10 s
- **THEN** the Picket is the faster

#### Scenario: A mech's skates last its own time
- **WHEN** the Mule skates from full until its skates give out
- **THEN** they give out after its own 55 s, not the Foreman's 40 s
