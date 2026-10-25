## ADDED Requirements

### Requirement: The pilot drives a car from its driver's seat

The pilot SHALL get into a stopped, parked car with Use at its driver door, SHALL see from the
driver's seat while driving, and SHALL keep control of where they look while getting in, driving and
getting out.

#### Scenario: Getting in
- **WHEN** the pilot stands at a parked sedan's driver door and presses Use
- **THEN** the view is carried to the driver's seat, the mouse still turns the view, and W drives the car

### Requirement: The dashboard shows the simulation's numbers

The car's speedometer and rev counter SHALL be driven from the speed and engine speed the car
simulation uses.

#### Scenario: Reading the speedometer
- **WHEN** the car's simulated speed is 50 km/h
- **THEN** the speedometer needle reads 50 km/h
