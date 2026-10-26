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
- [ ] 7.2 Near lace does not show yet in the `sea-away` still at 40 m (the threshold, 2.6 deviations, is too high: try 2.0-2.8); far gusts show as faint sheens. Re-shoot `sea-away`, `coast`, `cliffs-low`, `sea-sun-high` and look before calling it done
- [x] 7.3 Screen-space reflections: the rocks mirror in the water in the `cliffs-low` still
