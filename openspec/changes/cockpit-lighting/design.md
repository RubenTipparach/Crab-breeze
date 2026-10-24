# Design: cockpit-lighting

## 1. Separation (built)

- The sun is on render layer 0 only; effect lights (`fx` flashes, rocket motors) are on layer 0 by
  default. Bevy lights a view only with lights whose layers it shares, so none reaches the cockpit
  pass (layer 1).
- The cockpit camera overrides the global ambient with its own `AmbientLight`
  (`cabin_ambient`, a dim warm grey) so the cabin never goes black.
- Three screen-glow point lights on layer 1, children of the eye camera: 1.2 m ahead, and 1.0 m to
  each side 0.6 m ahead, 200 lm each, range 4 m, tint (0.75, 0.88, 1.0). Numbers in
  `data/hud/cockpit_light.ron`.

## 2. The screens' cube map (next)

- Six 64 x 64 faces rendered from the eye by one extra camera on layer 0 that turns to a new face
  each frame (no shadows, no fog, no particles past 50 m), into a cube `Image`; the cockpit
  camera's `GeneratedEnvironmentMapLight` filters it on the GPU each frame.
- The faces the cockpit has no screen on (back, floor) are dimmed to 20 % before filtering, as the
  cabin walls block them.
- Cost: one 64 x 64 view a frame (about 0.1 ms), the filter pass (Bevy's, about 0.2 ms at 64 px);
  measured on the RTX 3070 with `--bench defence` before and after.

## Open questions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| C10 | How bright the screens light the cabin | Enough to read the panels by day, a soft glow by night; the cabin lamps stay dim | provisional (survey C10) |
