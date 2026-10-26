## ADDED Requirements

### Requirement: Continue restores where the player was
A free-roam game and a career SHALL save where the pilot and their mech are, the mech's damage and its missiles, every 30 s, when the pause menu opens and when the game quits, and Continue SHALL restore them.

#### Scenario: Quitting and continuing free roam
- **WHEN** the player walks 200 m from the start in free roam, quits, and picks Continue then Free roam
- **THEN** the pilot stands where they were, facing where they faced, and the mech stands where they left it with the damage it had

#### Scenario: A broken save
- **WHEN** the save file does not parse
- **THEN** it is set aside as `.bad`, the log says so, and the game starts fresh
