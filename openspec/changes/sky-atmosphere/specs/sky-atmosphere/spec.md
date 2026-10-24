## ADDED Requirements

### Requirement: A scattering sky
The pilot's view SHALL draw the sky with Bevy's physically based atmosphere, darker overhead than at the horizon, unless `CB_NO_SKY=1` is set.

#### Scenario: Looking up and level
- **WHEN** the pilot looks straight up and then level at the horizon in clear air
- **THEN** the sky straight up is a deeper blue than the sky at the horizon
