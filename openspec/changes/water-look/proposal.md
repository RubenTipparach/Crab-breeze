# Proposal: water-look

## Why

Owner, 2026-10-25, with a Space Engineers 2 screenshot of a ship on a calm sea: "how can we get
water like this?"

What the reference has that our water (`water-rendering`, `assets/shaders/water_surface.wesl`)
does not:

| In the reference | Ours today |
| --- | --- |
| Dense small wavelets at every scale, so the surface is busy up close and still detailed far off | one noise fbm of ripples plus three swell components |
| The real sky in the water: clouds, the bright horizon haze | a two-colour gradient (horizon to zenith) |
| The ship reflected under itself | nothing above the water is reflected |
| A long, wide glitter path toward the sun made of thousands of glints | one Blinn-Phong highlight, a few sparkles |
| Turquoise light through the sides of wavelets, dark blue-green beneath | absorption only, no scattering |
| The ship's shadow and contact on the water | none |

Most of the parts are there already: refraction of the scene behind, depth absorption, Fresnel,
flow, foam. What is missing is the light on the surface.

## What changes

Five steps, each usable alone, in `water_surface.wesl`:

1. **Wave detail:** a tileable, animated normal map baked from an ocean spectrum (JONSWAP, the FFT
   method of Tessendorf's "Simulating Ocean Water"), sampled at three scales and scrolled apart,
   over the existing swell. It is filtered with distance so far water keeps its roughness instead
   of sparkling or going flat.
2. **The sky in the water:** reflect a cube map of the game's sky with its clouds, captured from
   the eye every 2 s and blended, instead of the two-colour gradient.
3. **Things in the water:** a screen-space reflection traced in the water shader itself, against
   the depth prepass and the scene copy the water already reads. The mech, a hull, the shore and
   buildings reflect; where the trace misses, the sky cube shows.
4. **The sun's glitter path:** a GGX specular whose roughness grows with the normal map's filtered
   variance (Toksvig), so the many wavelets give a long, broad path toward the sun.
5. **Light through the water:** a subsurface term that lights wavelet flanks turned away from the
   sun turquoise; the sun's shadow map dims glint and scatter under a hull or a mech.

The island mockup shows steps 1, 2 (its sky and billboard clouds), 4 and 5 on its sea and lakes.

## Out of scope

Displaced geometry for big waves and boats riding them, wakes (`water-ripples` covers the mech's),
and underwater god rays.
