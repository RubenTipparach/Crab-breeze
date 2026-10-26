## ADDED Requirements

### Requirement: The scope magnifies in a window, not the pilot's view
While Scope is held with a scoped weapon on board, the cockpit SHALL show a window on the front screen rendering the view down the aim at the selected magnification, and the pilot's own view SHALL keep its field of view.

#### Scenario: Scoping at 4 times
- **WHEN** the pilot holds Scope with a railgun whose `scope_zoom` is [4.0, 10.0]
- **THEN** the window shows the aim at a 25.8 degree field of view and the pilot's view stays at 85 degrees (with the seated view at 95 degrees, the window is 30.5 degrees)

### Requirement: Scoped look is slowed, never driven
While scoped, mouse look SHALL be scaled by one over the magnification, and nothing but the player's input SHALL turn the view.

#### Scenario: A mouse sweep at 10 times
- **WHEN** the pilot sweeps the mouse a distance that turns the view 20 degrees unscoped, while scoped at 10 times
- **THEN** the view turns 2 degrees

### Requirement: The drop mark is where the slug lands
The scope's drop mark SHALL be computed by `weapons::lead` with drop, the function the railgun's slug flies by.

#### Scenario: A still target at 2 km
- **WHEN** the selected target stands still 2 000 m away and the railgun fires at 2 400 m/s
- **THEN** the drop mark sits 3.4 m above the target, and a slug fired with the reticle on the mark strikes the target
