## ADDED Requirements

### Requirement: Every furniture box names its surface
Every box of every furniture kit item SHALL name the surface it is drawn with, one of wood, fabric, metal, plastic, cardboard or leaves; concrete and plaster SHALL be kept for the structure.

#### Scenario: A furnished storey
- **WHEN** a storey's plan is furnished
- **THEN** every box of every item carries a furniture surface

### Requirement: Building parts are drawn with textured surfaces
Columns, slabs and core walls SHALL be drawn with the concrete set, room partitions with the plaster set, furniture with the set its box names, and falling chunks with the concrete set; each set SHALL be a full PBR set (colour, normal, occlusion-roughness-metal) with a full mip chain.

#### Scenario: A shelled office floor
- **WHEN** a building's facade is shot away and its floor is seen from the street
- **THEN** its columns, slab, walls and furniture show their textures, tinted by their colours
