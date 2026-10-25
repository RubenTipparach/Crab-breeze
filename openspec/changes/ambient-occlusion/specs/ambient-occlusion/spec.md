## ADDED Requirements

### Requirement: The world view has screen-space ambient occlusion
The pilot's eye camera SHALL draw screen-space ambient occlusion with the radius and quality in the view data, unless the radius is 0 or `CB_NO_SSAO=1` is set.

#### Scenario: A shelled office floor in shade
- **WHEN** the pilot looks into a floor whose facade is shot away
- **THEN** the corners, the slab edges and the floor around the furniture are darker than the open floor
