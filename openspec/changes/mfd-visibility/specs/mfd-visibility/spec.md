## ADDED Requirements

### Requirement: Every MFD is whole and square from the seat

In every roster cockpit, seated and looking level at the 95 degree view, each of the six MFD screens SHALL lie wholly
inside the picture and unhidden, and SHALL be drawn between 0.9 and 1.2 times as high as it is wide.

#### Scenario: The Harrow's dash screens
- **WHEN** the pilot sits in the Harrow and looks straight ahead
- **THEN** the three dash screens stand whole above the dash, square, under the front screen

#### Scenario: A cockpit that hides a screen
- **WHEN** `measure_mfds.py` finds a screen less than 100 % visible in a roster cockpit
- **THEN** that cockpit is not done
