## ADDED Requirements

### Requirement: The pilot drives a car from its driver's seat

The pilot SHALL get into a stopped, parked car with Use at its driver door, SHALL see the car from
its chase camera while driving (`car-chase-camera`, the law's one exception), and SHALL keep control of
where they look while getting in, driving and getting out.

#### Scenario: Getting in
- **WHEN** the pilot stands at a parked sedan's driver door and presses Use
- **THEN** the view is carried to the driver's seat, the mouse still turns the view, and W drives the car

### Requirement: The dashboard shows the simulation's numbers

The car's speedometer SHALL be driven from the speed the car simulation uses.

#### Scenario: Reading the speedometer
- **WHEN** the car's simulated speed is 50 km/h
- **THEN** the speedometer needle reads 50 km/h
