# Design: glass-reflections

## 1. Why the glass looked like clear plastic

- The pilot's eye camera had no environment map. Bevy's PBR reflects the environment map and the
  analytic lights only, so a mirror-smooth pane reflected nothing but the sun's highlight.
- The glass was `AlphaMode::Blend` at alpha 0.35. That mode blends `src x alpha + dst x (1 - alpha)`,
  and `src` is the whole lit colour, the specular reflection included, so even a perfect
  reflection reached the screen at 35 %. `AlphaMode::Premultiplied` blends
  `src + dst x (1 - alpha)` and the shader leaves `src` as it is (`premultiply_alpha` in
  `bevy_pbr` 0.20's `pbr_functions.wesl`), so the reflection is added whole and the body colour is
  given already multiplied by alpha (`glass.tint`).

## 2. The cached sky (built)

**Source.** Poly Haven's `kloofendal_48d_partly_cloudy_puresky`, 2k equirectangular `.hdr`, CC0
(sha256 pinned in the tool). Midday, partly cloudy, sky only: the game's daylight. Chosen as the
most used midday "pure sky"; which sky (and whether there is more than one, by time of day or
weather) is open question GR2.

**Baking** (`tools/textures/gen_sky_cube.py`, a few seconds): six 512 px faces in wgpu order
(+X, -X, +Y, -Y, +Z, -Z), stacked into a 512 x 3072 linear Radiance file, 6.3 MB.
- The sun is clamped: any texel brighter than 1.5 x the sky's 99th percentile luminance (2.07, so
  3.11) is scaled down to it (5750 texels). The game's sun is a `DirectionalLight` with its own
  highlight; a second sun in the reflection would point elsewhere.
- Below the horizon is a flat grey-brown ground at 22 % of the sky's mean radiance (0.104 against
  0.453), blended in over 4 degrees. The city, terrain and water are the game's own.

**Use** (`crates/cb-app/src/sky_light.rs`). Loaded by the `AssetServer`, reinterpreted as a cube
(`reinterpret_stacked_2d_as_array(6)`, a `Cube` view) and put on the eye camera as a
`GeneratedEnvironmentMapLight` at `sky_cd_m2` = 2000 cd/m2 a unit, (the open sky's mean then sits
near 900 cd/m2, the flat ambient it replaces). The loading screen waits for it (`GAME_READY`).
The flat ambient stays at 250 cd/m2 as fill: the sky light is not occluded, so a room behind a
wall would otherwise be as lit as the street.

**Cached, not re-filtered.** Bevy's generator re-filters its source cube on every frame it is
present (seen in the lavapipe run's render diagnostics: `lightprobe_radiance_map` 214 ms and
`lightprobe_irradiance_map` 50 ms a frame). It writes into the images of an ordinary
`EnvironmentMapLight` it adds to the camera, so once the render world has counted 3 frames with
all five of the generator's compute pipelines compiled and the map queued, the generator is taken
off and the filtered `EnvironmentMapLight` stays. (Counting 20 plain frames first was not enough:
those frames passed while the pipelines were still compiling, and the cached map was black: the
whole street came out at less than half its brightness.)

**Cost.** One 512 px cube source (25 MB as loaded, Rgba32Float) and its filtered Rgba16Float
copy (about 17 MB with mips, plus a 32 px irradiance cube), filtered once while the loading
screen is up. Per frame: one more cube lookup in every PBR fragment shader, the same as any
environment map. Not measured: this is a cloud session (no GPU); a session on the owner's machine
measures it (task 1.5).

## 3. Cached street probes (designed, not built)

The sky alone reflects no buildings. The owner's "cache some cube maps" is the route:
- When the player has been within 60 m of a street crossing for a second, and no probe is cached
  there, the existing face-camera rig of `screens_cube` draws a 128 px cube at 6 m above the
  crossing, one face a frame (six frames), with buildings dressed. The faces are copied into a
  cube and kept.
- Each cached cube is a Bevy `LightProbe` with a `GeneratedEnvironmentMapLight`, its box the
  crossing's street segments out to the next crossing, falling back to the sky outside them.
- At most 8 probes are kept (8 x 128 px x 6 faces x 8 bytes x 4/3 for mips = 4 MB); the farthest
  goes first. A probe is redrawn when a building in its box falls (`structural-destruction`).
- Cost: six faces at 128 px once per new crossing. The live screens cube measured 4.7 ms a face on
  the owner's RTX 3070 because each face visits every mesh in the world; a probe's face is far-
  clipped at 250 m (the dressing radius), and it is drawn once, not every frame.

## 4. Data

`data/world/sky_light.ron` (`cb_core::sky_light::SkyLightConfig`): `cube`, `sky_cd_m2`, `yaw_deg`,
`ambient_cd_m2`, `ambient_without_sky_cd_m2` and `glass` (`tint`, `alpha`, `roughness`,
`reflectance`). An empty `cube` turns the sky light off and restores the old flat 900 cd/m2.

## Open questions

| Id | Question | Recommendation | State |
|---|---|---|---|
| GR1 | Show the HDRI as the visible sky (a skybox) too, or keep the flat sky colour and fog? (survey W19) | Keep the flat sky for now: the fog and clouds are tuned to it, and the reflections carry the HDRI's clouds | provisional |
| GR2 | Which sky, and one or several (time of day, weather)? (survey W19) | One midday partly cloudy sky now; a sky per mission later | provisional |
| GR3 | Glass colour and strength (survey W20; owner, 2026-10-24: "usually glass is tinted", "so it reflects more light, and interiors appear darker than the world around it", "too transparent", "go harder on the reflections") | Mirror-coated: alpha 0.88 (12 % of the room shows), coating 0.9 over a blue-grey tint (about 45 % head-on), each building's tint on top | provisional |
| GR4 | Build the cached street probes (section 3) next? (survey W21; the owner's reference photos show towers mirroring the street) | Yes | open |
