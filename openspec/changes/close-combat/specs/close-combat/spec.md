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

#### Scenario: A punch through a wall
- **WHEN** the pilot strikes with the right arm facing a building within reach
- **THEN** building cells at the fist break
