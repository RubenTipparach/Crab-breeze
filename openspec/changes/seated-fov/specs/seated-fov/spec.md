## ADDED Requirements

### Requirement: The view widens in the cockpit and narrows on foot, eased

On foot the view's vertical field of view SHALL be `on_foot_fov_deg` (75 degrees), seated in the cockpit
`view_fov_deg` (95 degrees, the Options setting), and moving between them SHALL ease over `fov_change_s` with no
jump; the view's direction SHALL not change.

#### Scenario: Sitting down
- **WHEN** the pilot is carried into the seat and sits
- **THEN** the view widens from 75 to 95 degrees over 0.8 s, easing in and out

#### Scenario: A wider setting
- **WHEN** the player sets the field of view to 100 degrees on the Options page
- **THEN** the view is 100 degrees seated and 80 degrees on foot
