## ADDED Requirements

### Requirement: The rockets rumble
While the rockets burn, the cockpit SHALL play the rocket loop and the sub rumble at a volume that rises with the lift, and SHALL shake the cockpit's position, never the view's direction.

#### Scenario: Full lift
- **WHEN** the pilot holds Space with the rockets ready
- **THEN** the rocket loop and the rumble play, the cockpit shakes by a few millimetres, and the pilot's yaw and pitch are unchanged

### Requirement: Far sounds arrive late
A sound in the world SHALL start after the time sound takes to travel from it to the pilot at 343 m/s.

#### Scenario: A distant shell
- **WHEN** a shell explodes 686 m from the cockpit
- **THEN** its flash is seen at once and its sound starts 2 s later
