# Tasks: day-night-cycle

Written up 2026-10-27; nothing built. Building starts on the owner's go (CLAUDE.md section 4), on the survey's
answers or its recommendations.

## 1. The clock, the sun and the moon

- [ ] 1.1 `cb_core::world::clock::WorldClock` in the fixed step from `data/world/day_night.ron`; the fog's
  `FogConfig::clock` removed and the morning mist reads the world's hour (tests: an hour is 120 s, the mist burns
  off by 11:00, the same steps give the same hour)
- [ ] 1.2 `sun_dir(hour)` and `moon_dir(hour)` in `cb-core` (tests: sunrise 06:00 due east, noon 60 degrees due
  south, sunset 18:00 due west, the moon opposite); `sun_rotation()` takes the hour at every reader
- [ ] 1.3 The sun and the moon as two directional lights, the one above the horizon shadowed, each faded over 3
  degrees at the horizon; the moon's disc
- [ ] 1.4 Exposure from the curve of the sun's height (section 3); the cockpit camera's own exposure unchanged
- [ ] 1.5 The sky's light and the flat ambient scaled by the sun's height; clouds, water and the fog take the key
  light, sun or moon
- [ ] 1.6 Stars on sky pixels in the fog pass, faded by the sky's brightness

## 2. What was baked from one sun

- [ ] 2.1 The horizon map (section 4) baked at load on worker threads, cached by seed; the far ground shades by
  it; the vertex colours lose the shadow (test: a valley behind a ridge is in shade at sunset and lit at noon)
- [ ] 2.2 The clouds' shade map rebaked every 2 degrees of sun travel on a worker, crossfaded
- [ ] 2.3 The far facades' night layer (section 6)

## 3. Lamps

- [ ] 3.1 `cb_core::world::lamps` from the road network and the air bases (tests: spacing by road kind, none in a
  building or in water, stable ids); `cb-tools cities` prints the count
- [ ] 3.2 Instanced poles and emissive heads, on at dusk
- [ ] 3.3 Tier 1: the nearest 96 within 250 m as pooled spot lights, stable order, distance and admission fades
  (test: the chosen set is the same for the same eye and lamps, whatever the query order)
- [ ] 3.4 Tier 3: glow sprites beyond 150 m

## 4. Windows and vehicles

- [ ] 4.1 `cb_core::world::windows::lit` and its WESL copy, pinned by a test on a table of cases; the glass shader's
  emission; interiors behind lit windows
- [ ] 4.2 Cars' emissive lights at dusk; the nearest 12 cars' headlights as pooled spot lights
- [ ] 4.3 The player's vehicle lamps with shadows, on a bindable control
- [ ] 4.4 Read Bevy 0.20's source on whether a light's `RenderLayers` limit its shadow casters (needed only if the
  owner wants the vehicle's shadow from street lamps, survey N3)

## 5. Survey

- [x] 5.1 N1-N8 on the survey page, with the recommendations above (2026-10-27)

## 6. Proof

- [ ] 6.1 Headless stills (lavapipe): noon, sunset, dusk, midnight and dawn over a city, from the street, from the
  cockpit, and from 1,500 m; a contact sheet looked at
- [ ] 6.2 Measured on the owner's machine: a city at midnight against the same take at noon, p50, p95 and p99
- [ ] 6.3 Playtest: a full 48-minute cycle on the streaming route, recorded, on Drive
