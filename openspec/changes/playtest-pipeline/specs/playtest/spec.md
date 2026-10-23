## ADDED Requirements

### Requirement: Scripted pilot plays through player input
The game SHALL provide a scripted pilot, started with `--playtest <name>`, that drives the pilot
only by writing the same input actions a player's devices write.

#### Scenario: A walk beat moves the body like a key press
- **WHEN** a script's `WalkTo` beat runs toward a marker 10 m ahead on flat ground
- **THEN** the pilot's body reaches within 0.5 m of the marker through the on-foot movement system, and no system moves the body directly

### Requirement: First person throughout a playtest
A playtest SHALL render only from the pilot's eyes: the head on foot and the cockpit seat's eye point in the mech.

#### Scenario: Boarding keeps the first-person camera
- **WHEN** a `Board` beat completes
- **THEN** the camera is at the cockpit's `EYE_pilot` point and no other camera rendered during the beat

### Requirement: Output lines for the recorder
The game SHALL print `PLAYTEST_READY`, `PLAYTEST_START`, `PLAYTEST_BEAT`, `PLAYTEST_RESULT` and `PLAYTEST_END` lines on stdout, in that order, flushed as each happens.

#### Scenario: A stuck beat is reported
- **WHEN** a beat makes no progress for 10 s of game time
- **THEN** the game prints `PLAYTEST_RESULT stuck` naming the beat and the pilot's position, then `PLAYTEST_END`

### Requirement: Every recorded take reaches Google Drive
The playtest tool SHALL copy an H.264 share copy of every recorded take to the Drive for Desktop folder, named `crab-breeze-<YYYY-MM-DD>-<name>.mp4`, and write a report in `docs/playtests/`.

#### Scenario: A recorded run is uploaded and reported
- **WHEN** `tools/playtest.py boarding --record` completes with `PLAYTEST_RESULT ok`
- **THEN** the share copy is in the Drive folder and `docs/playtests/<date>-boarding.md` names the commit, the machine, each beat's outcome and the contact sheet
