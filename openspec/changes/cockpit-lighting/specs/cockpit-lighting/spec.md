## ADDED Requirements

### Requirement: The world's lights stay outside
While the cockpit pass draws, no light of the outside world (the sun, a muzzle flash, an explosion) SHALL light the cockpit.

#### Scenario: Firing the cannon
- **WHEN** the pilot fires the 120 mm cannon from the seat with the hatch shut
- **THEN** the muzzle flash lights the world outside and not the cockpit's panels

### Requirement: The screens light the cabin
The cockpit SHALL be lit by its own cabin ambient and by the glow of its screens.

#### Scenario: Seated at noon
- **WHEN** the pilot sits sealed in the cockpit with the sun high
- **THEN** the panels are lit by the screens and the cabin ambient, and the sun's direction leaves no mark on them
