## ADDED Requirements

### Requirement: Military bases are full airfields
Each military base SHALL be built from one site template with a main runway, a parallel taxiway, hardened shelters,
an alert apron, a cargo apron and hangar, a helicopter line, the mech hangar bays, a tower, a fuel farm, munitions
bunkers, living blocks, radar and SAM sites and a fenced perimeter, at full PBR within its LOD budgets.

#### Scenario: Within budget
- **WHEN** the base's glb is checked
- **THEN** each LOD is within its triangle budget and every map is 2048 px or less

### Requirement: Military and civil helicopter aircraft exist
The game SHALL have `fighter`, `military_cargo`, `military_helicopter` and `civil_helicopter` models with damage
zones, rigs, lamps, an `EYE_pilot` and door marks, each within its LOD budgets.

#### Scenario: Every flyable kind can be boarded
- **WHEN** an aircraft manifest is read
- **THEN** it names an `EYE_pilot` and at least one door
