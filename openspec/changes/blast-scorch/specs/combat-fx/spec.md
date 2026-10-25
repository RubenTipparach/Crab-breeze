## ADDED Requirements

### Requirement: A blast on soil leaves a textured scorch
A crater dug in soil SHALL be covered by a scorch decal drawn from the `blast_scorch` Material Maker maps, sized from the crater's radius, and no more than `max_scorches` SHALL stand at once, the oldest going first.

#### Scenario: A rocket hits a meadow
- **WHEN** a 1 MJ rocket digs a crater in open ground
- **THEN** one scorch decal is spawned over it, its side `scorch_per_radius` times the crater's radius

#### Scenario: The cap
- **WHEN** a crater is dug while `max_scorches` scorches stand
- **THEN** the oldest scorch is removed and the new one spawned
