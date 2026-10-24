## ADDED Requirements

### Requirement: The world reflects a cached sky
The pilot's eye camera SHALL light the world with an environment map built once from the sky cube
named in `data/world/sky_light.ron`, and the loading screen SHALL stay up until it is applied.

#### Scenario: A glass tower from the street
- **WHEN** the pilot stands in the street and looks up at a curtain-wall tower
- **THEN** the glass shows the sky's clouds, stronger toward grazing angles, over the rooms behind it

#### Scenario: The sky light switched off
- **WHEN** `cube` is empty in `data/world/sky_light.ron`
- **THEN** the game starts with the flat ambient at `ambient_without_sky_cd_m2` and no environment map

### Requirement: Glass reflects at full strength
Window glass SHALL add the light it reflects in full and dim what is behind it by its `alpha` only.

#### Scenario: A clear pane
- **WHEN** a pane with alpha 0.88 faces the sky
- **THEN** its reflection is as bright as an opaque surface of the same reflectance, and the room behind shows through at 12 %

### Requirement: Sky light tuning is validated
`data/world/sky_light.ron` SHALL be refused on load when a glass value is outside 0..1 or a
brightness is not finite, naming the field.

#### Scenario: Glass more opaque than one
- **WHEN** the file sets `glass: (alpha: 1.5)`
- **THEN** loading fails naming `glass.alpha`
