## ADDED Requirements

### Requirement: The building skins are Material Maker sets
The city buildings' brick, concrete, roof and timber SHALL be drawn from Material Maker graphs rendered to full PBR
sets (colour, normal, ORM), never from a generator in game code.

#### Scenario: A brick wall in low sun
- **WHEN** the pilot stands at a brick walk-up with the sun low across its face
- **THEN** the mortar joints are shadowed below each course and the bricks vary one from the next, in the building's own brick colour

### Requirement: A skin is laid at its set's own tile
Each skin SHALL be laid at the tile its set lists in `data/materials/materials.ron`, and the build's tests SHALL fail
when the two disagree or a set's maps are missing.

#### Scenario: A set's tile changes
- **WHEN** a skin set's `tile_m` is changed in `materials.ron` but not in the game
- **THEN** `every_skin_wears_a_material_maker_set_at_its_listed_tile` fails and names the material
