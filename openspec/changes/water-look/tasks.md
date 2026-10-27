# Tasks: water-look

- [x] 0.0 Colour (design section 0): `deep_color`, `absorption_per_m`, `sky_zenith_color`,
      `sky_horizon_strength`, and the shallows' `shallow_color` by `shallow_depth_m` in
      `data/world/water_look.ron`; stills of the open sea from the air and of the shallows, before and after
- [ ] 0.1 The island mockup's sea and lakes: layered wavelets, the sky and clouds reflected, GGX glitter, scatter
- [ ] 1.1 `tools/water/bake_spectrum.py`: the 64-frame JONSWAP atlas (test: it tiles, it loops)
- [ ] 1.2 Three scales over the swell, filtered by footprint, the lost variance into roughness
- [x] 2.0 The sky in the water from the glass's cached sky cube (design section 0, "Still dull")
- [ ] 2.1 The sky cube from the eye, one face a frame, blended (the game's own clouds in it)
- [ ] 3.1 The screen-space trace in `water_surface.wesl`, falling back to the cube (design section 0, "Foam and things in the water")
- [ ] 3.2 Whitecaps and a wider surf band
- [x] 4.0 GGX glint, roughness from the detail fade (design section 0, "Still dull")
- [ ] 4.1 GGX over the sun's disc with Toksvig roughness (once the wave atlas exists)
- [x] 5.0 Scatter on crests toward the sun (design section 0, "Still dull")
- [ ] 5.1 The shadow map on glint and scatter
- [ ] 6.1 Headless stills of the sea at noon, against the sun, and with the mech wading; measured on the owner's hardware

## 7. Foam (paused 2026-10-26)

- [x] 7.1 Whitecaps from the wave field: the tallest visible wavelets, lifted by swell crests and wind gusts (design section 0, "Whitecaps")
- [ ] 7.2 Near lace does not show yet in the `sea-away` still at 40 m. The threshold is data now (`whitecap_break_dev`, the uniform's `light.w`) and is set to 2.0, the most lace of the 2.0-2.8 asked for; re-shot `sea-away`, `coast`, `cliffs-low`, `sea-sun-high` at 2.6, 2.3 and 2.0 (and 1.6 and 1.2 to see the trend) on 2026-10-27 and looked: the sea never whitens, but near lace stays a few flecks (bright foam pixels in `sea-away`'s near water 0.002 % at 2.6, 0.018 % at 2.0, 0.19 % even at 1.2), because only the gusts foam at all and they are `6 x whitecap_coverage` (4.8 %) of the sea; in `sea-away` one gust sits at the top right. Stills: `docs/screenshots/2026-10-27-foam-*.png`. Left open: the lever for lace in the near water is the gusts' coverage (or letting a little foam break outside a gust), not the threshold; that is a look decision for the owner
- [x] 7.3 Screen-space reflections: the rocks mirror in the water in the `cliffs-low` still
