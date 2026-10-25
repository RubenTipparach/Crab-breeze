## ADDED Requirements

### Requirement: A building's opaque parts are one draw
A damaged building's opaque frame, rooms, furniture and skin SHALL be one mesh drawn with one material that reads every opaque building look from texture arrays; its glass SHALL be a second mesh.

#### Scenario: A damaged office tower
- **WHEN** a dressed office tower is shelled
- **THEN** it is drawn as two meshes (opaque and glass) per pass, and looks as it did with a material per look
