## ADDED Requirements

### Requirement: Explosions dig craters
An explosion on open ground SHALL lower the ground into a bowl with a raised rim, sized from its energy, and every body and drawing SHALL see the new ground through the one ground query.

#### Scenario: A shell lands
- **WHEN** a 4 MJ explosion hits flat open ground
- **THEN** `height_at` at its centre is about 1.2 m lower, and the mech's feet stand on the new ground

### Requirement: Deformation is deterministic
The ground SHALL depend only on the base world and the ordered list of explosions.

#### Scenario: Replaying a fight
- **WHEN** the same explosions are applied twice in the same order
- **THEN** the two grounds are identical
