## ADDED Requirements

### Requirement: The cockpit is sealed and sees through screens
The cockpit SHALL be a sealed pod whose inner wall above the consoles is one panoramic screen
showing the mech's camera view by direction from the pilot's eye, with the HUD drawn on it.

#### Scenario: Looking out from the seat
- **WHEN** the pilot is seated with the mech powered
- **THEN** the screen wall shows the world outside in every direction the pilot looks, and no
  window or open frame is in view

### Requirement: The mech sits down to be boarded
The mech SHALL lower itself to a sitting pose before its hatch opens, and SHALL only stand again
once the hatch is sealed or the pilot is at least 3 m clear.

#### Scenario: Opening for a pilot on foot
- **WHEN** a pilot on foot uses a stopped mech
- **THEN** the mech sits, then the hatch opens in the stages of its boarding timeline

### Requirement: The hatch opens in timed stages with steam and mist
The hatch SHALL open and close by the stages, nodes, angles and times of the mech's boarding
timeline in data, and each stage SHALL release the steam and mist bursts the timeline names.

#### Scenario: A stage plays from data
- **WHEN** the timeline reaches a stage's start time
- **THEN** that stage's nodes move to their open angles over its duration and its vents emit

### Requirement: Boarding and disembarking stay in first person
The pilot's eyes SHALL be the camera through the whole sequence: walking up the stair, stepping
in, sitting, the hatch sealing and the screens powering up, and the reverse when leaving.

#### Scenario: No cut
- **WHEN** a pilot boards or leaves a mech
- **THEN** every frame is drawn from the pilot's eyes and the path has no jump
