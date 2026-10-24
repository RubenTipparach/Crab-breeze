## ADDED Requirements

### Requirement: The world is an island in the sea
The world generator SHALL build a square world 40,960 m a side from the seed, with dry land in its middle that falls over a shelf to an open sea floor between 20 and 60 m below sea level, and sea at every point of its edge.

#### Scenario: Land in the middle
- **WHEN** the shipped world is generated with seed 42
- **THEN** at least nine in ten points within 8 km of the centre stand above sea level

#### Scenario: Sea at the edges
- **WHEN** the ground is sampled along the world's edge and past it
- **THEN** every point is under the sea

#### Scenario: A sunk mech can walk ashore
- **WHEN** the open sea floor is sampled past the shelf
- **THEN** it lies 20 to 60 m below sea level, and below sea level the shore never climbs faster than 1 in 4

### Requirement: One water query reports the sea
`water_at` SHALL report the sea, with its surface at sea level and no flow, wherever the ground is below sea level, inside the world and past its edge.

#### Scenario: The sea past the edge
- **WHEN** `water_at` is asked at a point past the world's edge
- **THEN** it reports water whose surface is sea level and whose depth is sea level minus `height_at`

### Requirement: One ground query at every level of detail
The ground's height SHALL be a pure function of position, and `height_at` SHALL be the one query the drawn ground at every level of detail, the colliders, the pilot and the mechs read.

#### Scenario: Full detail lies on the query
- **WHEN** a chunk is meshed at full detail
- **THEN** `height_at` at the centre of each of its triangles equals the triangle's height

### Requirement: The ground is drawn at levels of detail
The ground SHALL be drawn as quadtree nodes of equal triangle count, coarser farther from the eye, with hysteresis at ring edges, skirts at their edges, and every chunk covered by exactly one drawn node.

#### Scenario: Farther is coarser
- **WHEN** nodes are selected for an eye at a corner of the ground
- **THEN** the node under the eye is a full-detail chunk, the far corner is drawn at level 2 or coarser, and no node is coarser than its distance allows

#### Scenario: No flicker on a ring's edge
- **WHEN** the eye moves back and forth by less than the hysteresis across a ring's edge
- **THEN** the selected nodes do not change

#### Scenario: Few meshes for the whole island
- **WHEN** the shipped island is selected for the eye at the start
- **THEN** fewer than 200 ground meshes cover it

### Requirement: Trees grow near the bodies
Trees SHALL be grown by tile, deterministically from their site, only within the keep radius of the pilot and every mech, and SHALL be let go beyond the release radius, remembering the felled ones.

#### Scenario: Grown in any order
- **WHEN** the same tile is grown after different tiles
- **THEN** its trees and their ids are the same

#### Scenario: A felled tree stays felled
- **WHEN** a tree is felled, its tile let go and grown again
- **THEN** the tree lies felled

### Requirement: The game starts at the island's centre
The first city placed SHALL be the largest, within `capital_within_m` of the island's centre, and the game SHALL start in it with every encounter starting on dry land.

#### Scenario: The capital
- **WHEN** the shipped world is generated with seed 42
- **THEN** city 0 has the largest size, its centre is within 2,500 m of the origin, and the mech starts in it
