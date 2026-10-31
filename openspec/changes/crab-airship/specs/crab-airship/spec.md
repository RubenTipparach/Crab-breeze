## ADDED Requirements

### Requirement: The fortress stays up on its lift thrusters
A crab air fortress SHALL hang on eight lift thrusters, its legs, and SHALL hold its height while any one of them is
working. When the last one is destroyed it SHALL open its six hull thrusters and hold on them, and it SHALL lose all its
thrust and fall once `hull_kills_to_fall` (3) of them are destroyed. It SHALL never sink or list part way.

#### Scenario: One leg left
- **WHEN** seven of its eight lift thrusters are destroyed
- **THEN** it holds its height and its last lift thruster runs past full throttle

#### Scenario: Every leg gone
- **WHEN** the last lift thruster is destroyed
- **THEN** its hull doors open, its hull thrusters light, and it holds its height throughout

#### Scenario: Half the hull thrusters gone
- **WHEN** three of its six hull thrusters are then destroyed, on either side
- **THEN** it stops launching and dropping, and falls level under gravity and drag alone until it hits

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

### Requirement: The fortress's crash is city damage
The buildings the fortress's wreck breaks SHALL count in the attacked city's damage, the share of its building cells destroyed, the same measure the city defence loses on. The fortress SHALL drift along its circle while its drive thrusters work and SHALL fall straight down from where it hangs once both are lost.

#### Scenario: Brought down over the city with its drives stopped
- **WHEN** both drive thrusters are destroyed, it comes to a stop over the city, and then it falls
- **THEN** it lands straight below where it hung and the city's damage rises by the cells its wreck broke

#### Scenario: Brought down with its drives running
- **WHEN** it falls with both drive thrusters working
- **THEN** it drifts on along its circle at 15 m/s until it lands

### Requirement: The fortress's deck guard
The fortress SHALL carry a deck guard of 2 Shrikes and 4 Mites standing on its tops from the start, and each working hangar lift SHALL bring a Mite up every 60 s while fewer than 6 stand and crabs are left below decks. A lost lift SHALL bring no more, none SHALL come up while it falls, and a guard SHALL keep to the tops it stands on.

#### Scenario: A lift is destroyed
- **WHEN** one of the two hangar lifts is destroyed
- **THEN** Mites come up only on the other one, every 60 s while the guard is short

#### Scenario: A guard walks to the edge
- **WHEN** a deck guard walks out over the edge of the deck
- **THEN** it stops at the edge as at a wall

### Requirement: The fortress appears over the nearest city's centre

When its mission starts, the fortress SHALL appear directly over the centre of the city nearest the pilot, at its
altitude over that city's ground, and SHALL fly out from there onto its circle round that centre.

#### Scenario: Starting the mission
- **WHEN** the fortress mission starts with the pilot nearest city 2
- **THEN** the fortress's first position is over city 2's centre, `altitude_m` over its ground
- **AND** it flies straight out to `circle_radius_m` and circles from there

### Requirement: Lit hull thrusters are targets

A hull thruster SHALL be boxed and lockable while it is lit, so a locked missile can climb into its port.

#### Scenario: Doors open
- **WHEN** the last lift thruster is destroyed and the hull thrusters light
- **THEN** every live hull thruster shows a box and can be selected and locked

### Requirement: A scripted pilot brings the fortress down

The playtest beat `BringDownFortress` SHALL fight the fortress as a player can, through the same input actions,
until it crashes: every lift thruster, then three hull thrusters with locked missiles. It SHALL fly on its jets to any
target more than 300 m off. After the crash, `ClearHostiles` SHALL shoot down the mechs, vehicles and aircraft left.

#### Scenario: The fortress-nuke playtest
- **WHEN** `--playtest fortress-nuke` runs in the Foreman
- **THEN** the fortress crashes and the run reports the win
- **AND** the pilot watches the mushroom cloud, then takes out the hostiles left

### Requirement: The fortress weaves on its circle

On its circle the fortress SHALL weave in and out by `weave_m`, `weave_per_turn` times a turn.

#### Scenario: A turn of its circle
- **WHEN** it has flown round its circle
- **THEN** its distance from the city's centre has ranged from `circle_radius_m - weave_m` to `circle_radius_m + weave_m`

### Requirement: Its crash goes nuclear

When the fortress hits the ground it SHALL dig a crater `crater_radius_m` in radius and `crater_depth_m` deep through
the city, bring down every building and house within `flatten_radius_m`, and raise a mushroom cloud whose cap climbs
to `cap_height_m`.

#### Scenario: The crash
- **WHEN** the fortress crashes over a city
- **THEN** the ground at the impact is `crater_depth_m` lower, nothing stands within `flatten_radius_m`, and a mushroom
  cloud rises over it

