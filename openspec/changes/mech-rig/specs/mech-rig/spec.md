## ADDED Requirements

### Requirement: One skeleton contract
The engine SHALL drive a mech only through the bone and socket names in `docs/design/mech-rig.md`, held in one list in `cb-core`, and SHALL drive a glb that follows the contract with no code change.

#### Scenario: A glb missing a bone is refused by name
- **WHEN** a mech's model has no bone named `hock_L`
- **THEN** binding fails with an error naming `hock_L` and the model, and the mech is not driven

#### Scenario: The stand-in follows the contract
- **WHEN** the shipped stand-in data is loaded
- **THEN** it defines every contract bone and socket with the contract's parent

### Requirement: Digitigrade walk
The mech SHALL walk on digitigrade legs whose feet alternate, stay planted in stance, and settle together at rest.

#### Scenario: A planted foot does not slide
- **WHEN** the mech walks at a steady speed
- **THEN** a foot in stance moves less than 5 % of the stride over the ground

#### Scenario: Feet settle at rest
- **WHEN** the mech comes to a stop
- **THEN** both feet are under the hips with no lift

### Requirement: Torso independent of the legs
The torso SHALL twist and pitch toward the pilot's aim within its limits, at its rate, independently of the direction the legs walk.

#### Scenario: Twist is limited
- **WHEN** the pilot aims 180 degrees to the left
- **THEN** the torso stops at its twist limit while the legs keep their heading

### Requirement: Movement modes reserved
The mech SHALL have walk, skate and jet modes, and SHALL keep walking when an unbuilt mode is requested.

#### Scenario: Skate is not built yet
- **WHEN** the pilot requests skating
- **THEN** the mech stays in walk mode
