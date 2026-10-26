## ADDED Requirements

### Requirement: A cockpit can wear an interior style

A cockpit SHALL be buildable in each interior style (wood, airliner, padded, excavator) by material changes alone, with its
geometry, nodes and screens unchanged, and the style's grain, quilting or stipple carried in its baked albedo and
normal maps.

#### Scenario: A padded cockpit
- **WHEN** a cockpit is built with the padded style
- **THEN** its walls and seat show diamond quilting in the normal map, and its triangle count is the same as unstyled

### Requirement: Styles are judged before they are baked

Each style SHALL have a seat-view mockup of the unbaked cabin before any cockpit is baked in it.

#### Scenario: Choosing a style
- **WHEN** a style is proposed for a cockpit
- **THEN** its front and side mockups from the seat exist and were shown to the owner

### Requirement: Nothing is mounted over a screen

Wall dressing (safety kit, panels, vents, wires) SHALL be placed only on solid wall, pillars, the roof and the
bulkhead, never over a screen showing the outside.

#### Scenario: The extinguisher
- **WHEN** a cockpit is dressed
- **THEN** no dressing item lies between the eye and any outside screen
