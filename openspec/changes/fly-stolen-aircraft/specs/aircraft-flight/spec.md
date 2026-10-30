## ADDED Requirements

### Requirement: The pilot can take and fly a grounded aircraft
The pilot SHALL be able to enter a stopped aircraft on the ground by its door, fly it with the same input actions the
scripted pilot writes, and leave it once stopped on the ground; getting in and out SHALL be first person.

#### Scenario: Into the seat
- **WHEN** the pilot presses Use at a stopped aircraft's door
- **THEN** the view is carried to its `EYE_pilot` and eases into the chase camera with no cut

### Requirement: Aircraft fly by a simulated model
A flown aircraft SHALL move by lift, drag, thrust and moments computed from its kind's data (fixed-wing) or by rotor
thrust, torque and ground effect (rotorcraft), and the flight HUD SHALL show the model's own values.

#### Scenario: Trim
- **WHEN** a plane flies level at its trim speed with no input
- **THEN** its lift is within 2 % of its weight

#### Scenario: Stall
- **WHEN** a plane's angle of attack passes `a_stall` with stall protection off
- **THEN** its lift coefficient falls and the stall warning shows

#### Scenario: Hover
- **WHEN** a helicopter has hover hold on and the controls centred for 30 s
- **THEN** it holds its height within 0.5 m

### Requirement: Cars and aircraft share one vehicle flow
Entering, operating and leaving SHALL be one implementation for every vehicle kind.

#### Scenario: The car is unchanged
- **WHEN** the car tests run after the change
- **THEN** they pass unchanged in what they check
