## ADDED Requirements

### Requirement: Escape pauses the game and opens the pause menu

Pressing Escape in play SHALL pause the simulation and show a pause menu with Resume, Controls and
Quit; Resume SHALL return to play where it stopped.

#### Scenario: Pausing mid-fight
- **WHEN** the pilot presses Escape during a wave
- **THEN** the simulation stops, the mouse is released and the pause menu shows
- **AND** Resume continues the wave from the same moment

### Requirement: The Controls page lists every control from the bindings

The Controls page SHALL list every control the game reads, with the keys bound to it in the bindings
data, grouped by where it is used.

#### Scenario: A rebound key shows
- **WHEN** `free_look` is bound to Left Ctrl in `data/input/bindings.ron`
- **THEN** the Controls page shows "Free look (hold)" as Left Ctrl

### Requirement: Options sets sound effects and music volume and saves each change

The Options page SHALL set the sound effects volume and the music volume, each 0 to 100 % in steps
of 10 %, applied at once, and SHALL save the settings file on every change.

#### Scenario: Turning the music down
- **WHEN** the player presses - on Music twice from 100 %
- **THEN** the music plays at 80 % of its level and `settings.ron` holds `music_volume: 0.8`

#### Scenario: An older settings file
- **WHEN** `settings.ron` has no volume fields
- **THEN** it loads with both volumes at 100 %
