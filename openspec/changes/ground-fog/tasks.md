# Tasks: ground-fog

- [x] 1.1 `cb_core::world::mist`: `mist_at` from hollows, water, the coast and height; tuning under `fog:` in `world.ron` (tests: `mist_thickens_the_fog_and_none_forms_up_high`, `seed_42s_valleys_are_mistier_than_its_ridges`). Presets per mission not yet
- [x] 1.2 The analytic optical depth as a pure function in the core, with a test against a numerical march along rays at every angle (`the_fog_is_exact_along_a_ray_at_every_angle`)
- [x] 2.1 The mist map baked with the world plan (256 x 256 cells of 160 m) and uploaded once as an R8 texture
- [x] 2.2 The fog in `water_post.wesl`'s compose pass: height fog with four mist taps, the sun-lit colour with the storm shade at the eye, the rain haze while the eye is under a shaft; the compose pass runs every frame while fog is on; the sealed cabin stays clear; the shader compiled in `cargo test`, its uniform's size pinned
- [x] 2.3 `CB_NO_GROUND_FOG=1`
- [x] 3.1 Headless stills (lavapipe): a river, the low ground of the wet west, under a storm's shaft (`CB_STILLS_WEATHER` views; `docs/screenshots/fog-rocks/`)
- [ ] 3.2 Measured on the owner's hardware with `CB_NO_GROUND_FOG` against the same take
- [ ] 3.3 Playtest: walk out of a misty valley and fly over the crater, on Drive
