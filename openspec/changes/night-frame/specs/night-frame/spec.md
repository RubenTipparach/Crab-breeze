## ADDED Requirements

### Requirement: The night holds the frame budget

At midnight in the densest city scene the game SHALL hold the same frame budget as at noon: every frame of the scripted
perf routes after `GAME_READY` within 16.7 ms on the owner's machine.

#### Scenario: Midnight in the capital

- **WHEN** `city-stand` plays with the clock held at midnight, in the window, vsync off
- **THEN** no frame after `GAME_READY` takes longer than 16.7 ms
