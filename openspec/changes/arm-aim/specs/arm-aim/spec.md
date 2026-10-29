## ADDED Requirements

### Requirement: Each gun arm aims its barrel at the aim point

Each frame, a mech's gun arms SHALL turn their shoulder and elbow so the barrel points at the aim point, within
each joint's limits and no faster than its slew rate. The shot SHALL still fly from the muzzle to the aim point.

#### Scenario: A target in reach
- **WHEN** the aim point is within the arms' reach and the arm has had time to slew
- **THEN** the barrel points within 0.5 degrees of the line from its muzzle to the aim point

#### Scenario: A close target
- **WHEN** the aim point is 22 m ahead
- **THEN** both guns angle in toward it, each within 0.5 degrees

#### Scenario: Past the shoulder's reach
- **WHEN** the aim point is beyond the joint limits
- **THEN** the arm stops at its limits and the shot still flies from the muzzle to the aim point

### Requirement: A striking arm lets go of its aim

While an arm strikes, its aim SHALL ease out, and it SHALL ease back in after the strike. A gun's recoil SHALL
play over the aimed pose.

#### Scenario: A jab
- **WHEN** the right arm jabs
- **THEN** the right arm follows the strike's pose and the left arm keeps aiming

### Requirement: A launcher arm is not aimed

An arm whose muzzle points more than 60 degrees off the mech's forward at rest SHALL keep its pose.

#### Scenario: The Sapper's missile pod
- **WHEN** the Sapper aims
- **THEN** its right gun aims and its left forearm pod keeps its pose
