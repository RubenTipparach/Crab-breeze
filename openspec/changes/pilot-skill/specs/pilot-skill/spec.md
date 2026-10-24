## ADDED Requirements

### Requirement: The pilot dodges incoming fire
An AI pilot SHALL sidestep a shot predicted to pass within 6 m of its torso inside 1.6 s, and SHALL jump on its jets for a rocket or heavy shell about to hit when the jets are cool.

#### Scenario: A rocket comes in
- **WHEN** a rocket is 0.8 s from passing through the mech's torso
- **THEN** the pilot jumps on its jets and steers across the rocket's path

### Requirement: No wasted fire
An AI pilot SHALL fire only when the line to its target is clear of buildings and allies.

#### Scenario: Target behind a wall
- **WHEN** a building stands between the mech and its target
- **THEN** the pilot does not fire and moves to find a clear line
