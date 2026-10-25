## ADDED Requirements

### Requirement: The world has ambience where it is quiet
The game SHALL play stereo ambience beds chosen by where the listener is (city, industrial district, forest, open ground), crossfading between them as the listener moves, under the music and under every other sound.

#### Scenario: Walking into a city
- **WHEN** the pilot walks from open ground into a city's streets
- **THEN** the meadow bed fades out and the city street bed fades in over a few seconds, with no gap and no cut

### Requirement: The cockpit is windier than the street
The wind bed SHALL rise with the listener's height above the ground, and every bed SHALL be muffled through the sealed cockpit like any other outside sound.

#### Scenario: Climbing into the seat
- **WHEN** the pilot climbs from the ground into the cockpit 24 m up
- **THEN** the wind bed rises by about 12 dB and the beds are low passed as the hatch seals

### Requirement: Night and rain beds wait for a time of day and weather
The night and rain beds SHALL NOT be chosen while the game has no time of day or weather.

#### Scenario: Today's game
- **WHEN** the game runs with no time of day or weather system
- **THEN** only the meadow, forest, wind, city street and industrial beds play
