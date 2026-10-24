## ADDED Requirements

### Requirement: Clouds from the seed
The sky SHALL hold cloud clusters placed from the seed between the cloud base and top in data.

#### Scenario: Same sky twice
- **WHEN** the world is generated twice with seed 42
- **THEN** the cloud clusters are identical

### Requirement: Flying into a cloud
When the pilot's view is inside a cloud, the view SHALL fog in proportion to how deep inside it is, read from the one cloud density query.

#### Scenario: Entering a cloud
- **WHEN** the mech's sensor head moves from clear air to 50 m inside a cloud
- **THEN** the visibility falls from the clear-air distance to under 100 m
