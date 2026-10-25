## ADDED Requirements

### Requirement: Fog lies low and fills the hollows
The view SHALL be fogged by a layer of air that is densest at sea level and thins with height, scaled by a mist rule that is strongest in hollows, near water, on the coast and on the crater floor and absent above 600 m.

#### Scenario: A valley is mistier than its ridge
- **WHEN** the mist is asked at the bottom of a river valley and on the ridge 200 m above it
- **THEN** the valley's mist is greater

#### Scenario: Fog is exact along a ray
- **WHEN** the fog's optical depth along a view ray is worked out in closed form
- **THEN** it matches a fine numerical march along the same ray within 1 %

### Requirement: It is thick under a storm's rain
Inside and near a rain shaft, the view SHALL be fogged more and greyer than in the open.

#### Scenario: Standing in the rain
- **WHEN** the eye is inside a rain shaft below its top
- **THEN** ground 3 km away is fogged at least half way to the fog's colour
