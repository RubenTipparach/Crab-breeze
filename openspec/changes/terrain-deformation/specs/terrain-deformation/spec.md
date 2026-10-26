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

### Requirement: The ground never opens while a crater is redrawn
A chunk's rebuilt mesh, which leaves a hole for each crater tile in it, SHALL go on screen only on the frame every one of those tiles is on screen or goes on screen with it; until then the chunk's old mesh stays drawn. A tile dug again while it is being meshed SHALL keep its job and be meshed once more after, never cancelled.

#### Scenario: A rocket lands on open ground
- **WHEN** a blast digs a new crater tile in a chunk near the eye
- **THEN** the old chunk stays drawn until the chunk's holed mesh and the new tile are both ready, and both go up on the same frame (`deform::patch_swap`, tests `a_chunk_with_a_new_hole_waits_for_its_tile` and `a_chunk_and_its_new_tile_go_up_on_the_same_frame`)

#### Scenario: A barrage on one spot
- **WHEN** rockets keep landing on the same tile faster than it is meshed
- **THEN** each finished mesh goes up and the next is started from the newer ground, so the crater is redrawn while the firing goes on
