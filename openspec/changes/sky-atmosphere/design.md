# Design: sky-atmosphere

- **The atmosphere:** one `Atmosphere::earth(ScatteringMedium::earth(256, 256))` entity. The
  pilot's camera gets `AtmosphereSettings::default()` (the lookup-texture method, the cheaper of
  Bevy's two) and an `Exposure`. Bevy requires HDR for it, and the water optics already turn HDR
  on; the atmosphere brings it either way.
- **The sun:** `DirectionalLight` at `lux::RAW_SUNLIGHT` (about 120,000 lux) with `SunDisk::EARTH`,
  so the atmosphere filters it (warmer when low).
- **Exposure:** `world.view.exposure_ev100` (13, Bevy's example value for raw sunlight), on the
  pilot's eyes only. The cockpit camera keeps its own lamps and exposure (cockpit-lighting).
- **Ambient:** the global ambient brightness rises by the exposure step from Bevy's default
  ev100 9.7 to 13 (2^3.3, about 10 times), so the shade side stays as light as before.
- **Fog:** clear-air visibility rises to 20 km. The atmosphere's aerial perspective does the
  hazing; the fog still thickens inside a cloud and under water as before.
- **Cost (expected):** the lookup tables are a few small compute passes a frame plus one
  full-screen sky pass, about 0.2-0.5 ms on the M4 Max (to be measured as `CB_NO_SKY=1`).

## Open questions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| S1 | Sun height and heading | keep today's (35 degrees up) | provisional |
| S2 | Atmosphere-driven ambient light (`AtmosphereEnvironmentMapLight`) | not yet; measure first | provisional |
