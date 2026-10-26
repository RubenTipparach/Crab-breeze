## ADDED Requirements

### Requirement: The fortress stays up on its lift thrusters
A crab air fortress SHALL hang on eight lift thrusters, four a side; each side SHALL carry half its weight and a thruster SHALL carry at most a quarter. When a side's outer thrusters cannot carry its half, the fortress SHALL open six hull thrusters, three a side, each carrying at most a sixth, and it SHALL fall only when a side cannot carry its half with both.

#### Scenario: Two lost on each side
- **WHEN** two lift thrusters are destroyed on each side
- **THEN** the fortress holds its height and every live thruster runs at full throttle

#### Scenario: Three lost on one side
- **WHEN** three lift thrusters on the same side are destroyed
- **THEN** the fortress sinks while it opens its hull thrusters, then holds its height again

#### Scenario: Two hull thrusters lost on that side
- **WHEN** two of that side's hull thrusters are then destroyed
- **THEN** the fortress stops launching and dropping, lists toward that side and sinks at no more than its terminal speed

### Requirement: The cockpit shows the fortress's lift from the lift rule
The cockpit's fortress readout SHALL show each side's lift and each thruster's throttle as computed by the lift rule the fortress flies by.

#### Scenario: A thruster is destroyed
- **WHEN** a lift thruster is destroyed
- **THEN** the readout's numbers for its side change in the same fixed step as the fortress's lift

### Requirement: The fortress launches fighters and drops crab mechs
A crab air fortress SHALL launch a fighter every 20 s from its live catapults and drop a pair of crab mechs every 45 s from its live bays, until its hangar or bays are empty, it is falling, or those parts are destroyed.

#### Scenario: A catapult is destroyed
- **WHEN** one of the two catapults is destroyed
- **THEN** fighters launch only from the other one

#### Scenario: Both bays are destroyed
- **WHEN** both drop bays are destroyed
- **THEN** no more crab mechs are dropped

### Requirement: The fortress carries what stands on it
The fortress's deck, shoulders and pylon walkways SHALL be supports that move with it, carrying a mech standing on them at the surface's velocity, and a mech SHALL slide off when the list passes its grip.

#### Scenario: Standing on the drifting deck
- **WHEN** a mech stands still on the deck while the fortress drifts at 15 m/s
- **THEN** the mech moves with the deck and stays where it stood on it

#### Scenario: Landing at the deck's speed
- **WHEN** a mech flying alongside at the speed of the surface below it comes down on the deck, a shoulder or a pylon walkway with its jets nearly spent
- **THEN** it stands where it came down, is carried with the fortress, and its jets recharge to full

#### Scenario: Reaching it on the jets
- **WHEN** any roster mech lifts at full from the attacked city's ground
- **THEN** it passes the height of the fortress's deck before its jets are spent

### Requirement: Thruster downwash pushes and burns
A mech inside a live lift thruster's downwash column SHALL be pushed down and damaged on the parts inside it.

#### Scenario: Flying under a thruster
- **WHEN** a mech hovers in the column under a live lift thruster
- **THEN** it is pushed down and its parts inside the column take damage each step

### Requirement: The fortress's parts are targets
Every part of the fortress (thrusters, turrets, catapults, bays, lifts, drive thrusters and bridge) SHALL be a lockable target hit through the one projectile and damage code, with its own armour and weak point.

#### Scenario: A thruster from outboard
- **WHEN** any round hits a lift thruster's outboard armour plate
- **THEN** it does not penetrate

#### Scenario: A hull port from the side
- **WHEN** a round reaches an open hull thruster's port hole more than 30 deg from the fortress's vertical
- **THEN** it meets the 300 mm hull and does no damage, while one from directly below does double damage

#### Scenario: The autocannon on a thruster's inboard side
- **WHEN** 35 mm autocannon rounds hit a lift thruster's exposed inboard side (60 mm)
- **THEN** they penetrate and do their damage, and the same rounds into its 40 mm nozzle do double damage

### Requirement: The fortress is called the Stratostacean
Everything the player reads about the crab air fortress (its mission mail, the HUD's mission line, the TARGET page and the mission log) SHALL name it the Stratostacean, from the one `name` in its data.

#### Scenario: Locking a thruster
- **WHEN** the pilot selects one of the fortress's lift thrusters as the target
- **THEN** the TARGET page names it under STRATOSTACEAN with its part name and id, and shows that part's health
