# Design: mech-ramming

## 1. The rule

`cb_core::mech::RamConfig::hole_radius_m(before_m_s, after_m_s) -> Option<f32>`:

- `None` unless `|before| >= speed_m_s` and `|after| < |before| x stopped_fraction`.
- Otherwise `min(radius_m x (|before| / speed_m_s)^(2/3), radius_max_m)`.

At 20 m/s: 4.5 m. At 55 m/s (skating after 5 s): 4.5 x 2.75^(2/3) = 8.8 m. At 120 m/s
(skating flat out): capped at 9 m.

The app (`mech.rs`) keeps where the hole is (2.5 m ahead of the body, 5 m up) and what follows
(the cells in the sphere break, the chunks fly, the mech keeps `kept_fraction` of its speed). It
calls the rule instead of its inline test.

## 2. Data

`ram:` in each mech file, defaults in code:

| Key | Default | Meaning |
|---|---|---|
| `speed_m_s` | 20.0 | least speed that damages a building |
| `stopped_fraction` | 0.6 | the wall must take the speed below this fraction in one step |
| `radius_m` | 4.5 | the hole at the ram speed |
| `radius_max_m` | 9.0 | the biggest hole |
| `kept_fraction` | 0.7 | the speed kept after smashing through |
| `reach_m` | 2.5 | the hole's centre ahead of the body's front |
| `height_m` | 5.0 | the hole's centre over the ground |

## 3. Cost

One comparison per mech per step; a ram costs what a hit already costs (one `break_sphere`).

## Open questions

| Id | Question | Recommendation | State |
|---|---|---|---|
| RM1 | Does ramming hurt the mech? (survey W23) | Yes, later, with the damage model's collision pass: torso structure by the same energy | decided: no, ramming is free (W23) |
| RM2 | 20 m/s as the ram speed? | Yes: above the 12 m/s walk, reached in under 2 s of skating | provisional |
