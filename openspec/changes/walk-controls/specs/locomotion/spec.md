## ADDED Requirements

### Requirement: Walking mode moves the mech directly with WASD

In walking mode on the ground, the player's movement keys SHALL walk the mech toward, away from, or
to the side of the view, the legs turning toward the way to walk, and SHALL stop it when no key is
held.

#### Scenario: Letting go of W
- **WHEN** the pilot stops holding W while walking
- **THEN** the throttle goes to zero and the mech brakes to a stop
