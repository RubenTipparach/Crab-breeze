## ADDED Requirements

### Requirement: The menu and the game are one program
The game SHALL open on its main menu with no world loaded, start the picked mode in the same window and process, and return to the main menu when the player quits from the game.

#### Scenario: Starting free roam
- **WHEN** the player picks Free roam on the main menu
- **THEN** the world loads behind the loading screen in the same window, and no second process starts

#### Scenario: Quitting to the menu
- **WHEN** the player picks Quit on the pause menu
- **THEN** the game saves, the world is taken down and the main menu shows in the same window

### Requirement: A mech picked in free roam is swapped in place
Picking another mech on free roam's Mech page SHALL replace the current mech where it stands, without restarting the program or reloading the world.

#### Scenario: Picking the Stevedore
- **WHEN** the player, in free roam in the Foreman, picks the Stevedore on the Mech page
- **THEN** the Foreman is gone, the Stevedore kneels where it stood with its hatch open, the pilot stands beside it, and the window never closes
