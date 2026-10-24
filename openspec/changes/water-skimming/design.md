# Design: water-skimming

Pure, in `cb-core::mech::locomotion`, on the one water query (`Ground::water_at`: surface height
and depth). Tuning in the mech's `locomotion:` data.

## Rules each step

1. **At the surface** means the root is no more than `float_catch_m` (1.0 m) below the water
   surface. **Deep** means water deeper than `max_wade_depth_m` (8 m); shallower water is waded.
2. **Skim** when at the surface of deep water, Ctrl is not held, both legs are on and the skate budget
   is not locked; the skates switch on by themselves. Water shallow enough to wade is waded with the
   skates on or off (owner, 2026-10-24: "shallow water dont need autoskate"). The water
   surface is the floor; the skates drive as on land and burn their budget.
3. **Rocket backup** when at the surface of deep water, Ctrl is not held, there is no skim, and the
   rockets are not locked: the rockets fire (they burn as boosting), the surface is the floor, and
   the mech moves at up to walking speed along its heading on the throttle.
4. **Sink** otherwise in deep water: vertical speed eases to -`water_sink_m_s` (4 m/s), horizontal
   speed drags off at 1 per second, and on the bottom the mech walks with the full wading slowdown
   (40 % of walking speed). Budgets recharge as anywhere else when not in use.
5. **No cooling afloat**: while the mech rides the water (skimming or on the rocket backup), a
   budget not in use neither burns nor recharges, so the two cannot take turns forever; they
   recharge once the mech has sunk (or is back on land).
6. **Rising**: Space under water lifts on the rockets, capped at `water_climb_m_s` (10 m/s). At the
   surface rule 2 or 3 takes over.
7. **Ctrl** in the air adds `jet.dive_m_s2` (30 m/s2) downward and raises the fall cap from
   `jet.fall_max_m_s` (60 m/s) to `jet.dive_max_m_s` (90 m/s). On water it stops rules 2 and 3.

From full, a mech left on deep water skims 40 s, hovers on the rockets 20 s, then sinks; at 4 m/s it
reaches a 40 m sea floor in 10 s.

## Open questions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| WS1 | Do the skates switch on by themselves on deep water? | yes, so landing on the sea skims | provisional |
| WS2 | Movement on the rocket backup | walking speed on the throttle, along the legs' heading | provisional |
