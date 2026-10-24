# Design: jet-flight

Tuning in the mech's data, `jet:` (defaults in `cb-core::mech::JetConfig`).

| Number | Default | Why |
| --- | --- | --- |
| `thrust_m_s2` | 28 | 2.9 g of lift: a brisk climb against 9.81 m/s2 |
| `climb_max_m_s` | 40 | reach the cloud base (600 m) in about 17 s |
| `speed_max_m_s` | 60 | 216 km/h in the air, about 8x walking |
| `heat_rise_per_s` | 0.04 | 25 s of full lift to the limit |
| `cool_per_s` | 0.08 | back to relight (0.6) in 5 s from the limit |

Each fixed step: heat rises with lift, or cools; vertical speed gains `thrust * lift - g`, capped;
the mech stands on `support_at` (ground or a roof no higher than its feet plus 1 m), and walls are
only the buildings whose roofs are above its feet (`push_out_at`). Cost: microseconds a step.

## Open questions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| M3 | What limits flight | heat now, fuel later | decided by the owner |
| G5 | Assisted flight | a hover-hold assist later | provisional (survey G5 decided assists, the change to come) |
