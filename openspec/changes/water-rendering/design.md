# Design: water-rendering

Source: Pale Blue Dot at revision `db25fe2` (the owner's Bevy 0.18.1 game), itself a term-for-term
WGSL port of Tenebris (`RubenTipparach/tenebris` at `ef98651`, `water.fs.glsl`, `water.vs.glsl`,
`composite.fs.glsl`). Files read: `crates/pbd-app/src/planet_water.rs`, `assets/shaders/water.wgsl`,
`assets/shaders/sea.wgsl`, `assets/config/water.ron`, `assets/config/weather.ron`,
`crates/pbd-app/src/weather.rs`, `docs/shader-port.md`.

## 1. What is ported, and how it changes for a flat world on Bevy 0.20

| Pale Blue Dot | Crab Breeze | Change |
| --- | --- | --- |
| `water.wgsl` `gn_*`, `gnoise3`, `fbm3` (Tenebris hash constants) | `assets/shaders/water_common.wesl` | Verbatim math; WESL module shared by both shaders. |
| `water.wgsl` `fragment` (the cap) | `assets/shaders/water_surface.wesl`, a Bevy `Material` in the transmissive phase | The cap was drawn in PBD's own post pass from its cell buffer; here every water mesh (rivers, lakes, the ocean) is an ordinary mesh with `WaterMaterial`. The planet radial is world up. The scene behind comes from Bevy's `view_transmission_texture` (a copy of the opaque pass) and its depth from the depth prepass, instead of PBD's swap and main depth. Day/night (`night_sky`, terminator) is dropped: Crab Breeze has one sun and no night yet. The sky reflection's horizon is the game's `SKY` colour, so the mirror and the sky are one colour. Bevy's own `apply_fog` fogs the surface, so the water fogs with everything else (the camera's `DistanceFog` is still the one fog in air). |
| `sea.wgsl` swell (12 directions x 6 bands, displaced vertices) | three swell components in the normals only | The surface is not displaced: the simulation's `surface_m` stays the surface the mech skates on and sinks under (CLAUDE.md 6.1). |
| flow advection by a per-cell flow vector | a two-phase flow map from the river mesh's vertex colour (rg = flow, m/s) | A constant offset `flow * t` stretches the noise at river bends after minutes; two phases half a period apart, cross-faded, never stretch (Vlachos 2010). Meshes without vertex colour (lakes, the ocean) are still water. |
| foam (crest and slope) | the same, plus shore foam | Shore foam: where the vertical water depth under a surface pixel (prepass depth) is under `shore_foam_m`, broken up by the fbm height. |
| `compose` (wet mask, distortion, absorption by travel, depth blur) | `assets/shaders/water_post.wesl` `compose`, a fullscreen pass | The sea sphere becomes the plane `y = surface_m` under the eye. A sealed cockpit is dry: travel through water starts `cabin_dry_radius_m` from the eye when seated. |
| `murk(eye_depth)` | `water_murk` in `water_common.wesl` | The deep colour is lit like an albedo by the light reaching the water (sun over pi plus ambient, times exposure), computed once on the CPU (`water_light`) and passed to both shaders, so the murk under water and the deep colour seen from above are the same number. |
| underwater branch of the cap (Snell's window) | the `below` branch of `water_surface.wesl` | The distance murk is left to `compose`, so the surface seen from below is fogged once. |
| `lens` (Heartfelt drops, emerge drips) | `water_post.wesl` `lens` | Verbatim drop math; the drops sit only on what is beyond the cabin when seated (they are on the external camera lenses, not on the instruments). Rain on the lens is ported and gated by `lens_rain` (0 until weather exists). |
| `lens_mist_pass`, clouds, rain volume, overlay | not ported | Need PBD's cloud and weather maps. |
| `submersion` (tri-state 0, 0.5, 1) and `emerge` | `cb_core::water_look::{submersion, Lens}` | Pure, tested headless. The eye's water comes from the one water query, `Ground::water_at`. |

Licence note: the drop functions are Martijn Steinrucken's "Heartfelt" (2017, CC BY-NC-SA 3.0), as
Tenebris and Pale Blue Dot carry them. The non-commercial clause matters before a commercial
release (open question WR3).

## 2. Frame order

1. Depth prepass (new on the pilot camera): opaque and alpha-masked geometry. The water material
   has no prepass, so the prepass depth is the ground under the water.
2. Opaque main pass.
3. Transmissive pass: Bevy copies the main texture to `view_transmission_texture`, then draws the
   water (depth test and depth write on, nothing culled so the underside draws).
4. Transparent pass (clouds, effects).
5. `water_compose` (only while the eye is under, straddling, or the glass is wet): reads the HDR
   main texture and the main depth (which includes the water surface), writes the other main
   texture.
6. `water_lens` (only while drops are on the glass): reads the composed image, writes the other.
7. Tonemapping (Bevy). Both water passes are in `Core3dSystems::PostProcess`, before `tonemapping`.

The passes run only on cameras that carry `WaterOptics`; today that is the one `PilotCamera`, which
the `--bench` run points at an offscreen image, so the bench measures them.

## 3. The eye: submersion, emerge, fog

Each frame in `PresentSet::Camera`, after the camera is placed:

- `eye_depth_m = surface_m - eye.y` from `plan.water_at(eye.x, eye.z)` (no water: dry).
- `submersion = 1` when `eye_depth_m > band`, `0.5` within `band` of the surface, `0` above
  (`partial_band_m`, PBD's 0.8 m).
- `Lens::step(now_s, submersion)`: PBD's `emerge`. Surfacing or straddling arms the window
  `emerge_dry_s` (2.6 s) ahead; drip intensity falls linearly from 1 to 0 over it; under water there
  are no drips.
- `weather_and_screens` stays the one place that decides the camera's fog: in air it fogs by the
  cloud at the eye as before; under water it pushes the air fog out to clear-air visibility, because
  the water's fog is `compose`'s per-channel absorption, which `DistanceFog` cannot give to sky
  pixels.

## 4. Numbers (data, `data/world/water_look.ron`)

PBD's values, except where marked. Absorption is clearer than PBD's (0.90, 0.25, 0.08 per metre,
tuned for a swimmer) so a 9.6 m-high mech eye on the sea floor sees about 20 m in blue-green
(provisional, WR1).

| Key | Value | Unit | Note |
| --- | --- | --- | --- |
| `absorption_per_m` | (0.45, 0.12, 0.06) | 1/m, RGB | WR1 |
| `deep_color` | (0.0, 0.12, 0.28) | albedo | PBD |
| `ripple_scale_per_m` | 0.6 | noise cells per metre | PBD 1.5 is a swimmer's scale |
| `ripple_steepness` | 0.45 | | PBD |
| `slope_max` | 0.7 | | PBD |
| `refract_amount`, `refract_max_uv` | 0.04, 0.03 | uv | PBD |
| `max_path_m` | 200 | m | PBD |
| `swell_wavelength_m` | (60, 23, 9) | m | three normal-only components |
| `swell_height_m` | (0.35, 0.12, 0.05) | m | |
| `foam_*` | PBD | | plus `shore_foam_m` 1.5 m |
| `specular_intensity`, `specular_power` | 0.06, 140 | | PBD |
| `partial_band_m` | 0.8 | m | PBD |
| `emerge_dry_s` | 2.6 | s | PBD |
| `underwater_distortion` | 0.0015 | uv | PBD |
| `wet_blur` | 0.02 | | PBD |
| `cabin_dry_radius_m` | 2.5 | m | the cockpit pod's half size |
| `lens_density`, `lens_refract`, `lens_speed`, `lens_size`, `lens_scale` | 1.4, 1.7, 1.0, 0.7, 2.0 | | PBD `weather.ron` |
| `lens_rain` | 0.0 | 0..1 | off until weather exists |

## 5. Costs

Measured: nothing yet. Every GPU figure below is an estimate for an RTX 3070 at 1920 x 1080 with
4x MSAA, to be measured in an owner-approved playtest (`perf-measure`, old against new, interleaved).
This Mac session may not run the game (CLAUDE.md section 8).

| Item | Per frame | Texture reads per pixel | Memory | Estimate |
| --- | --- | --- | --- | --- |
| HDR main target (was sRGB 8-bit) | always | none extra | +33 MB (two 1080p Rgba16Float, +4x MSAA colour) | +0.1-0.2 ms bandwidth, tonemapping becomes a 0.05 ms pass |
| Depth prepass (new) | always | none | 1080p 4x MSAA depth copy, 33 MB | +0.3-0.6 ms vertex work; early-z saves some main-pass shading |
| Transmission copy | when water is in view | 1 copy | one 1080p Rgba16Float, 16.6 MB | 0.05-0.1 ms |
| Water surface | water pixels only | 2 depth loads, 1 transmission sample; 9 noise octaves (18 on rivers) | none | 0.2-0.5 ms with half the screen water |
| `water_compose` | under water, straddling, or wet glass | 1 depth load, 1-2 colour samples; 17 more while the glass is wet | none | 0.1-0.3 ms; 0 when dry |
| `water_lens` | the 2.6 s after surfacing | 1 depth load, 1 colour sample, 3 drop evaluations | none | 0.2-0.4 ms; 0 otherwise |
| Main world | always | | | one water query and four uniform writes, under 0.01 ms |

Draw calls: unchanged for the water meshes (they move from the opaque to the transmissive phase), two
fullscreen triangles at most. The per-frame update of the water materials' uniform (eye, time,
light) re-uploads one 400-byte uniform per water material.

Worst case: the eye straddling the surface over a river with the glass wet: all of the above at
once, about 1.5-2.5 ms. That fits the 8.3 ms p95 budget only if the rest of the scene leaves room;
the measurement decides.

## 6. Risks

- The water shader is WESL against Bevy 0.20's `bevy_pbr` modules. `cargo test` compiles it the way
  Bevy's shader cache does (the `wesl` compiler with Bevy's options and the feature flags of the
  pilot camera's transmissive pipeline, with and without MSAA, vertex colour, prepass,
  transmission and fog) against Bevy's own WESL sources from the Cargo registry, then validates it
  with naga 30.0.1 and checks the uniform's size against the Rust struct; the test says it skipped
  when Bevy's sources are not in the registry. The post shader and the shared module are compiled
  and validated the same way with no Bevy sources needed. A pipeline is still only proven by the
  running game (bind group layouts, the shader defs Bevy really passes).
- HDR changes the whole game's output path. Tonemapping is the same operator (TonyMcMapface), now
  a pass instead of in-shader, so the look should not move; the playtest confirms it.

## Open questions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| WR1 | How clear is the water under the surface? | Clearer than Pale Blue Dot (about 20 m visibility in blue-green for the mech on the sea floor) | provisional (survey WR1) |
| WR2 | Should the drops on the glass cover the cockpit instruments too? | No: drops are on the external camera lenses, so only the screen image beyond the cabin gets them | provisional (survey WR2) |
| WR3 | The drops are "Heartfelt" (CC BY-NC-SA 3.0). Replace before a commercial release? | Yes, write our own drop field before any sale; keep the port for now | open (survey WR3) |
| T4 | Pale Blue Dot's source for the camera drops | Settled: the source was reachable and is ported here | answered |
