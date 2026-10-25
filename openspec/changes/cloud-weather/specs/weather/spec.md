## ADDED Requirements

### Requirement: Clouds are sorted billboards
Each cloud puff SHALL be drawn as one camera-facing billboard, lit as a sphere by the scene's sun and ambient light, its edge eroded by noise, and drawn back to front among the puffs in the draw radius. This replaces the blob meshes of `cloud-puffs`.

#### Scenario: One quad a puff
- **WHEN** the clouds in the draw radius are drawn
- **THEN** each puff is one quad, and the quads are in order from the farthest from the eye to the nearest

### Requirement: The weather is a set of seeded systems
The sky SHALL be laid out from the seed by weather systems, each a rule that places puffs with a kind, a darkness and a layer: fair cumulus by the ground's moisture, storm cells with anvils and rain shafts, a stratocumulus deck at sea, orographic cloud on the range's windward flank, lenticular caps over the highest peaks, and cirrus.

#### Scenario: The same seed, the same sky
- **WHEN** the world is built twice from the same seed
- **THEN** both skies hold the same puffs, shafts and cirrus in the same order

#### Scenario: Every puff in its layer
- **WHEN** the weather is built
- **THEN** every puff's centre lies between its system's base and top

#### Scenario: The desert is clearer than the jungle
- **WHEN** fair cumulus is placed
- **THEN** a tile over desert ground holds fewer clusters on average than a tile over jungle

### Requirement: It rains under a storm
Below a storm's base, inside its rain shaft, the cloud query SHALL report rain, and the glass SHALL take rain as it does inside a cloud.

#### Scenario: Under the shaft
- **WHEN** the eye is below a storm's base and inside its shaft
- **THEN** the rain on the glass builds toward 0.7 of full

#### Scenario: Beside the shaft
- **WHEN** the eye is below the base but outside every shaft and every puff
- **THEN** the glass dries
