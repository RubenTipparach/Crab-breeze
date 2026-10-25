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
