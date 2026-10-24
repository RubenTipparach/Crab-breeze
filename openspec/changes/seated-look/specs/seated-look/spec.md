## ADDED Requirements

### Requirement: The seated view is coupled to the mech's nose

While seated and not in free look, the pilot's view SHALL stay within `seated_look_limit_deg` of the
mech's heading, and mouse input past that limit SHALL turn the mech toward the view at the mech's
own turn rate instead of turning the view further.

#### Scenario: Looking past the edge turns the mech
- **WHEN** the seated pilot moves the mouse to turn the view past 90 degrees from the mech's nose
- **THEN** the view stops at 90 degrees from the nose
- **AND** the mech turns toward the view at its turn rate

#### Scenario: The view never faces behind the pilot
- **WHEN** the seated pilot keeps turning the mouse in one direction
- **THEN** the view stays within 90 degrees of the mech's nose throughout

### Requirement: Free look uncouples the view

While the free look control is held, the seated view SHALL turn with the mouse without limit and
the mech SHALL NOT turn for it; on release the view SHALL NOT be moved, and the mech SHALL turn
toward the view at its turn rate until the view is within the limit.

#### Scenario: Free look sees behind
- **WHEN** the pilot holds free look and turns the view 180 degrees
- **THEN** the view faces behind the mech and the mech's heading is unchanged

#### Scenario: Releasing free look
- **WHEN** the pilot releases free look with the view 150 degrees from the nose
- **THEN** the view stays where it is
- **AND** the mech turns toward it until the view is within 90 degrees
