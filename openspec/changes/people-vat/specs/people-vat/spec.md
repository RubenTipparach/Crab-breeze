## ADDED Requirements

### Requirement: People are real bodies from the humanoid pipeline
People SHALL be built by a committed Blender generator from packs pinned by SHA-256 (MPFB 2 bodies on the 53-bone game_engine rig, CC0 MakeHuman clothes, Quaternius' Universal Animation Library clips), standing 1.75 m before each person's own scale, and the generator SHALL rebuild every file byte for byte.

#### Scenario: Rebuilding the people
- **WHEN** `build_people.py -- --verify` runs with the pinned packs
- **THEN** every glb, bake and the manifest match the committed files

### Requirement: Clips are baked into vertex animation textures with a manifest
Every clip (idle, walk, run, sit, wave, talk, fall) SHALL be baked per level of detail into a texture of every vertex's position and normal per frame, with a manifest of each clip's rows, frames, rate, loop and walking speed, and a test SHALL hold the manifest to the bake files and to the glb.

#### Scenario: The manifest against the files
- **WHEN** the tests run
- **THEN** each level's file has the vertices, triangles, rows and bounds the manifest says, its triangles equal the glb's mesh of that level, and every clip is in the glb with a key a frame

### Requirement: Every level of detail has a triangle budget that is enforced
Each level SHALL carry a triangle budget in its manifest, the build SHALL refuse a level over it, and the manifest check SHALL refuse it too.

#### Scenario: One triangle over
- **WHEN** a level has one triangle more than its budget
- **THEN** the manifest check fails naming the body, the level and both numbers

### Requirement: What a person does picks their clip
Walking SHALL play the walk at the pace over the clip's own walking speed (within a cadence range), standing still or waiting the idle, sitting the sit, a greeting the wave and then the talk, fleeing the run, and a person struck down the fall held at its last frame; a change of clip SHALL crossfade over `draw.crossfade_s`.

#### Scenario: A greeting
- **WHEN** a person is greeted
- **THEN** they wave for `draw.wave_s`, then talk until the greeting ends

#### Scenario: Feet keep to the ground
- **WHEN** a person walks faster than the walk clip's own speed
- **THEN** the clip plays faster in proportion, so one loop covers the ground the body covers

### Requirement: People draw in a fixed number of draws
All people SHALL be drawn by one vertex-animated material per body and level from one storage buffer of every person, so the draw count does not grow with the crowd, and nothing per person SHALL be rebuilt or spawned on the CPU per frame.

#### Scenario: Two thousand people
- **WHEN** 2,000 people are round the pilot
- **THEN** they draw in the same number of draws as twenty
