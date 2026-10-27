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
| `sky_zenith_color` | 0.03, 0.18, 0.55 | 0.16, 0.22, 0.30 (main's, at the merge below) | a hazy grey-blue like the sky above, not a saturated constant |

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

**Merged with main's darker water.** Another session answered the owner's earlier "water is also too
blue, not relaistic at all" (2026-10-25) on main with a dark blue-green body (0.01, 0.05, 0.065),
absorption (0.45, 0.10, 0.08), a grey zenith (0.16, 0.22, 0.30) and `sky_horizon_strength` 0.5. The
owner's later choice (2026-10-26, a navy-grey open sea and teal shallows) keeps this section's body,
shallows and absorption; main's greyer zenith (0.16, 0.22, 0.30) is taken, as it is the "greyer" of
that choice; `sky_horizon_strength` stays 0.7 from the stills above.

**Still dull (owner, 2026-10-26: "Not sure that water convinces me.... It looks kinda dull", then
"Maybe need reflection probe like glass towers?").** Colour alone could not fix it: the sun's
glint was one tight Blinn-Phong dot, the sky in the water a two-colour gradient, nothing lit the
crests. Built now, ahead of the wave detail:
- **Step 4, the glitter path:** GGX with water's Fresnel of 0.02, as section 4 has it, its
  roughness `glint_roughness` (0.06) close by plus `glint_far_roughness` (0.25) where the ripples
  have faded into the distance (the shader's own detail fade stands in for the Toksvig term until
  the wave atlas exists). `specular_intensity` is now a multiplier on that highlight (1.0);
  `specular_power` is gone.
- **Step 2, the sky in the water, from the glass's probe:** the water samples the pilot camera's
  cached sky cube (`sky_light.rs`, the Poly Haven sky the buildings' glass reflects) in its
  reflected direction, at the mip its roughness asks for, so water and glass mirror one sky. The
  game's own cloud billboards are not in that cube; capturing them stays step 2's later part.
- **Step 5, light through the crests:** `scatter_color` (0.05, 0.35, 0.33) times
  `scatter_strength` (0.35), on wavelet crests (`max(h + 0.15, 0)`) seen toward the sun's bearing
  (`pow(toward, 4)`), where the surface is not mirroring. The shadow under hulls is still to come.
- Stills `sea-sun` and `sea-away` (40 m over open sea, toward and away from the sun):
  `docs/screenshots/2026-10-26-water-glint.png`.

**Glitter from high up (owner, 2026-10-26: "its definitely the glitter that improves it", "is
there a way we can make glitter show up more at higher altitudes? lower altitudes look great",
with Ace Combat 8 shots of the sea sheened and sparkling under the sun from altitude).** From 6.5 km
the sea was a flat navy sheet: the glint's roughness grew only where the ripples had faded, and a
pixel there covers 5-10 m of sea, whose wavelets a pixel cannot draw.
- **The sheen's width, by measurement:** Cox and Munk's sea-surface slopes (the standard model of
  sun glint seen from aircraft and satellites) give a slope variance of `0.003 + 0.00512 x wind`,
  0.034 at the game's 6 m/s wind (`cloud-weather`), a GGX roughness of about 0.26. Far water's
  total roughness (`glint_roughness` 0.06 + `glint_far_roughness` 0.20) is set to that.
- **The sparkle:** a smooth broad lobe is a sheen, not glitter. Each patch of sea a pixel and a half
  across takes its own random facet tilt, `glitter_slope` (0.22) times how far the ripples have
  faded, re-drawn `glitter_twinkle_hz` (3) times a second; the sun's glint is drawn off that tilted
  facet with a narrower lobe (half the far roughness), so far water sparkles in a sheen as wide as
  Cox and Munk's. The sky's reflection keeps the untilted surface.
- Stills: `sea-sun-high` (1.5 km up) and `sea-sun-top` (6.5 km), toward the sun.
- Cost: two hashes and a second normal a pixel.

**Foam and things in the water (owner, 2026-10-26: "theres sea foam, and screenspace reflections
which make it look nice too", with an Ace Combat 7 shot of a harbour; then "ok looks good, keep
going").**
- **Step 3, the trace, built as section 3 has it:** Bevy 0.20's own screen-space reflections run on
  the deferred path's gbuffer, and the water is a transmissive forward material, so the water traces
  itself. From the surface along the reflected ray, `reflect_steps` (24) steps that grow by a fixed
  factor from 1 m so the last reaches `reflect_max_m` (3,000 m) (the factor is 1.334); each step is
  projected to the screen and compared with the depth prepass (the water is not in it, so it never
  reflects itself). A hit is the first step behind the ground by less than its own length plus
  2 m, refined by four halvings, its colour read from the scene copy the water already reads. It
  fades out over the screen's outer tenth, with the steps it took, and toward reflected rays that
  climb steeply (those see sky); `reflect_strength` (1.0) scales it. Where it misses, the sky cube
  shows. What reflects: the ground, the coast, buildings, the mech, a hull; not the clouds (the
  cube's sky stands in for them).
- **Whitecaps:** at the 6 m/s wind real whitecaps cover about 0.2 % of the sea (Monahan's
  `3.84e-6 U^3.41`); the references show more, and foam that reads from the air is what sells it.
  The foam is the sea's own waves breaking, never patches laid on it: hashed cells of foam, and then
  thresholded noise blobs, read as "floating dots" (owner, 2026-10-26: "that seafoam isnt part of
  the noise stuff on the ocean lol"). A slow noise of gusts, `whitecap_scale_m` (40 m) across the
  wind and twice that along it, drifts with the wind; its threshold is set from the noise's measured
  spread (standard deviation 0.166 over 400,000 samples) so `whitecap_coverage x 6` of the sea is in
  a gust. A gust only shortens the odds: the foam is the wavelets the eye sees, where their height
  in their own deviations, lifted by the swell's crests (`0.8 x` the swell's height in amplitudes)
  and by the gust (up to one deviation), passes `whitecap_break_dev` (2.6 as first built, 2.0 since 2026-10-27). In a gust's heart on a crest about 20 % of the
  wavelets foam, in an ordinary gust 2 %, outside one none: lace in the waves' own pattern, torn
  into streaks along the wind. Where the wavelets fade with distance, a gust shows as a faint sheen
  (18 %). The shore's surf band widens from 1.5 m to 3 m of water (`shore_foam_m`).
- Cost: up to 24 projected depth reads and four refinements a water pixel (section 6 had
  0.4-0.6 ms at half the screen); the foam is one noise a pixel. Measured on the owner's machine.

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
