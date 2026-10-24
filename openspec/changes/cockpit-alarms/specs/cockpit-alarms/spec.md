## ADDED Requirements

### Requirement: The cabin lights warn of what the simulation knows
The sealed cockpit SHALL carry alarm lamps at its interior's `LIGHT_*` points, driven only by the player's mech's
simulated state: a lost part, booster or skate heat, a missile locked on the mech, and hits on the chest.

#### Scenario: A limb is shot off
- **WHEN** the left arm's armour reaches zero
- **THEN** the left sill lamp strobes red for 3 s and then holds dim red while the arm is gone

#### Scenario: The boosters overheat
- **WHEN** the boosters' heat passes 0.6
- **THEN** the sill lamps glow orange in proportion, and pulse while the boosters are locked out

#### Scenario: A missile locks on
- **WHEN** an enemy missile in flight has the player's mech as its locked target
- **THEN** the overhead lamp flashes amber, faster as the missile's time to impact falls, until it hits or is lost

#### Scenario: A round hits the chest
- **WHEN** a hit lands on the torso
- **THEN** sparks burst from the console on the hit's side and the cabin lamps flicker for 0.2 s

### Requirement: Alarm priority
When several alarms want the same lamp, the lamp SHALL show the missile lock first, then a lost part, then heat, with
the hit flicker laid over whichever shows.

#### Scenario: Overheated with a missile inbound
- **WHEN** the skates are locked out hot and a missile is locked on
- **THEN** the overhead lamp flashes amber and the sills keep their heat glow
