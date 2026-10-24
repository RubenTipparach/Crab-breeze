## ADDED Requirements

### Requirement: One water material for all water
Every water surface (rivers, lakes and the ocean) SHALL be drawn with the one `WaterMaterial`, made by the one helper `world::water_material`, so that every body of water shares one look and one implementation.

#### Scenario: Rivers and lakes share the material
- **WHEN** the world spawns its lakes and rivers
- **THEN** every water mesh carries a `WaterMaterial` and none carries a `StandardMaterial`

### Requirement: The surface stays where the simulation says it is
The water's look SHALL NOT displace the surface: the swell and ripples change only the shading normal, so the surface drawn is at the `surface_m` the one water query returns.

#### Scenario: The swell is in the normals
- **WHEN** the water surface is drawn at a point the water query gives a surface of 4.0 m
- **THEN** the drawn surface is at 4.0 m whatever the swell

### Requirement: The eye knows how wet it is
Each frame the pilot's eye SHALL be classed from the one water query as dry (0), straddling the surface within the partial band (0.5) or under water (1), and the class SHALL drive the underwater view.

#### Scenario: Under, straddling and dry
- **WHEN** the eye is 3 m below a surface with a 0.8 m band
- **THEN** it is under water (1)
- **WHEN** the eye is 0.5 m above the surface
- **THEN** it is straddling (0.5)
- **WHEN** the eye is 2 m above the surface, or over dry ground
- **THEN** it is dry (0)

### Requirement: Water runs off the glass after surfacing
When the eye comes up out of the water, drops SHALL run on the glass at full strength and fade to nothing over the dry time in data; while straddling the surface the glass SHALL stay wet; under water there SHALL be no drops.

#### Scenario: Surfacing
- **WHEN** the eye goes from under water to dry with a 2.6 s dry time
- **THEN** the drops are at full strength, at half strength 1.3 s later, and gone after 2.6 s

#### Scenario: Diving again
- **WHEN** the eye goes back under water while the glass is wet
- **THEN** there are no drops until it surfaces again, when a fresh window opens

### Requirement: Water look tuning is data
The water's look SHALL be tuned in `data/world/water_look.ron`, loaded and validated by the one data loader: an unknown key, a non-finite number or a value out of range stops startup naming the file and field.

#### Scenario: A bad value
- **WHEN** `water_look.ron` sets `emerge_dry_s: -1.0`
- **THEN** loading fails naming `data/world/water_look.ron` and `emerge_dry_s`

### Requirement: The water shaders compile
The water's WGSL and WESL shaders SHALL be compiled and validated by `cargo test`, so a broken shader fails the build checks rather than the game.

#### Scenario: A broken post shader
- **WHEN** `water_post.wesl` or `water_common.wesl` has a type error
- **THEN** `cargo test --workspace` fails
