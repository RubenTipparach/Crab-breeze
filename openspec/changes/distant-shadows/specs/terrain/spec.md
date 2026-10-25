## ADDED Requirements

### Requirement: Far terrain shows the terrain's own shadows

Terrain drawn beyond the live shadows SHALL be darkened where the terrain shades it from the sun, from
a shadow grid baked once at load.

#### Scenario: A ridge at sunset
- **WHEN** a ridge stands between the sun and a valley more than 700 m from the pilot
- **THEN** the valley floor behind the ridge is drawn in shade
