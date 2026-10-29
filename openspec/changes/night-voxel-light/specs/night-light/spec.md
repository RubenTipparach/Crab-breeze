## ADDED Requirements

### Requirement: Lamp and window light is baked onto the static world at night
The game SHALL bake each city's street lamp light and lit window light into a 2-channel vertex attribute on the static meshes of that city, by a summing diffusion through the air voxels of the city's occupancy, and the materials of those meshes SHALL add it to their lit colour scaled by the lamps' dusk switch and the hour's lit window share.

#### Scenario: A lamp far from the eye lights the street under it
- **WHEN** it is 00:00 and the eye is 1,500 m above a city
- **THEN** the street under every lamp of that city is lit in a pool, not only the lamps within 250 m of the eye

#### Scenario: Light does not cross a wall
- **WHEN** a lamp stands on one side of a building
- **THEN** the street directly behind the building, out of the lamp's reach round its corners, gets none of that lamp's light

#### Scenario: Light turns a corner
- **WHEN** a point in the street is round a corner from a lamp, at the same distance as a point in the lamp's sight
- **THEN** it is lit, and less than the point in sight

#### Scenario: Nothing shows by day
- **WHEN** the lamps are off and no window is lit
- **THEN** the baked term adds nothing to any surface

### Requirement: The bake is deterministic
The bake SHALL depend only on the city's plan and the bake's data, so the same city gives the same light on every vertex in every run, whatever order the sources are given in.

#### Scenario: Two bakes agree
- **WHEN** one city is baked twice, with its lamps listed in two different orders
- **THEN** both bakes give the same bytes

### Requirement: A damaged building's light is rebaked
When a building is damaged, the game SHALL rebake the light within its windows' reach and replace the baked light on the meshes there, so its windows' light leaves the street and its neighbours.

#### Scenario: A building is shot at night
- **WHEN** a building with lit windows is damaged at 22:00
- **THEN** its windows go dark at once, and once the rebake lands, the facing building no longer carries its window light
