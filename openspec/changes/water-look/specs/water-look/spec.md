## ADDED Requirements

### Requirement: The water reflects the sky and what stands in it
The water surface SHALL reflect the game's sky with its clouds, and SHALL reflect things drawn before it (the ground, buildings, the mech) where a screen-space trace finds them.

#### Scenario: A cloud overhead
- **WHEN** a cloud stands over calm water in view
- **THEN** the water under it shows the cloud's colour within the reflection's blur

#### Scenario: The mech wading
- **WHEN** the mech stands in water in view
- **THEN** the water in front of it shows the mech's reflection, and the sky where the trace leaves the screen

### Requirement: Far water keeps its roughness
Wave detail SHALL be filtered by the pixel's footprint and the detail lost SHALL raise the surface's roughness, so far water shows a broad sun path rather than sparkle or a mirror.

#### Scenario: Looking toward a low sun
- **WHEN** the sun is 10 degrees up ahead and the sea fills the lower half of the view
- **THEN** the glint covers a band toward the sun wider than the sun's disc, and no pixel flickers between frames at rest
