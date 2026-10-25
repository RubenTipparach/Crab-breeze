## ADDED Requirements

### Requirement: The mech's readouts are on the cockpit's MFDs

In a cockpit with MFD screens, the leg compass, damage, booster and skate gauges, flight, guns,
mission and navigation readouts SHALL be shown on the MFD screens and not over the pilot's view.

#### Scenario: Seated in the Foreman
- **WHEN** the pilot sits in a mech whose interior has `HUD_mfd_0..2`
- **THEN** each MFD screen shows one page of readouts (MECH, FLIGHT, MISSION)
- **AND** only the crosshair and targeting marks are drawn over the view

### Requirement: An MFD changes page when pressed

With free look held, clicking while looking at an MFD screen SHALL move that screen to its next page,
and the mech's guns SHALL NOT fire while free look is held.

#### Scenario: Pressing the left MFD
- **WHEN** the pilot holds free look, looks at the left MFD and clicks
- **THEN** the left MFD shows its next page and no shot is fired
