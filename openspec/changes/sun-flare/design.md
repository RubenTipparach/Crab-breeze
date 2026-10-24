# Design: sun-flare

## 1. Where it lives

- `cb_core::sun_flare` (no engine): the data types (`SunLook { bloom, flare }`, validated by field),
  and the rules as pure functions: `sun_ndc` (the sun's screen position), `on_screen` (the edge
  fade), `occlusion_taps` / `tap_uvs` (where the depth is sampled), `depth_visibility` (the share of
  the sun the depth leaves clear), `medium_fade` (cloud and water at the eye), `fade_toward`
  (smoothing), `strength` (light and exposure) and `starburst_angle`.
- `cb_app::sun_flare` (`SunFlarePlugin`): gives the cameras their bloom and markers, follows the
  medium at the eye, and runs the passes in the `Core3d` schedule (Bevy 0.20 has no render graph;
  the same pattern as `water_render`).
- `assets/shaders/sun_flare.wesl`: the flare and the cockpit's taps.
- `data/world/sun_flare.ron`: every number. `GameData.sun`, so `cb-tools validate` checks it.

## 2. Bloom

- Bevy's `Bloom` on the pilot's eyes only, from `bloom` in the data: energy-conserving, intensity
  0.08 (Bevy's default is 0.15), low-frequency boost 0.5, no threshold (physically based: every
  pixel scatters a little in proportion to its light).
- Bevy's first downsample weighs each pixel by `1 / (1 + luma / 4)` (Karis), so the sun disk (about
  1e5 times the sky in linear units under raw sunlight) cannot flood the screen: it glows, it does
  not white out the view. The flare's own core and glow give the sun its wide warm bloom.
- **The HUD.** Bevy 0.20 runs `ui_pass` after `Core3dSystems::PostProcess` (and before upscaling)
  on whichever camera draws the UI: bloom runs inside the post-process set, so it never reaches
  Bevy UI. The HUD stays sharp.
- **The cockpit.** The cockpit camera (`CockpitCamera`, order 1, layer 1) draws the cabin after the
  eyes have finished their post-process passes, into the same HDR texture. It gets no `Bloom`: the
  cabin, its panels and the HUD combiner stay crisp, and only the outside view through the
  screens blooms. (A blooming cabin would also bloom the outside a second time.)
- `CB_NO_BLOOM=1` leaves the bloom off.

## 3. The flare

### 3.1 The sun's position

`sun_ndc(clip_from_world, toward_sun)`: the direction to the sun (the first directional light's
`back()`) as a point at infinity, `clip = M * (d, 0)`; `None` when `clip.w <= 1e-6` (behind the eye
or exactly beside it), otherwise `clip.xy / clip.w`. Computed on the CPU each frame in the render
world from the view's own `clip_from_world`, so it is exactly the frame drawn. Sizes are in
**screen heights** (`_h`): origin at the centre, y up, one screen height is 1, square pixels.

### 3.2 The elements (`flare_at` in the shader)

With `p` the pixel and `s` the sun (screen heights), `d = p - s`, `r = |d|`, and `S` the flare's
strength (section 3.4), each element adds `S *` its colour times:

| Element | Shape | Data (defaults) |
|---|---|---|
| Core | `exp(-r^2 / R^2)` | warm `(6.0, 5.0, 3.8)`, `R = 0.012` |
| Glow | `(1 / (1 + (r/G)^2))^k` | `(1.0, 0.72, 0.45)`, `G = 0.05`, `k = 1.4` |
| Spikes | `abs(cos(a n / 2))^m * exp(-r / L_c)`, `L_c = L (1 + f c)` per channel | 6 spikes, `m = 90`, `L = 0.16`, fringe 0.08 |
| Fine rays | `abs(cos(a N / 2))^(m/4) * exp(-r / l_i)`, `l_i` hashed per ray, 0.45-1.0 of `l` | 24 rays, strength 0.35, `l = 0.07` |
| Streak | `exp(-(d.y / T)^2) * exp(-abs(d.x) / W)` | cool `(0.30, 0.50, 1.0)`, `T = 0.0025`, `W = 0.45` |
| Halo | `exp(-((r - H_c) / w)^2)`, `H_c = H (1 + f c)` | faint `(0.10, 0.12, 0.14)`, `H = 0.22`, `w = 0.012`, fringe 0.035 |
| Ghost i | soft body with a brighter rim at `s (1 - t_i)`, disc or hexagon of radius `g_i`, red drawn 4 % larger and blue 4 % smaller | 7 ghosts, `t` 0.35-2.10, `g` 0.010-0.140, cool tints |

`a` is the angle round the sun plus the starburst's turn, `0.6 rad` per screen height of the sun's
`x + y` (`starburst_angle`): the burst wheels slowly as the view swings the sun across the screen,
and never jumps. `c` is -1, 0, +1 for blue, green, red. Ghost `t = 0` sits on the sun, `t = 1` at
the centre, `t = 2` the sun mirrored. With the default gain the core peaks near 5.5 (white after
tonemapping), the spikes near 1.3, the streak 0.9 and each ghost 0.03-0.3: the ghosts read as
tinted glass reflections, not stickers.

### 3.3 Fading

The flare's visibility is the product of:

- **Depth occlusion, the world**: 17 taps on a golden-angle (Vogel) spiral over a disc of
  `occlusion_radius_h = 0.012` (about 13 px at 1080p) round the sun. A tap is clear where the
  depth is the sky's (reverse-Z, `<= 1e-7`); off-screen taps count as clear. The share clear is
  `depth_visibility`. The spiral has no mirror symmetry, so a straight edge sliding over the sun
  takes the flare down in steps of at most 2/17 (test: an edge swept across in 40 steps never
  drops it more than 2/17 at once); it fades over about 26 px, it does not pop.
  The GPU computes it once per vertex of the fullscreen triangle (51 depth loads a frame, not
  per pixel) and hands it to the pixels as a flat value. Sky is depth 0: the atmosphere writes no
  depth. Buildings, terrain, trees and the mech on foot hide the sun this way.
- **Depth occlusion, the cockpit**: seated, the pilot's mech and cabin are drawn by the cockpit
  camera on its own depth, after the world, so the world's depth never sees them. The cockpit
  camera runs the same 17 taps against its own depth (`taps_vertex` / `taps_fragment`) into a 1x1
  `R16Float` texture; the eyes read it the next frame. Its screens are open (the outside shows
  through them), so only the frame, pillars, header beam and monitors hide the sun. One frame of
  lag (17 ms at 60 Hz) under a fade that already spans 26 px is not visible. On foot the cockpit
  camera draws nothing and the share is 1.
- **On screen**: 1 inside the view, falling by smoothstep to 0 at `offscreen_margin_ndc = 0.3` NDC
  past the edge, so the ghosts and streak linger a little as the sun leaves. Behind the eye there is
  no flare.
- **Cloud and water at the eye**: `medium_fade = (1 - submersion) * (1 - smoothstep(0,
  cloud_full_fade, density))`, the submersion from `WaterEye` (0 dry, 0.5 straddling, 1 under) and
  the density from the one cloud query (`Clouds::density_at`, as `weather_and_screens` uses), then
  smoothed with a 0.15 s time constant so diving or flying into cloud fades it rather than cuts it.

### 3.4 Strength

`S = light colour * illuminance (lux) * exposure * gain`, with Bevy's `Exposure::exposure()`
(`2^-ev100 / 1.2`). Under raw sunlight (about 120,000 lux) at EV100 13.4 the exposed illuminance is
about 9.25; `gain = 0.1` gives `S` about 0.9. A dimmer sun or a brighter exposure scales the whole
flare with it, as a real lens would. The old flat-sky mode (`CB_NO_SKY=1`: 10,000 lux at EV100
9.7) lands near the same strength.

### 3.5 Passes and order

Both are systems in `Core3d`, in `Core3dSystems::PostProcess`, after Bevy's `bloom`, before the
water's passes (`WaterPasses`, so drops on the glass refract the flare too) and before
`tonemapping`.

- **Eyes** (`SunFlareEye`): one fullscreen triangle, additive blend (`One, One` on colour, alpha
  kept) straight into the main HDR texture: no read of the scene, no ping-pong copy.
- **Cockpit** (`SunFlareCockpit`): one triangle into the 1x1 texture.

A pass is skipped on the CPU when the sun is behind the eye, more than the margin off screen, or
(the eyes) when cloud or water has put the flare out. The depth buffers of both cameras are made
samplable (`water_render::samplable_depth`, now generic over the marker).

### 3.6 The view

The flare never reads or writes a transform: it adds light to the image only. The view's direction
stays the player's (CLAUDE.md section 8).

## 4. Costs (estimates, not measured; cloud sessions and agents measure nothing)

| Item | Per frame at 1920 x 1080 |
|---|---|
| Passes | Bloom: Bevy's mip chain (1 prefilter downsample, about 8 down and 8 up, max mip 512 px) plus 1 full-res composite. Flare: 1 fullscreen triangle on the eyes, 1 one-pixel triangle on the cockpit camera. |
| Texture reads per pixel | Flare: 0 scene reads; the additive blend reads and writes the RGBA16F target once (16 bytes). Depth: 51 loads for the whole frame (vertex stage) on each camera. Bloom: about 9 bilinear taps per full-res pixel in its last upsample, and its smaller mips. |
| ALU per pixel (flare) | About 150-300 operations: core and glow, two `pow` and `atan2` for the burst, the streak, the halo, and up to 7 ghosts (skipped outside 1.4 radii). |
| GPU, RTX 3070 (estimate) | Flare 0.08-0.15 ms; taps under 0.01 ms; bloom 0.15-0.3 ms. |
| GPU, M4 Max (estimate) | Flare 0.1-0.25 ms; taps under 0.01 ms; bloom 0.25-0.5 ms. |
| CPU | One cloud query and a few hundred flops: under 0.01 ms. |
| Memory | One 1x1 `R16Float` texture; two 704-byte uniform buffers; Bevy's bloom mips (about 1.5 MB at 512 px `Rg11b10Ufloat`). |
| When the sun is behind or off screen | The flare costs nothing (both passes skipped); bloom always runs. |

To be measured (`perf-measure`) as the default against `CB_NO_FLARE=1` and `CB_NO_BLOOM=1`, in the
cockpit, looking at the sun, interleaved.

## 5. Tests

- `cb_core::sun_flare`: behind the eye gives no position; straight ahead is the centre; the edge
  of the field is the edge of NDC; the sun follows the view not the eye's position; square pixels;
  the edge fade is smooth; all sky is 1 and all hidden 0; an edge sliding over the sun fades in
  steps of at most 2/17; off-screen taps count as clear; the tap uvs sit on the sun; cloud and water
  put it out; the smoothing never jumps; strength scales with light and exposure; the starburst
  turns smoothly; the data file loads; bad data is refused by field.
- `cb_app::sun_flare`: the WGSL compiles and validates with naga with and without MSAA, with all
  four entry points; the uniform's size matches the WESL struct; the shader's array sizes and sky
  depth are the core's; the CPU's draw decision (ahead, behind, past the margin, under water, the
  cockpit still testing, a dark sun); the uniform carries the data; the bloom is the data's.

## Open questions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| F1 | How strong and how "epic" | the defaults here: a bright warm core and burst, a thin cool streak, faint ghosts; raise `gain` for more | provisional |
| F2 | Does the cabin bloom (lit buttons, screens' glow) | no: the cabin stays crisp; bloom only the outside view | provisional |
| F3 | Should smoke, particles and cloud billboards hide the sun (they write no depth) | not yet: cloud at the eye fades it; a translucent occluder would need its own coverage pass | provisional |
| F4 | Flare colour for a low, reddened sun | not yet: the light's colour only; the atmosphere's transmittance at the sun later | provisional |
| F5 | The cockpit camera shares the eyes' HDR texture and has Bevy's default tonemapper: does it tonemap the world a second time over the eyes' ACES? | check in a screenshot with the cockpit camera's `Tonemapping::None`; out of this change | to check |
