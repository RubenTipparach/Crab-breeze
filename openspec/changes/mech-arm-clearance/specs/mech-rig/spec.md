## ADDED Requirements

### Requirement: A mech's arms clear its legs
A detailed mech's arms SHALL be held clear of its legs by its pose, its model unchanged, with no surface of an arm intersecting a surface of a leg, standing,
walking, turning, skating and crouching.

#### Scenario: The Harrow walks
- **WHEN** the Harrow is posed through a full walk cycle
- **THEN** no arm triangle intersects a leg triangle, and the least gap is at least 0.3 m
