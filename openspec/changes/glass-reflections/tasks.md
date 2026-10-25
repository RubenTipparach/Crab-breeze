# Tasks: glass-reflections

- [x] 1.1 `tools/textures/gen_sky_cube.py`: the Poly Haven sky fetched (pinned), sun clamped, ground below the horizon, baked to a stacked cube
- [x] 1.2 `data/world/sky_light.ron` and `cb_core::sky_light`, validated, with tests
- [x] 1.3 The sky cube on the eye camera as a `GeneratedEnvironmentMapLight`; the loading screen waits for it; the ambient from data
- [x] 1.4 Window glass premultiplied, from data
- [ ] 1.5 Measure the frame with and without the sky light on the owner's machine (perf-measure)
- [ ] 1.6 Playtest on the owner's machine: the towers mirror the sky from the street and from the cockpit
- [ ] 2.1 Cached street probes (design section 3), after GR4
