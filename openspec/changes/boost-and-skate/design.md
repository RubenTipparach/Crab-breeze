# Design: boost-and-skate

Tuning in the mech's data (`jet:`, `skate:`). Heat is the share of the budget used, 0 to 1: time
left is `(1 - heat) * burn_s`, time to recharge is `heat * recharge_s` (after the delay).

| | Burn | Recharge | Delay | Locks out until | Top speed | Accel |
| --- | --- | --- | --- | --- | --- | --- |
| boosters | 20 s | 8 s | 1.5 s | 30 % back | 45 m/s horizontal, 40 m/s climb | 30 m/s2, lift 28 m/s2 |
| skates | 40 s | 10 s | 1.0 s | 20 % back | 30 m/s | 10 m/s2 |

Skates: velocity eases toward the heading at the target speed with a grip of 1.2 per second, so a
sharp turn slides before it bites; turn rate 90 degrees a second. Pose: pelvis 0.6 m lower, feet
staggered 1.6 m, no stepping. Boosters: horizontal thrust along the view (WASD), lift on Space,
air drag 0.3 per second without thrust.

## Open questions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| M3 | Flight limit | 20 s of boost, heat only | decided by the owner (2026-10-24) |
| M4 | Skates | foot wheels, 40 s, drifting | decided by the owner (2026-10-24) |
