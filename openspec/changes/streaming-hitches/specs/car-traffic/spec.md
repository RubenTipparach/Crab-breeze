## ADDED Requirements

### Requirement: A car that has not moved is not redrawn
A drawn car's transform, and its wheels', SHALL be written only when the simulation moved it, so a parked or waiting
car costs the transform propagation nothing.

#### Scenario: Parked cars in a city
- **WHEN** the streaming route passes the capital
- **THEN** the cars' transforms written a frame fall from all of them (500 to 590) to the moving ones (360 to 380)
