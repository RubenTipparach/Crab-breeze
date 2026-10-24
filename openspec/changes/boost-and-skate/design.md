# Design: boost-and-skate

Tuning in the mech's data (`jet:`, `skate:`, code defaults in `cb-core::mech::config`). Heat is the
share of the budget used, 0 to 1: time left is `(1 - heat) * burn_s`, time to recharge is
`heat * recharge_s` (after the delay).

| | Burn | Recharge | Delay | Locks out until | Top speed | Drive |
| --- | --- | --- | --- | --- | --- | --- |
| rockets | 20 s | 8 s | 1.5 s | 30 % back | 60 m/s climb, 60 m/s fall (90 m/s diving) | lift 56 m/s2 (TWR 5.7) |
| skates | 40 s | 10 s | 1.0 s | 20 % back | 120 m/s | 12 m/s2 x (1 - (v / 120)^2) |

## Controls (owner, 2026-10-24)

| Key | On the ground | In the air |
| --- | --- | --- |
| Space (hold) | rockets lift off | rockets lift: climb, or hold against a fall |
| Shift (press) | skates on or off | skates on or off |
| WASD, skates off | walk: W/S throttle lever, A/D turn the legs | nothing: the mech coasts (drag 0.3/s) |
| WASD, skates on | wheels: W/S throttle lever, A/D turn the legs | skate thrust relative to the view |
| Ctrl (hold) | (water: sink, see `water-skimming`) | dive: 30 m/s2 extra down |

Rockets never push sideways and skates never lift. The rocket's TWR doubled from 2.85 to 5.7, and
the climb cap went from 40 to 60 m/s with it, so the extra thrust shows past the first second (my
call: the owner asked for the TWR only).

## Skate drive

Along the heading, `a = accel * (1 - (v / v_max)^2)` toward `throttle * v_max`, so
`v(t) = v_max * tanh(accel * t / v_max)` from a stand: 55 m/s at 5 s, 91 m/s at 10 s, 109 m/s at
15 s. Easing off brakes at 15 m/s2. Velocity across the heading decays at the grip, 1.2 per second,
so a turn at speed slides before it bites; turn rate 90 degrees a second. Pose: pelvis 0.6 m lower,
feet staggered 1.6 m, no stepping. In the air with skates on, WASD thrusts along the view's heading
(forward, back, strafe) with the same curve along the thrust direction, and burns the skate budget
only while a direction is held. On the ground the skates burn while switched on.

## Open questions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| M3 | Flight limit | 20 s of boost, heat only | decided by the owner (2026-10-24) |
| M4 | Skates | foot wheels, 40 s, drifting, 120 m/s | decided by the owner (2026-10-24) |
| B1 | Rocket climb cap | 60 m/s with the doubled thrust | provisional |
