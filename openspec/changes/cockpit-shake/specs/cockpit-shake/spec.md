## ADDED Requirements

### Requirement: The seated view shakes from the mech's own body
While the pilot is seated, the view SHALL shake from the mech's landings, collisions, hits taken, its own guns, its melee
strikes and its jets lighting (kicks), rumble while its jets fire or its skates roll, and dip and spring back on a
landing or collision (jolt); the shake SHALL ride on the view only and never change where the player aims.

#### Scenario: A hard landing
- **WHEN** the mech lands from a jump at 10 m/s downward
- **THEN** the view kicks and the eye dips and springs back within about a second, and the aim is unchanged

#### Scenario: Footsteps
- **WHEN** the mech walks on level ground
- **THEN** the footfalls do not move the view

### Requirement: The player can turn the shake down
The settings SHALL hold a camera-shake strength from 0 to 1 that scales every shake; at 0 the view does not shake.

#### Scenario: Shake off
- **WHEN** the setting is 0 and the mech lands hard
- **THEN** the view does not move
