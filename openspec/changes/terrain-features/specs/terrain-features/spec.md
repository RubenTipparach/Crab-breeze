## ADDED Requirements

### Requirement: Steep ground is textured without stretching
Ground steeper than 30 degrees SHALL be textured by biplanar projection with per-axis normals, so no texture stretches down a slope.

#### Scenario: A canyon wall
- **WHEN** a canyon wall is drawn
- **THEN** its texture is projected from the side, at the same texel density as flat ground

### Requirement: Cliffs inside feature volumes are meshed from a density field
Inside feature volumes over canyons, cliff bands, sea cliffs and waterfall faces, the ground SHALL be meshed from a signed distance field that can hold overhangs, ledges, arches and caves, and SHALL meet the heightmap without a gap at the volume's edge.

#### Scenario: The edge of a volume
- **WHEN** a feature volume's mesh and the heightmap outside it are drawn
- **THEN** the two surfaces meet within 0.1 m along the volume's edge

#### Scenario: An overhang
- **WHEN** a vertical line passes through an overhang inside a volume
- **THEN** the field crosses zero more than once along it

#### Scenario: Neighbouring cubes at different detail
- **WHEN** a 1 m cube sits beside a 2 m cube
- **THEN** their meshes share the boundary with no crack
