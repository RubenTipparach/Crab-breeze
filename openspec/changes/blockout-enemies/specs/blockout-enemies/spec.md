## ADDED Requirements

### Requirement: Enemies wear block-out skins
An enemy that names a skin SHALL be drawn as that block-out model on the stand-in's skeleton, with the stand-in's boxes
hidden, and SHALL fight, take hits and be destroyed exactly as the stand-in does.

#### Scenario: A crab in the first fight
- **WHEN** the first encounter names the skin `tyrant` for an enemy
- **THEN** that enemy is drawn as the Tyrant crab, walks with its legs stepping in time with its gait, and fires from the stand-in's sockets

#### Scenario: An unknown skin is refused
- **WHEN** an encounter names a skin that is not in `data/enemies/skins.ron`
- **THEN** loading fails naming the skin

### Requirement: A lost part takes its limb
When a skinned enemy loses an arm or a leg part, the matching limb of its skin SHALL come off whole as debris.

#### Scenario: An arm is shot off a combat mech
- **WHEN** a skinned combat mech's right arm part reaches zero
- **THEN** its `ARM_R` node with the weapon falls away under physics and the stump burns
