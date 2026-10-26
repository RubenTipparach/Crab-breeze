## ADDED Requirements

### Requirement: Free roam starts in any roster mech the player picks

The menu SHALL list every roster mech (each mech file with a cockpit interior) before free roam starts, and free
roam's pause menu SHALL list them too; picking one SHALL start free roam in that mech, with its model, its cockpit
and its boarding.

#### Scenario: Picking the Sapper on the menu
- **WHEN** the player chooses Free roam, then the Sapper
- **THEN** free roam starts with the Sapper standing at the start, and boarding it puts the pilot in the Twin tier

#### Scenario: Changing mech from the pause menu
- **WHEN** the player, in free roam in the Foreman, opens the pause menu's Mech page and picks the Mule
- **THEN** the game reloads and free roam starts again in the Mule

#### Scenario: The roster
- **WHEN** the roster is read from `data/mechs/`
- **THEN** it holds the eight roster mechs and neither the stand-in nor the retired Breeze
