# Tasks: day-night-cycle

Written up 2026-10-27. Survey N1-N8 answered 2026-10-28 ("All questions done go"): every recommendation but N2,
where the owner chose real moon phases. Building from 2026-10-28.

## 1. The clock, the sun and the moon

- [x] 1.1 `cb_core::day_night::WorldClock` in the fixed step from `data/world/day_night.ron`; the fog's
  `FogConfig::clock` removed and the morning mist reads the world's hour (tests: an hour is 120 s, the mist burns
  off by 11:00, the same steps give the same hour)
- [x] 1.2 `sun_dir(hour)` and `moon_dir(hour, phase)` in `cb-core`, the moon's phases (survey N2) (tests: sunrise 06:00 due east, noon 60 degrees due
  south, sunset 18:00 due west, the moon opposite); `sun_rotation()` takes the hour at every reader
- [x] 1.3 The sun and the moon as two directional lights, the one above the horizon shadowed, each faded over 3
  degrees at the horizon; the moon's disc
- [x] 1.4 Exposure from the curve of the sun's height (section 3); the cockpit camera's own exposure unchanged
- [x] 1.5 The sky's light and the flat ambient scaled by the sun's height; clouds, water and the fog take the key
  light, sun or moon
- [x] 1.6 Stars on sky pixels in the fog pass, faded by the sun's depth below the horizon; the moon's disc with its phase

## 2. What was baked from one sun

- [ ] 2.1 (Until it is built, far ground has no baked shadow: one baked for a fixed sun pointed the wrong way from the first hour.) The horizon map (section 4) baked at load on worker threads, cached by seed; the far ground shades by
  it; the vertex colours lose the shadow (test: a valley behind a ridge is in shade at sunset and lit at noon)
- [ ] 2.2 The clouds' shade map rebaked every 2 degrees of sun travel (done, on the main thread; the move to a worker and the crossfade are left)
- [x] 2.3 The far and massing walls light their windows by the same rule as the glass (`building_windows.wesl`), a window inside each cell, the wall between dark, in place of a baked night layer

## 3. Lamps

- [x] 3.1 `cb_core::world::lamps::place` from every road (owner: "Every stretch of road should have street lamps"): city streets, arterials and the highways between cities; none in a building or in water, stable ids (tests: spacing by road kind, both sides on arterials, none on the squares where streets cross). Seed 42: 5,241 lamps (2,297 streets, 2,136 arterials, 808 highways), printed by `cb-tools cities`
- [x] 3.2 Poles and heads merged into one mesh per kilometre tile (two draws a tile, not instancing), the heads glowing from dusk
- [x] 3.3 Tier 1: the nearest 96 within 250 m as a pool of spot lights moved from lamp to lamp, nearest first with ties by id, fading in over 0.5 s and out over their last 50 m
- [x] 3.4 Tier 3: every lamp a glow quad sized on screen (at least 3 px), fading in from 150 to 300 m (`lamp_glow.wesl`); from 1,500 m a city reads as a grid of lights and the highways as dotted lines (`docs/screenshots/2026-10-28-city-lights-at-night.png`)

## 4. Windows and vehicles

- [x] 4.1 Lit windows in the glass shader (`building_surface.wesl`): each window a cell one storey high and 2.5 m wide, lit when its hash falls under the share of the hour (`WindowLights::lit_share`, tested); a damaged building, drawn by its frame with its own element slots, never lights (owner: "If the building is damaged all of its lights turn off"). Left: far facades' night layer (2.3) and lit interiors behind lit windows
- [ ] 4.5 The houses and small businesses (`small_buildings`, lot-access) light their windows by the same rule; their glass materials hold zero for now
- [ ] 4.2 Cars' emissive lights at dusk; the nearest 12 cars' headlights as pooled spot lights
- [ ] 4.3 The player's vehicle lamps with shadows, on a bindable control
- [ ] 4.4 Read Bevy 0.20's source on whether a light's `RenderLayers` limit its shadow casters (needed only if the
  owner wants the vehicle's shadow from street lamps, survey N3)

- [x] 4.6 Free roam's pause menu picks the time of day and freezes it (owner, 2026-10-28: "in free roam there is a toggle to pick time of day, or stop changinign time of day (freeze ToD)", "this is in the pause menu"; it stands in for survey N7's Options setting): TIME OF DAY with the hour, an hour back or on, Freeze or Run, and Dawn, Noon, Dusk and Night (`WorldClock::frozen`, `set_hour`, `shift`, tested)

## 5. Survey

- [x] 5.1 N1-N8 on the survey page, with the recommendations above (2026-10-27)

## 6. Proof

- [ ] 6.1 Headless stills (lavapipe): noon, sunset, dusk, midnight and dawn over a city, from the street, from the
  cockpit, and from 1,500 m; a contact sheet looked at
- [ ] 6.2 Measured on the owner's machine: a city at midnight against the same take at noon, p50, p95 and p99
- [ ] 6.3 Playtest: a full 48-minute cycle on the streaming route, recorded, on Drive
- [x] 6.4 Night's flat ambient raised to 16 % of the day's and the sky's light to 9 % (were 10 and 6; owner, 2026-10-29: "we need to bump up ambient color a bit, at night its too dark")
