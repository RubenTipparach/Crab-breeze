## ADDED Requirements

### Requirement: The fortress stays up on its lift thrusters
A crab air fortress SHALL hang on eight lift thrusters, four a side; each side SHALL carry half its weight, a thruster SHALL carry at most a quarter, and the fortress SHALL fall when either side has fewer than two live thrusters.

#### Scenario: Two lost on each side
- **WHEN** two lift thrusters are destroyed on each side
- **THEN** the fortress holds its height and every live thruster runs at full throttle

#### Scenario: Three lost on one side
- **WHEN** three lift thrusters on the same side are destroyed
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

### Requirement: Thruster downwash pushes and burns
A mech inside a live lift thruster's downwash column SHALL be pushed down and damaged on the parts inside it.

#### Scenario: Flying under a thruster
- **WHEN** a mech hovers in the column under a live lift thruster
- **THEN** it is pushed down and its parts inside the column take damage each step

### Requirement: The fortress's parts are targets
Every part of the fortress (thrusters, turrets, catapults, bays, lifts, drive thrusters and bridge) SHALL be a lockable target hit through the one projectile and damage code, with its own armour and weak point.

#### Scenario: The autocannon on a thruster's cowl
- **WHEN** 35 mm autocannon rounds hit a lift thruster's 140 mm cowl
- **THEN** they do not penetrate, and the same rounds into its 40 mm intake grille do double damage
