## ADDED Requirements

### Requirement: The cockpit is drawn in the mech's own frame
While the pilot is seated with the hatch shut, the cockpit pass SHALL draw the cabin and place its camera from the
local transforms between each part and the mech's root, so no position in the pass depends on where the mech stands in
the world.

#### Scenario: A still mech far from the origin
- **WHEN** a mech stands still 8 000 m from the map's origin with the pilot seated
- **THEN** every transform the cockpit pass draws with is within a few metres of its origin, and the cabin does not
  move against the view from frame to frame

### Requirement: The seated view glides between simulation steps
The seated view's yaw SHALL be the view's own direction blended between the last two fixed steps, plus the mouse not
yet stepped, so a turning mech's view neither lags nor snaps at a step.

#### Scenario: A mech turning toward the view
- **WHEN** the mech turns at a steady rate toward a held view and frames fall at fractions of each step
- **THEN** the drawn yaw moves the same way at every frame and by no more than the turn since the last frame
