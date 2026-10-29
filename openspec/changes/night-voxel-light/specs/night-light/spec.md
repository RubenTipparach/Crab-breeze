## ADDED Requirements

### Requirement: Lamp and window light is baked onto the static world at night
The game SHALL bake each city's street lamp light and lit window light into a 2-channel vertex attribute on the static meshes of that city (lamp light and window spill by a shortest path through the air voxels of the city, a room's own vertices by their distance from the room's light point), and the materials of those meshes SHALL add it to their lit colour scaled by the lamps' dusk switch and the hour's lit window share.

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

### Requirement: A lit window is a lit room, not glowing glass
At night the window glass SHALL not glow; a window SHALL read as lit by the room behind it being lit (its ceiling fitting on and its surfaces lit by the room's baked light within 250 m, an interior-mapped lit room past it), by the per-room lit rule, and a dark room SHALL stay dark inside.

#### Scenario: Looking into an office at 21:00
- **WHEN** the pilot stands in the street 40 m from an office whose room is lit
- **THEN** through the clear glass they see the ceiling fitting on and the desks and walls lit, and the glass itself adds no light

#### Scenario: A dark room beside a lit one
- **WHEN** one room is lit and the room beside it is not
- **THEN** the dark room's walls and furniture get no room light

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

### Requirement: The airport and the seaport are lit every night
The airport's and seaport's buildings and ships SHALL light all their rooms from dusk to dawn, a control tower's cab SHALL be lit day and night, their masts and floodlights SHALL stand where the approved pages put them, and every runway SHALL carry its edge, centre line, threshold and end lights as glow dots that light nothing.

#### Scenario: A site's rooms are always lit at night
- **WHEN** the glass's night share is above zero
- **THEN** every room of a site building or ship carries the full night share of its room light, with no hash and no hourly share

#### Scenario: A tower's cab is always on
- **WHEN** it is day or night
- **THEN** the rooms of a control tower's cab glow at `cab_gain` times a lit room

#### Scenario: The masts come from the page
- **WHEN** the sites are exported from their approved pages
- **THEN** each site's table lists the page's masts, floodlights, dots and runways, and the game places a lamp for each mast at the same point of the site's frame

#### Scenario: A damaged ship goes dark
- **WHEN** a ship or a crane is damaged
- **THEN** the lights standing on it go out

