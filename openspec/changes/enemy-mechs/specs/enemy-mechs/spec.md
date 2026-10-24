## ADDED Requirements

### Requirement: Enemies use the same simulation
Enemy mechs SHALL move, aim, fire and take damage through the same simulation as the player's mech, driven by an AI that writes the same controls.

#### Scenario: The AI aims
- **WHEN** an AI mech has the player in view
- **THEN** it twists its torso toward the player's predicted position and fires when its aim error is under its skill's threshold
