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

### Requirement: Morning mist thins by day, and combat-heavy missions fly in clear air
The fog SHALL begin each world as a morning mist and thin to a day's fog on the fog's own clock, and while a mission with its own preset runs (the city defence and the air fortress) the fog SHALL be that preset's, clear air with no mist (owner, 2026-10-26, survey F1: "Approve all").

#### Scenario: The mist burns off
- **WHEN** the fog's clock runs from the world's start through the morning
- **THEN** the fog never thickens, and by 11:00 it is the day's

#### Scenario: A defence at dawn
- **WHEN** the city defence runs at 06:00 on the fog's clock
- **THEN** the fog is the clear preset, with no mist
