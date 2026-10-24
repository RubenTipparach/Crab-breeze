## ADDED Requirements

### Requirement: Every shipped mech glb follows the contract
Every glb named by a mech file in `data/mechs/` SHALL carry every contract bone and socket with the contract's parent, and the data check SHALL refuse one that does not, naming the file and the node.

#### Scenario: A glb missing a socket fails the data check
- **WHEN** a mech file names a glb with no `LADDER_base`
- **THEN** `cb-tools validate` fails naming that glb and `LADDER_base`

#### Scenario: The eight detailed mechs pass
- **WHEN** the data check runs on the shipped mechs
- **THEN** the Foreman, Ironhorse, Harrow, Roustabout, Driller, Mule, Picket and Sapper all pass

### Requirement: A mech's sizes are its own
The game SHALL read the pilot's eye height, the hit capsules, the wreck's centre and the smash height from the mech's data, and a mech generated from its glb (one that carries an interior) SHALL agree with that glb on the eye, hip and overall height within 0.05 m.

#### Scenario: Aiming from a 24 m mech
- **WHEN** the pilot sits in the Foreman and targets an enemy
- **THEN** the targeting ray starts at the Foreman's eye height from its data, not at 9.6 m

#### Scenario: Stale data is refused
- **WHEN** a mech is rebuilt taller and its data file is not regenerated
- **THEN** the data check fails naming the file and the height it measured

### Requirement: The cockpit interior flies with the mech
A mech whose data names an interior SHALL carry that interior in its chest, its opening parts SHALL open with the chest pieces that carry them, and seated with the hatch shut the pilot SHALL see the cabin and, through its screen wall, the world.

#### Scenario: The chest and the cockpit open as one
- **WHEN** the Foreman sits and opens to be boarded
- **THEN** each interior part named in its rides moves with its chest joint and none is left in the doorway

#### Scenario: The sealed view
- **WHEN** the pilot is seated in the Foreman with the hatch shut
- **THEN** the interior is drawn on the cockpit layer, and neither the screen wall nor the hull is drawn
