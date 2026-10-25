## ADDED Requirements

### Requirement: Crabs walk on their own legs
A crab SHALL move on its legs with a gait that keeps at least half its legs planted and its planted feet still on the ground.

#### Scenario: A Skitter scuttles sideways
- **WHEN** a Skitter strafes left at full speed for 5 s
- **THEN** at every step at least two of its four feet are on the ground, and a planted foot does not move until it lifts

### Requirement: The defence raiders are crabs
The defence mission's waves SHALL be alien crabs from `data/crabs/`, not copies of the player's mech.

#### Scenario: Wave 1 appears
- **WHEN** wave 1 starts
- **THEN** its 10 raiders are Skitters

### Requirement: Each leg and claw arm can be shot off
Every crab's legs and claw arms SHALL be parts with their own armour, and a limb whose armour reaches zero SHALL come
off whole as debris, and the crab SHALL lose what that limb gave it: speed and gait for a leg, its gun for an arm.

#### Scenario: A hind leg is shot off
- **WHEN** a two-pair crab's left front hind leg reaches zero armour
- **THEN** the leg falls away as one body, the crab walks on the three hind legs left at three quarters of its top speed

#### Scenario: A claw is shot off
- **WHEN** a crab's right claw arm reaches zero armour
- **THEN** the arm falls away with its gun, the crab stops firing that gun, and it walks leaning on the left claw
