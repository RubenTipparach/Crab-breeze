## ADDED Requirements

### Requirement: Many kinds of building downtown
The city generator SHALL pick each building's style from per-band weights in `data/world/buildings.ron`, among the styles whose storey range holds the building's storeys.

#### Scenario: Downtown is not one glass type
- **WHEN** seed 42 is generated
- **THEN** no single style is more than 30 % of the buildings within 500 m of the start, and brick stands on buildings above 6 storeys

### Requirement: Every facade surface has relief
Every facade material, trim included, SHALL carry a normal map.

#### Scenario: A sill in low sun
- **WHEN** the sun is low across a walk-up's facade
- **THEN** the sills, frames and cornice show relief as well as the brick does
