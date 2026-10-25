# Design: cloud-rain

- `cloud_glass(prev, density, dt_s, look) -> f32` (`cb-core::water_look`):
  - target = `cloud_rain * clamp(density)`;
  - it rises at `1 / cloud_wet_s` a second toward a higher target and falls at `1 / cloud_dry_s`
    toward a lower one;
  - non-finite inputs are treated as dry.
- `water_render::track_eye` keeps it per camera (`WaterEye::rain`) from `Clouds::density_at` at the
  eye.
- The lens pass reads `max(lens_rain, rain)` and runs while it is above zero, so the drops cost
  nothing in clear air.
- Data: `data/world/water_look.ron` `cloud_rain`, `cloud_wet_s`, `cloud_dry_s`.
- Cost: the lens pass (already measured with surfacing) only while wet.

## Open questions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| T4 | Pale Blue Dot raindrops | the ported lens pass, now fed by cloud too | done in this change (survey T4) |
