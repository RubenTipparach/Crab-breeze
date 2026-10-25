## ADDED Requirements

### Requirement: Rooms and furniture are drawn near the pilot only
A building's rooms and furniture SHALL be drawn only within the rooms radius of the pilot's eye (250 m), hidden beyond it plus a margin, and SHALL cast no shadows.

#### Scenario: A damaged tower 1 km away
- **WHEN** the pilot is 1 km from a shelled office tower
- **THEN** its frame and skin are drawn and its rooms and furniture are not
