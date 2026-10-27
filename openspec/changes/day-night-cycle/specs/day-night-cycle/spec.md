## ADDED Requirements

### Requirement: Day and night take 24 minutes each
The world SHALL keep one clock, advanced in the fixed step, on which twelve hours of day (06:00 to 18:00) take 24 minutes and twelve hours of night take 24 minutes, and every hour-dependent rule (the sun, the moon, the exposure, the morning mist, the lights, the ambience) SHALL read that clock.

#### Scenario: A full cycle
- **WHEN** the clock runs for 48 minutes of fixed steps
- **THEN** it has advanced exactly 24 hours

#### Scenario: The mist follows the world's clock
- **WHEN** the world's clock passes 11:00 in free roam
- **THEN** the fog is the day's, as survey F1 sets it

### Requirement: The sun and the moon cross the sky
The sun SHALL rise due east at 06:00, peak at 60 degrees elevation due south at 12:00 and set due west at 18:00, and a full moon SHALL ride opposite it; whichever is above the horizon SHALL light the world and cast its shadows.

#### Scenario: Noon
- **WHEN** the clock reads 12:00
- **THEN** the sun is 60 degrees up, due south

#### Scenario: Midnight shadows
- **WHEN** the clock reads 00:00
- **THEN** the moon is the light that casts shadows, and the player's mech or car casts one

### Requirement: Cities light up at night
At night every street lamp's head, lit windows and every car's lights SHALL glow, and the lamps and car headlights nearest the eye SHALL light the surfaces round them, without shadows; only the player's own vehicle's lamps SHALL cast shadows among the night's lights.

#### Scenario: A street at midnight
- **WHEN** the pilot stands in a city street at 00:00
- **THEN** the street is lit in pools under its lamps and some windows above are lit

#### Scenario: No popping
- **WHEN** the eye moves so that a lamp enters or leaves the set of real lights
- **THEN** its light on the street fades rather than switching
