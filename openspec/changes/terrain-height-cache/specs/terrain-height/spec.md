## ADDED Requirements

### Requirement: Ground heights are worked out once
The ground's height at a lattice corner SHALL be worked out at most once per run and kept, and every height read SHALL
return exactly the value the uncached formula gives.

#### Scenario: The same height, cached
- **WHEN** `height_at` is read at a point whose tile was filled earlier in the run
- **THEN** it returns the same value, bit for bit, as working the corners out from the island's shape, flats and rivers

#### Scenario: Past the world's edge
- **WHEN** a height is read past the edge of the world's lattice
- **THEN** it is worked out directly, as before, and no tile is made for it
