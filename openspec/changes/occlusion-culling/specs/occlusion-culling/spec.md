## ADDED Requirements

### Requirement: Hidden meshes are not drawn
The pilot's eye camera SHALL cull meshes hidden behind nearer geometry on the GPU, unless the view data switches it off or `CB_NO_OCCLUSION=1` is set.

#### Scenario: A street of towers
- **WHEN** the pilot stands in a street with towers on both sides
- **THEN** the buildings behind those towers are not drawn, and nothing that can be seen is missing
