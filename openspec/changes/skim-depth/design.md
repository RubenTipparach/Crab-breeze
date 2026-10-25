# Design: skim-depth

- `skimmable = depth > skim_depth_share * body_height_m` (locomotion config).
- `rising = eye below the surface by more than float_catch_m && skates on && skimmable && not in
  the air`.
- `skim = (at_surface || rising) && !sink && skate_power && skimmable`.
- While skimming below the surface, `y += skim_rise_m_s * dt` until the surface, then it rides it.
- `deep` (past `max_wade_depth_m`) still decides hovering on the rockets and sinking.
- Data: `skim_depth_share`, `skim_rise_m_s` in each mech's `locomotion`. A mech file without them
  takes 0.5 and 6 m/s.
