# Proposal: walk-controls

## Why

The owner (2026-10-25): "I dont like the legs being tank controls, tank controls on skate makes sense,
but when in walking mode, wasd should be standard walking", and "when flying if skate is off I should
inherit inertia, think of skate as lateral thrusters, for horizontal movement".

## What changes

- **Walking mode:** WASD is direct, as on foot.
  - W walks toward where the pilot looks, S backs away from it, A and D step to either side.
  - The legs turn toward the way to walk while the torso keeps facing the view.
  - With no key held the mech stops.
- **Skating and the jets** keep the throttle lever and the leg turn.
- **In the air** the mech keeps its momentum: air drag falls from 30 % a second to about 4 %, near a
  40 t mech's real drag. The skates stay its side thrusters for moving across.
