## ADDED Requirements

### Requirement: Three waves on three cities
The defence mission SHALL send three waves of 10 raiders, the first on the city the pilot starts in and the next two on the next largest cities, each wave appearing after the last is destroyed and a lull.

#### Scenario: A wave is cleared
- **WHEN** the last raider of wave 1 is destroyed
- **THEN** after the lull wave 2's 10 raiders appear outside city 1 and its clock starts

#### Scenario: The last wave is cleared
- **WHEN** the last raider of wave 3 is destroyed and no city has passed half damage
- **THEN** the mission is won

### Requirement: The mission is lost by the city, the clock or the mech
The defence mission SHALL be lost when the attacked city's damage passes 50 %, when a wave is not cleared within 600 s of appearing, or when the player's mech is destroyed.

#### Scenario: The city falls
- **WHEN** wave 2 is still fighting and city 1's damage reaches 50.1 %
- **THEN** the mission is lost because the city fell

#### Scenario: Out of time
- **WHEN** 600 s have passed since wave 1 appeared and one raider is still alive
- **THEN** the mission is lost because time ran out

### Requirement: City damage is one number
A city's damage SHALL be the share of its buildings' cells destroyed since the start, and the cockpit readout SHALL show that same number.

#### Scenario: A tower falls
- **WHEN** a building of 400 cells collapses in a city of 4 000 cells and nothing else is broken
- **THEN** that city's damage is 10 %

### Requirement: Raiders shell the city unless they fight the pilot
A raider SHALL shell the nearest standing building of the attacked city, unless it is one of the three raiders nearest the pilot within 300 m, or it was hit in the last 8 s and the pilot is within 700 m, in which case it SHALL fight the pilot.

#### Scenario: A raider far from the pilot
- **WHEN** a raider is 900 m from the pilot and has not been hit
- **THEN** its goal is a facade point of the nearest standing building in the attacked city

#### Scenario: Five raiders near the pilot
- **WHEN** five raiders are within 300 m of the pilot
- **THEN** the three nearest fight the pilot and the other two keep shelling the city
