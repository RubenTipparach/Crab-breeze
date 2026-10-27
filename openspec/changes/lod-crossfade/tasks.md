# Tasks: lod-crossfade

- [ ] 1.1 `cb-app::lod_fade::{range, levels}` with headless tests (margins touch, never inverted, zero widths abrupt)
- [ ] 1.2 Aircraft and the fortress onto the helper, same numbers
- [ ] 2.1 Trees: full and far versions overlap across the 60 m ring at 600 m; tiles fade out over the last 150 m to 2 km
- [ ] 2.2 Cars fade in and out (1 km, 1.2 km); people (400 m, 600 m); traffic lights (700 m); widths in data
- [ ] 2.3 Check Bevy 0.20 re-reads a changed `VisibilityRange`; a car spawned in view fades in over 0.5 s
- [ ] 3.1 Buildings: the far look and the near detail overlap across a 40 m ring
- [ ] 3.2 `ArrayState` and `ElementState` fragment shaders call `visibility_range_dither`
- [ ] 4.1 Measured on the 3070: `streaming-route`, release, before and after interleaved; the rings looked at in the game
