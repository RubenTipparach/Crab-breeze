# Tasks: water-rendering

## 1. Data and the eye (cb-core)

- [x] 1.1 `data/world/water_look.ron` and `cb_core::water_look::WaterLook`, loaded by `GameData` (tests: `the_shipped_water_look_loads`, `a_negative_dry_time_is_rejected_naming_the_field`, `an_unknown_key_is_rejected`; scenario "A bad value")
- [x] 1.2 `submersion` and `Lens` (Pale Blue Dot's `emerge`) (tests: `the_eye_is_under_straddling_or_dry`, `the_glass_dries_after_surfacing`, `straddling_keeps_the_glass_wet`, `diving_again_clears_the_drops_until_the_next_surfacing`, `a_dry_eye_that_never_dived_has_no_drops`; scenarios "Under, straddling and dry", "Surfacing", "Diving again")

## 2. Shaders

- [x] 2.1 `assets/shaders/water_common.wesl`: Tenebris's noise and fbm, the light through water, the Heartfelt drop field
- [x] 2.2 `assets/shaders/water_surface.wesl`: the surface (fbm and swell normals, flow map, Fresnel sky, refraction and absorption, foam, glint, Snell's window from below)
- [x] 2.3 `assets/shaders/water_post.wesl`: `compose` and `lens`
- [x] 2.4 Compiled and validated in `cargo test` (tests: `every_shared_water_function_compiles`, `the_post_shader_compiles_with_and_without_msaa`, `the_post_uniform_matches_the_wesl_struct`; scenario "A broken post shader")
- [x] 2.5 The surface shader against Bevy 0.20's own WESL modules (test: `the_surface_shader_compiles_against_bevy_pbr`, which says it skipped when Bevy's sources are not in the Cargo registry)

## 3. The game (cb-app)

- [x] 3.1 `water_render::WaterRenderPlugin`: `WaterMaterial`, its per-frame uniform (eye, time, light), the two post passes gated to `WaterOptics` cameras and to frames that need them (tests: `a_dry_eye_runs_no_water_pass`, `under_water_composes_but_puts_no_drops_on_the_glass`, `a_wet_glass_runs_both_passes`, `the_water_body_is_lit_like_the_ground`, `the_water_surface_is_never_displaced`; scenario "The swell is in the normals")
- [x] 3.2 `world::spawn_water` makes every water mesh's material with `world::water_material`; rivers carry their flow in vertex colour; the old ripple normal map and its scrolling are gone (scenario "Rivers and lakes share the material": playtest beat 4.1)
- [x] 3.3 Pilot camera: `WaterOptics` (HDR, depth prepass, screen-space transmission, samplable depth); `weather_and_screens` hands the fog to the water under water

## 4. Proof

- [ ] 4.1 Playtest on the owner's machine, with the owner's permission first: wade into a river, sink under a lake, walk on the bottom looking up at Snell's window, surface and watch the drops run off, skate along a river and look at the flow, foam and glint; record and upload
- [ ] 4.2 Measure (`perf-measure`): old build against new, dry and under water, the costs in design section 5
- [ ] 4.3 Move the requirements into `openspec/specs/water-rendering/spec.md` once the playtest shows them
