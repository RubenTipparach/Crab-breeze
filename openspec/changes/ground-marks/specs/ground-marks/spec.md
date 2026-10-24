## ADDED Requirements

### Requirement: The ground decides the mark
The surface under a foot or a wheel SHALL be decided by one rule: water SHALL give no mark; ground where the shared cover rule with no clearance says nothing grows (a highway, a city's paved ground, a building's lot) SHALL be paving; all other ground (open country, city parks) SHALL be soil.

#### Scenario: A road, a street, a meadow and the sea
- **WHEN** the surface is asked on a highway on dry land, a city street, a meadow and the sea of seed 42
- **THEN** the highway and the street are paving, the meadow is soil, and the sea gives no mark

### Requirement: A foot strike is the gait's stance start
A foot strike SHALL be decided in one place: the left foot when the gait clock crosses a whole cycle, the right half a cycle later, while the stride swings at least 0.3 and the leg is there, and both feet on the step the mech lands from the air. The footfall spray, the footprint, the strike effect and the pilot's jolt SHALL all use it.

#### Scenario: Walking
- **WHEN** the gait clock passes 1.0, then 1.5, then moves within a stance
- **THEN** the left foot strikes, then the right, then neither

#### Scenario: A shuffle, a lost leg and a landing
- **WHEN** the stride swing is 0.1, or the left leg is lost, or the mech lands
- **THEN** no foot strikes in the first two cases, and both strike on the landing

### Requirement: Marks live in a fixed ring and fade
Every mark SHALL live in a ring of `max_marks` slots: a new mark SHALL take the slot after the last one laid, replacing the oldest; a mark SHALL show fully for the first two thirds of its life, fade linearly to nothing at its end, and then be cleared. A mark with a non-finite position SHALL be refused and change nothing.

#### Scenario: Past the capacity
- **WHEN** four marks are laid in a ring of three
- **THEN** the fourth replaces the first, the second is still there, and the next one replaces the second

#### Scenario: Fading
- **WHEN** a 90 s footprint is 59 s, 75 s and 90 s old
- **THEN** it shows fully, at half strength, and not at all

### Requirement: A paving print is bold
A footprint on paving SHALL be at least 0.08 m deep, and its quad SHALL reach `look.crack_reach` half sole lengths from its middle every way, so its cracks reach about 1.5 times the sole, while the sole keeps its size.

#### Scenario: A paving print and a soil print
- **WHEN** a paving and a soil footprint are made from the shipped data
- **THEN** the paving one is 0.08 to 0.12 m deep, its quad reaches at least 1.5 sole half lengths along and across, and both have the same sole size

### Requirement: Marks lie on the ground
A mark SHALL be laid at the ground's height as the one height query gives it, with the heights of the drawn grid's vertices sampled from the same query, so it follows the slope; a footprint's toe SHALL point along the mech's heading, and a landing SHALL press deeper than a step; paving SHALL have no lip.

#### Scenario: A footprint on a slope
- **WHEN** a footprint is laid on ground rising 0.5 m a metre across x
- **THEN** its middle is on the ground, its corner heights differ by the slope over its width, and its toe points along the heading

### Requirement: Skate trails are laid end to end
A wheel rolling on dry ground faster than `trail_min_speed_m_s` SHALL lay a straight segment from where its last segment ended each time it has glided `trail_segment_m`, the segment filling its quad's length; a jump longer than `trail_break_m` or a wheel not reported for 0.3 s SHALL start a new line.

#### Scenario: A line of segments
- **WHEN** a wheel moves 0, 2, 4.5 and 9 m along a line, then jumps 51 m
- **THEN** a segment from 0 to 4.5 m is laid, the next starts at 4.5 m, and the jump lays nothing

### Requirement: All marks are one draw
Every mark SHALL be drawn by one entity with one mesh, one material and one storage buffer of fixed size, as a multiply over the ground drawn before anything see-through, with a mesh of a 5 by 5 vertex grid a slot whose only vertex data is (slot, grid x, grid z). The shader SHALL compile with naga against Bevy 0.20's `bevy_pbr`, and its uniform and mark row SHALL match the Rust sizes.

#### Scenario: The mesh and the shader
- **WHEN** the marks mesh is built for 1 024 slots and the shader is compiled for the pilot's pass
- **THEN** it has 25 600 vertices and 32 768 triangles, and the WGSL `MarksLook` and `Mark` are the sizes of `MarksUniform` and 40 floats

### Requirement: The pilot feels a step in position only
On the piloted mech's foot strikes while the pilot is seated, the pilot's eye SHALL drop and recover over `jolt.duration_s`, by an amount growing with speed up to `jolt.max_cm` (or `jolt.landing_cm` on a landing); the jolt SHALL change only the eye's position, never its rotation.

#### Scenario: A step at speed
- **WHEN** the mech strikes a foot at 20 m/s and the eye is jolted for 0.2 s
- **THEN** the eye's rotation and scale never change, and its position moves at most 5 cm and returns

### Requirement: The ground picks the effect
A foot strike SHALL play `foot_strike_dirt` on soil and `foot_strike_concrete` on paving (`landing_dirt` and `landing_concrete` on a landing); a wheel faster than `skate_fx_min_m_s` SHALL play `skate_dirt` on soil and `skate_grit` on paving; every one SHALL be a recipe the data must carry.

#### Scenario: Recipes
- **WHEN** the recipes for each surface are asked
- **THEN** each is one of `cb_core::fx::RECIPES`, which startup checks the data carries

### Requirement: Ground marks are tuned in data and can be switched off
Every number SHALL come from `data/fx/ground_marks.ron`, loaded and validated through `GameData` (unknown keys refused, ranges checked, a trail break no shorter than a segment), and `CB_NO_GROUND_MARKS=1` SHALL lay and draw no mark while the effects and the jolt stay.

#### Scenario: A bad file
- **WHEN** the file sets `trail_break_m` shorter than `trail_segment_m`, or names an unknown jolt key
- **THEN** loading fails naming the file and the field

### Requirement: Soil prints and skate grooves clear the grass for their life
Every live soil footprint and soil skate segment SHALL clear the grass under it: a grass tile SHALL pack zero density in every cell whose middle lies inside the mark's crush rectangle (the sole grown by `crush_margin_m`, or the trail's width), and the cell SHALL grow again once the mark has faded out. Paving marks clear nothing.

#### Scenario: A print in a meadow
- **WHEN** a tile is packed with a footprint's crush rectangle over it
- **THEN** the cells under the sole pack zero, a cell 3 m away keeps its density, and with no rectangle the tile packs as baked

#### Scenario: The ring tells the grass what changed
- **WHEN** a soil print is laid, a paving print is laid, and the soil print fades out
- **THEN** the crush version moves on the first and the third, and the crush list holds the soil print only while it lives

### Requirement: No vertex stage reads the view's lights
No game shader SHALL read Bevy's view `lights` binding in a vertex stage, since Bevy binds it to fragments only.

#### Scenario: The game's shaders
- **WHEN** every game shader is compiled
- **THEN** no vertex entry point reaches the `lights` global
