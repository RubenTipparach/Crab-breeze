## ADDED Requirements

### Requirement: Every enemy in range is boxed
The cockpit SHALL draw a box with its distance round every live enemy mech within 2 000 m that is in front of the camera.

#### Scenario: A raider in the trees
- **WHEN** a raider stands 700 m ahead
- **THEN** a box round it reads its distance as 700 m

### Requirement: The lead marker is the simulation's lead
The lead marker for the selected target SHALL be drawn at the point `weapons::lead` gives for the main gun, the same function the AI pilots aim with.

#### Scenario: A crossing target
- **WHEN** the selected target crosses the view at 10 m/s, 500 m away
- **THEN** the lead marker sits ahead of its box by the distance the shell's flight time carries it

### Requirement: Missile lock only in range
The selected target SHALL lock when it is within 1 200 m and within 12 degrees of the aim for 1.0 s, SHALL NOT lock beyond 1 200 m, and SHALL lose the lock when the aim swings away.

#### Scenario: Lock and lose
- **WHEN** a target 800 m away stays within the aim for 1.0 s, then the aim swings 40 degrees off
- **THEN** the box reads `LOCK`, then loses it

#### Scenario: Out of range
- **WHEN** a target 1 500 m away stays on the crosshair for 5 s
- **THEN** it never locks

### Requirement: Missiles home on the lock
A missile fired with a lock SHALL turn toward its target's torso each step by no more than its seeker's turn rate, and one fired without a lock SHALL fly straight.

#### Scenario: A locked missile
- **WHEN** a missile is fired with a lock on a raider 600 m away walking across its path
- **THEN** it turns toward the raider at no more than 70 degrees a second and strikes it
