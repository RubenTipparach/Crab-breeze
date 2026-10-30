## ADDED Requirements

### Requirement: Three airliners are modelled with LODs, PBR materials and budgets
The game SHALL have a propliner, a four-jet narrowbody airliner and a three-engine wide-body, each built by
`tools/blender/build_airliners.py` into a glb with three levels of detail and a manifest, each level within its
triangle budget, every custom surface a full PBR set, and none wearing a real airline's or maker's name.

#### Scenario: The build refuses a level over budget
- **WHEN** a level of an aircraft has more triangles than its budget in the generator
- **THEN** the build stops with the level and both numbers, and writes nothing

#### Scenario: The models are their real aircraft's size
- **WHEN** the manifest's bounds are read
- **THEN** each aircraft's length, span and height are within 2 % of its table in the design

### Requirement: Gear, propellers, control surfaces and lights move by data
Every moving part SHALL be a named pivot listed in the manifest's `rig` with its axis, travel and time, and the page
and the game SHALL move them from that one table: the gear folds into its wells behind its doors, propellers and fans
turn, and the navigation lights, beacons and strobes light and flash in their real patterns.

#### Scenario: Gear up leaves nothing hanging
- **WHEN** the gear sequence reaches its end (gear up)
- **THEN** every wheel and leg is inside the fuselage or nacelle outline and every gear door is shut

### Requirement: Aircraft have interiors seen through their windows
Each aircraft SHALL be a hollow shell with window holes and glass, with its flight deck and every passenger seat row
modelled inside, drawn at the nearest level of detail only.

#### Scenario: A seat in every row
- **WHEN** the manifest's interior counts are read
- **THEN** the seats match the design's table for that kind

### Requirement: Aircraft break into damage zones
Each aircraft SHALL be split at structural joints into named damage zones, each its own mesh with closed, dressed cut
faces, its own hit capsules measured off its mesh, a parent in a part tree and a share of the kind's health; a zone at
zero health SHALL detach with its children and fly on as a debris piece of its own mesh.

#### Scenario: The zone tree is whole
- **WHEN** the manifest's zones are read
- **THEN** every zone but the centre has a parent that is a zone, the shares sum to the design's total, and every
  zone has at least one capsule inside the aircraft's bounds
