## ADDED Requirements

### Requirement: Smoke is drawn as lit puffs
Explosion smoke and thruster and missile trails SHALL be drawn as 3D puffs lit by the scene's sun and ambient light the way the clouds are, each sorted on its own, growing, rising and fading over its life.

#### Scenario: The pilot lifts on rockets
- **WHEN** the pilot holds the rockets and climbs
- **THEN** a trail of puffs is left below and behind the mech, lasting several seconds, visible when the pilot looks down or round

#### Scenario: A shell explodes in a street
- **WHEN** a shell bursts
- **THEN** its smoke is lit puffs that do not flicker as other smoke comes and goes
