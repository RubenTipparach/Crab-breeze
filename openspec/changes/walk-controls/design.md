# Design: walk-controls

## 1. The rule (`cb-core::mech::walk`, pure)

`walk(keys, view_yaw, heading, throttle_min) -> Walk { throttle, turn_axis }`:
- **No keys:** throttle 0 (the mech brakes to a stop), no turn.
- **Straight back** (S alone): the legs face the view and back up at `throttle_min`.
- **Otherwise:** the way to walk is the view's yaw turned by the keys (`atan2(-x, y)`).
  - The legs turn toward it at 2 per radian off, which is a full turn beyond about 30 degrees.
  - The throttle is the keys' strength times the cosine of how far off the legs still are, so
    the mech does not stride off sideways while it turns.

`MechControls::throttle_set` holds the lever at the walking throttle; the lever keys do nothing in
walking mode.

## 2. Where it applies

Only when all of these hold:
- the seated pilot's own keys drive (`Actions::walk_keys`);
- the mech walks (not skating);
- it is on the ground.

With no key held, the coupled look's edge still turns the mech toward the view (seated-look).

The scripted pilot still drives the lever for now: its beats set `throttle_axis` and leave
`walk_keys` off. Moving its beats to the walking keys is task 2.1.

## 3. Air drag

`jet.drag_per_s` falls from 0.3 to 0.04. Drag on about 100 m2 at 30 m/s, about 54 kN on 40 t, is
about 1.3 m/s2, roughly 4 % a second. The skates' push in the air is unchanged.
