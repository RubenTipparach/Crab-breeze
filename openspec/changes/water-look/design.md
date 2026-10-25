# Design: water-look

All in `assets/shaders/water_surface.wesl` and its uniform in `crates/cb-app/src/water_render.rs`;
numbers in `data/world/water_look.ron`.

## 1. Wave detail

- **The bake:** a 512 px, 64-frame tileable normal and height atlas covering a 64 m patch, baked
  from a JONSWAP spectrum (wind 6 m/s from the west as `cloud-weather`; fetch 50 km) by an inverse
  FFT. It loops in 8 s. That is 16 MB as BC5 normals. A tool writes it
  (`tools/water/bake_spectrum.py`); nothing runs an FFT at run time.
- **Sampling:** at 64 m, 13 m and 3.1 m (scale 1, 4.9 and 20.6, chosen so no two tile together),
  each scrolled along the wind at its phase speed, summed as slopes with the existing swell's.
  Rivers keep their two-phase flow over the same atlas.
- **Filtering:** each scale fades out once a pixel covers a quarter of its patch, as the swell does
  now. What it would have added goes into roughness (step 4), so the far sea stays a broad shimmer.

## 2. The sky in the water

- A 256 px cube map of the sky and its clouds (the visible sky pass plus the cloud billboards, no
  ground) is rendered from the eye every 2 s, one face a frame, and blended over 0.5 s, so a
  passing cloud shows in the sea without flicker. That is 1.5 MB as RGBA16F.
- The reflection reads it at the reflected ray, its mip chosen by roughness.

## 3. Things in the water

- **A trace in the water shader:** 24 steps along the reflected ray in screen space, against the
  depth prepass, taking the colour from the scene copy (`view_transmission_texture`) the water
  already reads. Steps lengthen with distance. A hit fades out near the screen's edge and where the
  ray leaves the screen. A miss shows the sky cube.
- **What reflects:** everything drawn before the water (ground, buildings, the mech, a hull). The
  water does not reflect itself or the clouds (the cube does).
- **Why not Bevy's screen-space reflections:** in the Bevy versions they shipped with they need the
  deferred path, and the water is a transmissive forward material (to be checked against Bevy
  0.20's docs before building). The trace uses the same two textures the water already binds.
- **Why not a planar mirror camera:** exact for a flat sea, but it draws the scene twice.

## 4. The sun's glitter path

- GGX specular with the water's F0 of 0.02. The roughness is `0.02 + toksvig`, where `toksvig` is
  the variance the filtered normals lost (the length of the averaged normal). Close water
  sparkles; far water shows a wide streak.
- The sun is a disc of 0.53 degrees: the highlight is integrated over it, so glints are never
  single white pixels.

## 5. Light through the water

- **Scatter:** `scatter_rgb x sun x max(0, h) x pow(max(dot(view, -sun_horizontal), 0), 4)`,
  where `h` is the wavelet's height. Turquoise (0.05, 0.35, 0.33) on crests seen against the sun.
- **Shadow:** the sun's shadow map (Bevy's directional shadow cascades) dims the glint and the
  scatter under a hull or a mech. The body colour stays, as real water barely takes a shadow.
- **Contact:** the existing shore foam already reads the ground's depth under the water, so it
  rings a mech's legs and a hull too.

## 6. Cost (expected; measured on the owner's hardware with the water filling half the screen)

| Item | Cost |
| --- | --- |
| Wave detail (3 scales, 2 taps each) | 6 samples a pixel, about 0.1 ms |
| Sky cube, one face a frame | about 0.15 ms a frame (sky and clouds only) |
| Screen-space trace, 24 steps | about 0.4-0.6 ms |
| GGX, scatter, shadow tap | about 0.05 ms |
| Memory | 16 MB atlas, 1.5 MB cube |

## Open questions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| W29 | Which steps for the water | all five, in order (detail, sky, trace, glitter, scatter and shadow) | proposed (survey W29) |
