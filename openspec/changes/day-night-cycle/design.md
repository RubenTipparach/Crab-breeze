# Design: day-night-cycle

What the game has today, measured or read from the source on 2026-10-27:

- **One fixed sun**, `cb_core::world::sun_shade::sun_rotation()` (yaw 0.8 rad, pitch -0.75 rad), lit at Bevy's raw
  sunlight under the scattering sky, with three live shadow cascades. A fixed exposure of `exposure_ev100` 13.4.
- **Baked from that sun:**
  - The terrain's far shadow, `SunShade` on a 40 m grid over the 40,960 m island (1,024 x 1,024 cells). It is
    multiplied into the far ground chunks' vertex colours (`ground.rs`), so it cannot move without remeshing them.
  - The clouds' shade map, 1,024 x 1,024 texels made on the CPU (`clouds_render::shade_map`).
  - The far facades, rendered from the near ones at startup (`far_facades.rs`).
- **The sky's light:** a Poly Haven day cube, filtered once (`sky_light.rs`), plus a flat `GlobalAmbientLight`.
- **No light sources in the world** but effects' flashes (`fx.rs`) and the cockpit's own lights (`mech.rs`).
- **The world (seed 42, `cb-tools cities`):** 13 cities, 600-960 m across, with 3,609 buildings.
- **The fog's clock** (`FogConfig::clock`): starts at 06:00 and runs at 0.00333 hours a second (one hour per
  300 s).

## 1. The clock

- **`cb_core::world::clock::WorldClock`:** hours since midnight, advanced in the fixed step by
  `hours_per_s` x dt.
  - `hours_per_s` is 1/120 (0.008333): one game hour every 120 s, so 12 hours of day (06:00-18:00) is 24
    minutes and 12 of night is 24.
  - Deterministic: the same step count gives the same hour.
  - `start_h` (provisional 07:00, survey N1).
- **It replaces the fog's own clock.** `FogConfig::clock` goes and the morning mist reads the world's hour. Its
  burn-off, 07:00-11:00, now passes in 8 minutes instead of 20; the survey F1 behaviour is kept, only faster.
- **Readers:**
  - The ambience's `night` flag (already wired in `cb_core::audio`).
  - Every light and look below.
  - Later: missions that set their hour (survey N7).
- **Data:** `data/world/day_night.ron`:
  - `clock: (start_h: 7.0, hours_per_s: 0.008333)`;
  - an Options override: `cycle | day | night` (survey N7).

## 2. The sun and the moon

- **The sun's path:** a day at the equinox at 30 degrees north.
  - The sun rises due east at 06:00, peaks at 60 degrees elevation due south at 12:00, and sets due west at
    18:00.
  - Its direction is the hour angle `h = (hour - 12) x 15 degrees` turned about the polar axis, tilted 30
    degrees from the zenith. It moves 15 degrees a game hour, which is 7.5 degrees a real minute.
  - Pure function `sun_dir(hour)` in `cb-core`, with tests: rises at 06:00, sets at 18:00, peaks at 12:00.
  - `sun_rotation()` becomes `sun_rotation(hour)`. Every reader passes the hour; there is one sun.
- **The moon:** real phases over 29.53 days (survey N2, owner 2026-10-28: "real moon phases", against the
  recommended full moon every night). It trails the sun by its phase of a day: a full moon is opposite the sun
  (up at 18:00, down at 06:00), a new one rides with it, so some nights are black. The first night is full
  (`moon_start_phase` 0.5), so a new player's first night can be seen. Its light is the full moon's times the
  lit share of its face, `(1 - cos(2 pi phase)) / 2`. Its disc is drawn by the fog pass with its phase lit on the
  sunward side (Bevy's `SunDisk` would draw it whole).
  - A second directional light at `moon_lux`, with `SunDisk` for its disc (Bevy's atmosphere draws a disc for
    each directional light that has one).
  - Real full moonlight is 0.25 lux, against about 100,000 for the sun.
  - Its value is set with the night exposure (section 3) so a moonlit field reads dark blue-grey at about 12 %
    of a day's brightness on screen: provisional `moon_lux: 4.0` against a night exposure 13 EV below day's.
    The ratio is what counts.
- **Shadows:** Bevy draws cascades for each shadowed directional light, so only one casts at a time.
  - The sun casts while it is above the horizon, the moon while the sun is below it.
  - The switch is at the horizon: both are near zero there (the sun's light fades to zero over its last 3
    degrees, the moon's rises over its first 3), so it cannot be seen.
  - Same cascades, same cost as today.
- **Stars:** drawn in the fog pass on sky pixels, which it already visits.
  - A direction hashed onto a cube of cells gives about 2,000 stars brighter than a threshold, with twinkle.
  - They are faded by the sky's own brightness from the sky-view LUT, so they show from about 30 minutes of game
    time after sunset (the sun 8 degrees down).
  - Clouds draw over them.
  - Cost: one hash a sky pixel.
- **The sky's light at night:** scaled by a curve of the sun's height to about 2 % at night, with a bluish
  ambient from the moon. The day cube stays: it is near black at night, and reflections at night are of
  lights, through the water's screen-space reflections.
- **Clouds** take the light from whichever body is up: one key-light direction, colour and strength
  (`water_light` already reads "the sun"; it reads the key light instead).
  - Over a city at night, cloud bases take an orange glow from below: a city-glow term in the cloud shader
    from the city lamps' count within 2 km. Data: `city_glow`.

## 3. Exposure

The exposure follows a curve of the sun's height in data:

`ev100(sun_deg) = lerp(night_ev100, day_ev100, smoothstep(-8, 6, sun_deg))`

- `day_ev100` 13.4 (today's value), `night_ev100` 0.4 (13 EV down).
- It is a function of the clock, not of the image: no histogram, no adaptation lag, the same frame every run.
- The cockpit's camera keeps its own exposure: its screens are emissive and read the same day and night.
- The sun flare, bloom and the sky's light already scale with exposure (`sun_flare.rs`, `sky_light.rs`).

## 4. The far terrain shadow at any hour: a horizon map

The ground's far shadow is in its vertex colours, made for one sun. A moving sun would rebuild every far chunk's
colours every few seconds, which is too much remeshing. Instead:

- **Bake once at load a horizon map:** for each cell of an 80 m grid (512 x 512 over the island) and each of 16
  compass directions, the elevation angle of the highest ground seen that way within 8 km, as u8 (0-90 degrees in
  0.35-degree steps).
  - 512 x 512 x 16 = 4 MB on the GPU as a 16-layer R8 array.
  - Bake: 262,144 cells x 16 directions x 64 steps = 268 million height reads, about 1 s on eight threads.
    Deterministic, so it can be cached to disk by seed.
- **The far ground's shader shades by it:** read the two compass layers on each side of the sun's (or moon's)
  azimuth, interpolate, then `lit = smoothstep(horizon - soft, horizon + soft, sun_elevation)` with `soft` 1.5
  degrees. The same `shade_albedo` dims what is in shade.
  - The vertex colours lose the shadow term.
  - `SunShade` becomes the horizon map. Its users (`ground.rs`, the grass's overlook start) ask it for a sun
    direction.
  - Near ground keeps the live cascades, as now.
- Cost: two texture reads a far-ground pixel.

## 5. Lights: forward+, three tiers

**Forward+ (Bevy's clustered forward), not deferred** (owner: "I'll let you pick"; survey N8):
- Every surface in the game that matters at night has its own forward material: buildings (texture-array
  shader), ground, grass, water, glass, clouds and the cockpit. Going deferred means a deferred variant of each,
  and glass, water and clouds would stay forward anyway.
- Deferred's win is very many lights over a lot of overdraw. Here the lights that light surfaces are capped
  near the eye (tier 1), and the rest are emissive (tiers 2 and 3), which costs nothing in the lighting.
- Bevy's clustered forward assigns lights to view-space clusters each frame, on the CPU, by count. Under 150
  lights that is cheap; 6,000 would not be. Hence the cap.
- If the owner's machine shows pixel shading dominating, deferred is the change to measure next.

**The lamps (`cb_core::world::lamps`), one rule from the road network:**
- Every stretch of road is lit (owner, 2026-10-28: "Every stretch of road should have street lamps"): city
  streets get a lamp every 32 m, alternating sides; arterials one every 30 m on both sides; highways one every
  50 m, alternating sides, from city to city (the first design left highways dark away from cities).
- Air bases get runway edge lights every 60 m, apron floodlights and a rotating beacon.
- Each lamp has a position, a head height (9 m streets, 12 m arterials), a kind and a stable id. It is built
  with the roads and validated: no lamp in a building, none in water.
- Estimated about 5,000 lamps (13 cities, about 25 km of street a square kilometre of city); the build counts
  them.
- Sodium 2,000 K orange (survey N5), 15,000 lm.

**Tier 1, real lights:**
- The nearest `real_lamps` (96) lamps within `real_lamp_m` (250 m) of the eye are Bevy `SpotLight`s: pointing
  down, 65-degree outer cone, 35 m range, no shadows.
- Chosen each frame from a 64 m grid of lamps, in stable order (distance, then id).
- Each one's intensity fades over its last 50 m of the real range, and one newly admitted fades in over 0.5 s,
  so nothing pops.
- A pooled set of 96 light entities is moved, never spawned per frame.

**Tier 2, emissive heads:** every lamp's head is an emissive part of an instanced pole mesh (pole and head about
200 triangles). On at dusk (the sun 2 degrees down), off at dawn. One draw per kind of pole with instancing.

**Tier 3, glow:** beyond 150 m each lit head also carries a glow sprite (`soft_sprites`, additive), sized in
screen pixels with a floor of 2 px, so a city reads as a grid of lights from the air, and bloom spreads it.

**Far light pools on roads** (survey N4): not in the first build.

## 6. Lit windows

- **Lit or dark:** a window is lit by a rule of its own: a hash of the building's id and the window's cell (its
  storey and its column on the facade, from the glass fragment's position against the building's storey height
  and window pitch).
- **Each window has a switch-on hour and a switch-off hour,** drawn from data curves, so the share lit goes:
  - 0 % by day;
  - 65 % at 20:00 (lights come on through dusk, 17:30-20:00);
  - 35 % at midnight;
  - 12 % at 03:00 (some all night: night shifts, stairwells).
- **Colours:** 70 % warm incandescent (2,700 K), 25 % office fluorescent (4,000 K) and 5 % flickering TV blue.
- **Drawn as glass emission** in the glass material (one hash a glass pixel, no extra draw). Behind it the
  interior (`building-interiors`), where one is shown, takes a matching flat light.
- **Far facades** gain a night layer: the same rule rendered into an emission map at startup per building type,
  a time-of-day threshold in its alpha so the share lit follows the clock without rebaking.
  - Memory: one 1,024 px RGBA8 layer with mips per type, about 5.6 MB each.
- **A wrecked building goes dark** (owner, 2026-10-28: "buildings also need some sort of cell lighting. If the
  building is damaged all of its lights turn off"; 2026-10-29: "damaged building should stay lit until the player
  destroys about 30% of the building", then "our 50% building destruction rule for switching off lights"): once `structure.lights_out_share` (0.5, data) of its frame's elements have
  fallen (`Frame::lit`), none of its windows lights again; a building hit less than that keeps its windows. The rule
  rides the element state the shaders already read: bit 2 of every word of the building's range is set once it is
  dark (`building_state::standing_words`), bit 1 is still the element's own standing.
- One implementation: `cb_core::world::windows::lit(building, cell, hour)` is the rule. The glass shader reads
  it as a WESL copy pinned by a test against the core on a table of cases. The far facade bake calls the core.

## 7. Vehicles

- **Every car** carries emissive headlights and tail lights, on from dusk (the same switch as the lamps). The
  nearest `real_headlights` (12) cars within 150 m get a pair of `SpotLight`s each (25-degree cone, 60 m range,
  no shadows), pooled like the lamps. Traffic lights are emissive already.
- **The player's own vehicle** (the mech, or the car being driven) carries lamps that cast shadows: the mech's
  two chest floodlights and the car's headlights. These are the only night lights with shadow maps (owner: "no
  shadows from night lights, except for on whatever vehicle we have"; survey N3).
  - Two spot shadow maps at 1,024 px, about 0.3-0.6 ms on a 3070 (estimate).
  - The pilot switches them on and off with a bindable control (survey C2).
  - The mech or car itself also casts its shadow from the moon, as everything does.
- **Street lamps cast no shadows.** If the owner meant the player's vehicle should cast shadows from the street
  lamps it passes, that needs a light whose shadow map holds only that vehicle. Bevy 0.20 has no such filter;
  whether a light's `RenderLayers` limit its shadow casters is to be read in the source before it is offered
  (task 4.4).

## 8. Costs (estimates; measured on the owner's machine, section 7 of CLAUDE.md)

| Item | GPU | CPU | Memory |
| --- | --- | --- | --- |
| 96 real lamp lights, no shadows | 0.5-1.5 ms at 1080p (3-6 lights a pixel in a street) | 0.1-0.3 ms clustering | none |
| 12 cars' headlights | 0.1-0.3 ms | small | none |
| Player's lamps, 2 shadowed spots | 0.3-0.6 ms | casters within 60 m | 8 MB |
| Moon shadows | as the sun's today (swapped, not added) | same | same |
| Lamp poles, about 5,000 instanced | under 0.2 ms (about 1 M triangles at full detail before culling) | 1 draw per kind | small |
| Glow sprites | under 0.2 ms | rebuilt as the eye moves | small |
| Stars | under 0.05 ms | none | none |
| Horizon map | 2 reads a far-ground pixel | 1 s bake at load (cacheable) | 4 MB |
| Window rule | 1 hash a glass pixel | none | far facade night layers, about 5.6 MB a type |
| Cloud shade rebake | upload 1 MB every 16 s | one 1,024 x 1,024 bake on a worker | 2 MB (two maps to crossfade) |

Total at night in a city, estimated: 1.5-3 ms GPU over today's day frame, inside the 8.3 ms p95 target only if
today's city frame leaves that headroom. That is the first measurement (task 6.2).

## Open questions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| N1 | Start hour | 07:00 | open (survey) |
| N2 | Moon | full every night, playable without lights | open (survey) |
| N3 | Night shadows | moon on everything; the player's own vehicle lamps; street lamps none | open (survey) |
| N4 | Far road light pools | not at first | open (survey) |
| N5 | Lamp colour | sodium orange, mercury white at air bases | open (survey) |
| N6 | Night changes play | fewer cars and people after 23:00; sensors unchanged | open (survey) |
| N7 | Fixing the hour | Options: cycle, always day, always night; missions may set an hour | open (survey) |
| N8 | Forward+ or deferred | forward+ with a near-eye light cap | provisional (section 5) |
