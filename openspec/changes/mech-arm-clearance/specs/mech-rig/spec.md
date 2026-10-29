## ADDED Requirements

### Requirement: A kneeling mech's arms clear its legs
When a detailed mech kneels to be boarded, its arms SHALL stay clear of its legs, with no surface of an arm intersecting
a surface of a leg once the kneel is three quarters done, by moving the arms in the kneel alone: its other poses and its
model SHALL be unchanged.

#### Scenario: The Harrow kneels
- **WHEN** the Harrow is posed at 75% and at 100% of its kneel
- **THEN** no arm triangle intersects a leg triangle, and the least gap is at least 0.2 m

#### Scenario: Standing is untouched
- **WHEN** a mech stands with its kneel at 0
- **THEN** its arm pose is exactly the pose it had before this change
