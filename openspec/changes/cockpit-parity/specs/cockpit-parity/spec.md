## ADDED Requirements

### Requirement: Every cockpit carries the Foreman's six MFD screens

Every interior a mech's data names SHALL carry the three hanging MFD screens (`HUD_mfd_0..2_screen`) and the dash's
three screens (`SCREEN_left/center/right_screen`), with `EYE_pilot`, `SEAT_pilot`, at least four `LIGHT_*` lamps and
every part the mech's data rides on its chest, and `cb-tools validate` SHALL refuse an interior that does not.

#### Scenario: A cockpit missing its dash
- **WHEN** a mech's data names an interior glTF with no `SCREEN_center_screen`
- **THEN** `cb-tools validate` fails, naming the mech file, the interior and the missing screen

#### Scenario: Seated in the Ironhorse
- **WHEN** the pilot sits in the Ironhorse (the Excavator cab)
- **THEN** six screens show live MFD pages, three hung above the front screen and three on the dash

### Requirement: Nothing crosses the front screen near the eye line

In every roster cockpit, no frame member, rail or seam SHALL cross the front screen within 15 degrees above or below
the eye line, seen from `EYE_pilot`.

#### Scenario: The Cab's row seam
- **WHEN** the pilot sits in the Excavator cab and looks straight ahead
- **THEN** the rail between its two screen rows is hidden behind the dash's glare shield
