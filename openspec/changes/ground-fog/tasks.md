# Tasks: ground-fog

- [ ] 1.1 `cb_core::world::mist`: `mist_at` from hollows, water, the coast, the crater floor and height; presets; tuning under `fog:` in `world.ron` (tests: a valley is mistier than the ridge above it, nothing above 600 m, the same world gives the same map)
- [ ] 1.2 The analytic optical depth as a pure function in the core, with a test against a numerical march along rays at every angle (level, up, down, from inside and above the layer)
- [ ] 2.1 The mist map baked at start on a worker thread into an R8 texture
- [ ] 2.2 The fog in `water_post.wesl`'s compose pass: height fog with the mist taps, the sun-lit colour with the storm shade, the rain haze from the shafts; the compose pass runs every frame while fog is on; the sealed cabin stays clear; the shader compiled in `cargo test`, its uniform's size pinned
- [ ] 2.3 `CB_NO_GROUND_FOG=1`
- [ ] 3.1 Headless stills (lavapipe): the crater at dawn from the rim, a river valley from the ground, under a storm's shaft, from the cockpit at 1 km over the coast (`CB_STILLS_WEATHER` views)
- [ ] 3.2 Measured on the owner's hardware with `CB_NO_GROUND_FOG` against the same take
- [ ] 3.3 Playtest: walk out of a misty valley and fly over the crater, on Drive
