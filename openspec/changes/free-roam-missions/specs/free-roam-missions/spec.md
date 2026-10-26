## ADDED Requirements

### Requirement: Free roam has mail that starts missions
Free roam SHALL have an inbox, opened with the mail key, holding one message for each mission, each with a button that starts that mission.

#### Scenario: Starting the fortress from the mail
- **WHEN** the pilot in free roam opens the mail and presses Start on the Stratostacean's message
- **THEN** the crab air fortress mission begins over the city nearest the pilot

### Requirement: One mission runs at a time
Starting a mission SHALL turn off the mission running, removing its enemies and stopping its clock and readout, before the new one begins; stopping a mission SHALL leave none running.

#### Scenario: Starting another mission
- **WHEN** the city defence is running and the pilot starts the Stratostacean
- **THEN** the defence's raiders and vehicles leave the world and only the fortress mission runs

#### Scenario: Stopping
- **WHEN** the pilot presses Stop on the running mission's message
- **THEN** no mission runs and its enemies leave the world

#### Scenario: What a stopped mission broke
- **WHEN** the fortress mission is turned off after its rounds have broken buildings and while its rounds are still in the air
- **THEN** its enemies and their rounds in the air leave the world in that frame, and the broken buildings stay broken

### Requirement: Accepting a job abandons the job under way
In a career, accepting a job offer while another job is accepted or active SHALL abandon that job, unpaid, and take its enemies away.

#### Scenario: A second job
- **WHEN** a job is under way and the pilot accepts another offer
- **THEN** the first job is abandoned with no pay and the new one is accepted
