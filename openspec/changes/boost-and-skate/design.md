# Design: boost-and-skate

Tuning in the mech's data (`jet:`, `skate:`, code defaults in `cb-core::mech::config`). Heat is the
share of the budget used, 0 to 1: time left is `(1 - heat) * burn_s`, time to recharge is
`heat * recharge_s` (after the delay).

| | Burn | Recharge | Delay | Locks out until | Top speed | Drive |
| --- | --- | --- | --- | --- | --- | --- |
| rockets | 20 s | 8 s | 1.5 s | 30 % back | 60 m/s climb, 60 m/s fall (90 m/s diving) | lift 56 m/s2 (TWR 5.7) |
| skates | 40 s | 10 s | 1.0 s | 20 % back | 120 m/s | 12 m/s2 x (1 - (v / 120)^2) |

## Per mech (survey C15)

Owner, 2026-10-26, on survey C15: "every mech has dash/skate, just diferent stats for them". Every roster mech has the
rockets (the dash) and the skates, on the one simulation above; only the numbers differ, and they live in each mech's
own file (`data/mechs/<id>.ron`, `jet:` and `skate:`), written by `tools/mechs/gen_mech_data.py` from its `MOBILITY`
table. The Foreman keeps the shipped tuning (the table above, the code defaults) and the rest are set round it by
build and job. My picks (the owner said "use recommended" for choices like these):

| Mech | Lift (m/s2) | Climb cap (m/s) | Burn | Recharge | Skate top (m/s) | Drive (m/s2) | Grip (/s) | Turn (deg/s) | Skate use | Recharge |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Foreman | 56 | 60 | 20 s | 8 s | 120 | 12 | 1.2 | 90 | 40 s | 10 s |
| Picket, light scout | 62 | 65 | 22 s | 7 s | 135 | 14 | 1.4 | 105 | 45 s | 9 s |
| Harrow, missile carrier | 60 | 65 | 24 s | 8 s | 125 | 13 | 1.2 | 90 | 40 s | 10 s |
| Roustabout, brawler | 52 | 55 | 18 s | 8 s | 115 | 12 | 1.3 | 85 | 40 s | 10 s |
| Driller, mining rig | 50 | 50 | 16 s | 9 s | 110 | 11 | 1.8 | 80 | 45 s | 10 s |
| Sapper, siege | 46 | 50 | 16 s | 10 s | 105 | 10 | 1.4 | 80 | 35 s | 12 s |
| Mule, hauler | 44 | 45 | 17 s | 10 s | 100 | 9 | 1.6 | 70 | 55 s | 12 s |
| Ironhorse, locomotive | 42 | 45 | 17 s | 11 s | 140 | 10 | 1.0 | 70 | 50 s | 10 s |

The weakest lift, the Ironhorse's 42 m/s2, is a thrust to weight of 4.3. The Mule and the Ironhorse were first given
14 s of burn; merged with the Stratostacean (survey H4, "reach it on the jets as they are") that left both overheated
and locked as they passed its deck, 649 m up, so each has 17 s: still the two slowest climbs (about 15 s to the deck),
with about 2 s of burn to spare, held by the fortress test `every_mech_reaches_the_deck_on_its_jets_as_they_are`. The cockpit's gauges already read the
piloted mech's own numbers (`hud.rs` reads `spec.jet` and `spec.skate`), so nothing else changes. Checks:
`every_roster_mech_has_rockets_and_skates_of_its_own` and `each_mech_skates_and_flies_on_its_own_budgets` in
`cb-core` (each mech's skates and rockets give out at its own seconds, never past its own top speed).

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
`v(t) = v_max * tanh(accel * t / v_max)` from a stand in the air: 55 m/s at 5 s, 91 m/s at 10 s, 109 m/s at
15 s. Easing off brakes at 15 m/s2. Velocity across the heading decays at the grip, 1.2 per second,
so a turn at speed slides before it bites; turn rate 90 degrees a second. On the ground the wheels push
`ground_accel_scale` (4) times as hard as in the air (owner, 2026-10-28: "4x the acceleration from skate if not in
the air"), along the legs' heading, as before; in the air the push is `accel` alone, along the view. Pose: pelvis 0.6 m lower,
feet staggered 1.6 m, no stepping. In the air with skates on, WASD thrusts along the view's heading
(forward, back, strafe) with the same curve along the thrust direction, and burns the skate budget
only while a direction is held. On the ground the skates burn while switched on.

## Open questions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| M3 | Flight limit | 20 s of boost, heat only | decided by the owner (2026-10-24) |
| M4 | Skates | foot wheels, 40 s, drifting, 120 m/s | decided by the owner (2026-10-24) |
| B1 | Rocket climb cap | 60 m/s with the doubled thrust | provisional |
| C15 | Does every mech get the rockets and skates | Yes, each with its own stats ("Per mech") | decided by the owner (2026-10-26): "every mech has dash/skate, just diferent stats for them" |
