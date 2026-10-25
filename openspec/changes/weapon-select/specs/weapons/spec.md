## ADDED Requirements

### Requirement: The primary trigger fires the selected station

The pilot's primary trigger SHALL fire only the selected weapon station, and stepping the selection
SHALL skip a station whose arm is lost.

#### Scenario: Wheeling to the cannon
- **WHEN** the autocannon is selected and the pilot steps the selection once
- **THEN** the next usable station is selected and the left mouse fires it alone

#### Scenario: A lost arm's gun is skipped
- **WHEN** the station after the selected one is on a lost arm and the pilot steps the selection
- **THEN** that station is passed over
