## ADDED Requirements

### Requirement: The cockpit shows the mech's body and its damage

A DAMAGE MFD page SHALL draw the mech's head, torso, arms and legs, each coloured by the structure it has left as the
damage simulation holds it, and mark a lost part.

#### Scenario: A hurt leg
- **WHEN** the left leg is down to 42 % of its structure
- **THEN** the DAMAGE page draws the left leg amber with 42 % in it, and its line reads `LEG L 42%`

#### Scenario: A lost arm
- **WHEN** the right arm is lost
- **THEN** the DAMAGE page draws the right arm crossed out

### Requirement: Six screens, six pages

When the pilot sits, the cockpit's six screens SHALL each show a different page.

#### Scenario: Sitting down
- **WHEN** the pilot is seated in any roster mech
- **THEN** the hung screens show MECH, FLIGHT and MISSION and the dash screens DAMAGE, TARGET and WEAPONS
