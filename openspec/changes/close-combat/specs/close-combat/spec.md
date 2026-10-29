## ADDED Requirements

### Requirement: Close combat switches on in melee range and toggles on Tab

The mech SHALL enter close combat when its target comes within melee range, SHALL leave it when the
target leaves range unless the pilot switched it on by hand, and Tab SHALL toggle it, holding it off
until the target leaves range when it was on.

#### Scenario: An enemy walks into reach
- **WHEN** the selected enemy comes within 30 m
- **THEN** close combat is on

#### Scenario: An untargeted enemy walks into reach
- **WHEN** no enemy is targeted and an enemy comes within 30 m
- **THEN** close combat stays off

#### Scenario: Tab with nobody near
- **WHEN** no enemy is within melee range and the pilot presses Tab
- **THEN** close combat is on until Tab is pressed again

### Requirement: In close combat the arms strike

In close combat, each fire control SHALL start a strike with its arm instead of firing a gun, and a
strike SHALL damage what its sweep meets through the same damage rules as a shot.

#### Scenario: A punch staggers a mech
- **WHEN** the pilot's first strike lands on an enemy mech
- **THEN** the enemy mech staggers and stays standing

#### Scenario: A punch through a wall
- **WHEN** the pilot strikes with the right arm facing a building within reach
- **THEN** building cells at the fist break

### Requirement: Three blows break a mech's stance and it falls

Each strike that lands on a mech SHALL add its move's stance (1, or 1.5 for the overhead smash and the
shoulder charge) to that mech's stance damage. When the damage reaches the mech's `break_at` (3 by
default), the mech SHALL fall, lie down and get up (3.2 s in all), and SHALL not walk, turn or fire until
it has got up. Blows SHALL add nothing while it is down. A mech untouched for 3 s SHALL mend its stance
at 0.5 a second.

#### Scenario: The third punch
- **WHEN** three strikes land on an enemy mech within 3 s of each other
- **THEN** it falls on the third and does not move or fire until it has got up

#### Scenario: No stun lock
- **WHEN** a strike lands on a mech that is down
- **THEN** its stance damage does not change, and it stands with its stance whole

#### Scenario: Blows spread out
- **WHEN** strikes land on a mech 6 s apart
- **THEN** it does not fall
