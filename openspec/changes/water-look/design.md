# Design: water-look

All in `assets/shaders/water_surface.wesl` and its uniform in `crates/cb-app/src/water_render.rs`;
numbers in `data/world/water_look.ron`.

## 0. Colour: a dark open sea, teal shallows

Owner, 2026-10-26: "Water is still too blue", "I want water to look realistic. Check out reference
images and look at other games triple a especially for inspiration", then, offered a dark grey-navy
open sea or greener coastal water: "1 and 2 are what I want. We can have both". Data only
(`data/world/water_look.ron`); the shader's model already has the parts.

**What makes it cyan today.** The water's own colour, `deep_color` (0.0, 0.12, 0.28), is lit by the
full sun and sky, so the sea sends back 28 % of blue light. Real deep ocean sends back about 2-3 %
of blue and under 1 % of red and green (its colour from the air is mostly the sky it reflects, 2-6 %
of it at the angles a pilot sees: a 1970s NASA photogrammetry report puts calm water at 3-6 % of
diffuse skylight). The reflected zenith is a saturated constant (0.03, 0.18, 0.55), and from the air
the water mirrors mostly the upper sky, so it mirrored that constant.

**References.**
- Deep water from altitude reads deep indigo or navy, shallows near the coast turquoise or teal
  (aerial descriptions; University of Texas Marine Science Institute, "Blue Ocean").
- Pure water absorbs red about 30 times faster than green and 35 times faster than blue: Pope and
  Fry (1997), about 0.30-0.34 /m at 600-650 nm, 0.057 /m at 550 nm, 0.009 /m at 450 nm (the field's
  standard values).
- Sea of Thieves (SIGGRAPH 2018, "The Technical Art of Sea of Thieves") blends a dark deep-water
  colour with a lighter subsurface colour by view angle, sun direction and wave crests; the
  subsurface part is this change's step 5. Crest (Unity) and most ocean renderers model the body
  as absorption plus a weak, blue-leaning in-scatter, with Fresnel reflection of the sky on top.

**The numbers.**

| Value | Was | Now | Why |
| --- | --- | --- | --- |
| `deep_color` (in-scatter albedo) | 0.0, 0.12, 0.28 | 0.004, 0.018, 0.040 | deep ocean returns about 1 % of red-green and 3-4 % of blue: navy |
| `absorption_per_m` | 0.45, 0.12, 0.06 | 0.32, 0.07, 0.02 | pure water (Pope and Fry) with a little coastal matter: sand shows teal through 2-10 m |
| `sky_zenith_color` | 0.03, 0.18, 0.55 | 0.16, 0.30, 0.56 | the game's sky deepened toward the zenith, not a saturated constant |

Through 3 m of water a sand bed (about 0.6, 0.5, 0.35) keeps (0.38, 0.81, 0.94) of its light: a
green-teal; through 10 m (0.04, 0.50, 0.82): teal going blue; past 30 m the dark body takes over.

**First stills, and what they showed.** The open sea went navy, but two things were off:
- Seen at a slant from the air, the light crosses six or more times the water's depth, which strips
  the green too, so the shallows came out cobalt, not teal. Coastal water is teal because what is
  in it (sediment, plankton) scatters green as well as blue: its in-scatter colour is teal, not
  navy. So the body colour follows the water's depth under the pixel (the shader already reads it
  for the shore foam): `shallow_color` (0.012, 0.060, 0.052) in the shallows, easing to
  `deep_color` by `shallow_depth_m` (14 m).
- Toward the horizon the sea stayed black: the reflection there was multiplied by
  `sky_horizon_strength` 0.22 where the reflected ray runs level. Water reflects most at a grazing
  angle, which is what greys a real sea toward its horizon; 0.7 keeps a little of the darkening that
  stops the far sea going white.

| Value | Was | Now |
| --- | --- | --- |
| `shallow_color` (new) | none | 0.012, 0.060, 0.052 |
| `shallow_depth_m` (new) | none | 14 |
| `sky_horizon_strength` | 0.22 | 0.7 |

Cost: one more read of the depth the shader already samples, a mix; nothing measurable.

**Second stills.** The horizon greyed as meant, but the shallows' fringe was royal blue: from a
pilot's height the view crosses the water at a low angle, so the light off the bed travels about six
times the depth, and at 0.07 /m the green was gone within a metre or two of depth. Clear tropical
water absorbs less in the blue-green than Pope and Fry's values (later measurements, Lee 2015 and
Mason 2016, put 410-550 nm about half as high), so green and blue absorption are halved, and the
shallows' scatter is stronger:

| Value | Second | Third |
| --- | --- | --- |
| `absorption_per_m` | 0.32, 0.07, 0.02 | 0.32, 0.035, 0.012 |
| `shallow_color` | 0.012, 0.060, 0.052 | 0.020, 0.090, 0.075 |

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
| W30 | How the water is coloured | a dark navy-grey open sea that reflects the sky, teal shallows over sand (step 0) | decided (owner, 2026-10-26: "1 and 2 are what I want. We can have both") |
| W29 | Which steps for the water | all five, in order (detail, sky, trace, glitter, scatter and shadow) | proposed (survey W29) |
