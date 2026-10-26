## ADDED Requirements

### Requirement: Driving a car is seen from a chase camera the player orbits

While the pilot drives a car, the camera SHALL sit behind and above the car, looking along the player's view from
the car's nose, at a distance that grows with the car's length; the mouse SHALL orbit it all the way round the car
and within a pitch range; it SHALL stop short of buildings and the ground; and its field of view SHALL be the
on-foot one.

#### Scenario: Driving away
- **WHEN** the pilot drives a sedan forward with the mouse still
- **THEN** the camera is behind the sedan and above its roof, about 8 m from its middle, at the on-foot field of view

#### Scenario: Looking round
- **WHEN** the player turns the mouse 150 degrees while driving
- **THEN** the camera has orbited 150 degrees round the car

#### Scenario: A wall behind the car
- **WHEN** a building stands between the car and where the camera would be
- **THEN** the camera stands on the car's side of the wall

### Requirement: The view drifts back behind a moving car after the mouse is still

With the mouse still for `recenter_after_s` while the car moves faster than `recenter_min_speed_m_s`, the view
SHALL drift back behind the car at `recenter_deg_s`; a parked car's view SHALL stay where the player left it.

#### Scenario: Letting go of the mouse
- **WHEN** the player has orbited the camera beside a car moving at 12 m/s and leaves the mouse still for 5 s
- **THEN** the camera is behind the car again

### Requirement: Getting in and out of a car never cuts the view

Getting in, the view SHALL be carried in first person to the driver's seat and then ease out to the chase camera;
getting out, it SHALL ease back into the pilot's eyes; neither SHALL cut or go black.

#### Scenario: Getting in
- **WHEN** the pilot presses Use at a parked car's door
- **THEN** the view moves from the pilot's eyes to the seat, then out to behind the car, with no jump between frames
