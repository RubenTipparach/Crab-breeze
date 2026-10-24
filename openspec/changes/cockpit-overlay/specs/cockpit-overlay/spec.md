## ADDED Requirements

### Requirement: Nothing draws through the cockpit
While the pilot is seated, the pilot's mech SHALL be drawn after the world on its own depth, so no world geometry or effect can appear inside or through it.

#### Scenario: Smoke at the cockpit
- **WHEN** a smoke puff drifts through the space the cockpit frame occupies
- **THEN** the frame is drawn whole in front of it

#### Scenario: On foot
- **WHEN** the pilot walks beside the mech
- **THEN** the mech is drawn with the world and sorts with it as any other object
