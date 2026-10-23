## ADDED Requirements

### Requirement: The pilot walks in first person
The pilot SHALL walk on foot from their own eyes, on the shared ground query, and SHALL NOT pass through buildings.

#### Scenario: Walking into a wall stops at the wall
- **WHEN** the pilot walks into a building for 5 s
- **THEN** the pilot's body stays outside the building's footprint

### Requirement: Boarding is one continuous first-person motion
Boarding SHALL move the pilot's eyes along one continuous path from where they stand, up to `HATCH_entry` and into `EYE_pilot`, with no cut and no other camera.

#### Scenario: Boarding from the ladder
- **WHEN** the pilot presses Use within 3 m of `LADDER_base`
- **THEN** the eyes move along the path with no step longer than speed times `dt_s`, and end seated at `EYE_pilot`

#### Scenario: Too far to board
- **WHEN** the pilot presses Use 10 m from `LADDER_base`
- **THEN** the pilot stays on foot

### Requirement: Disembarking
A seated pilot SHALL climb out along the reverse path when the mech is stopped, and stand at `LADDER_base`.

#### Scenario: Climbing out
- **WHEN** the seated pilot presses Use with the mech stopped
- **THEN** the pilot ends on foot at `LADDER_base`
