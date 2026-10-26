# Tasks: water-look

- [x] 0.0 Colour (design section 0): `deep_color`, `absorption_per_m`, `sky_zenith_color`,
      `sky_horizon_strength`, and the shallows' `shallow_color` by `shallow_depth_m` in
      `data/world/water_look.ron`; stills of the open sea from the air and of the shallows, before and after
- [ ] 0.1 The island mockup's sea and lakes: layered wavelets, the sky and clouds reflected, GGX glitter, scatter
- [ ] 1.1 `tools/water/bake_spectrum.py`: the 64-frame JONSWAP atlas (test: it tiles, it loops)
- [ ] 1.2 Three scales over the swell, filtered by footprint, the lost variance into roughness
- [ ] 2.1 The sky cube from the eye, one face a frame, blended
- [ ] 3.1 The screen-space trace in `water_surface.wesl`, falling back to the cube
- [ ] 4.1 GGX over the sun's disc with Toksvig roughness
- [ ] 5.1 Scatter; the shadow map on glint and scatter
- [ ] 6.1 Headless stills of the sea at noon, against the sun, and with the mech wading; measured on the owner's hardware
