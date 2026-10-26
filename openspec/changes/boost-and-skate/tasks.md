# Tasks: boost-and-skate

- [x] 1.1 Budgets, burn and recharge (tests: `rockets_burn_for_twenty_seconds_then_lock_and_recharge`, `skates_last_forty_seconds_then_it_walks`)
- [x] 1.2 Rockets vertical only, thrust doubled; since the owner's 2026-10-25 swivel they push across only by their
  vectoring (test: `rockets_push_across_only_by_their_swivel`, which replaced `rockets_never_push_sideways`)
- [x] 1.3 Skates at 120 m/s on a fading curve, with drift (tests: `skating_builds_speed_over_time`, `a_skating_mech_slides_through_a_turn`)
- [x] 1.4 Skates in the air push relative to the view on the skate budget (test: `skates_strafe_in_the_air_on_their_own_budget`)
- [x] 1.5 Ctrl dives from the air (test: `ctrl_dives_faster_than_a_fall`)
- [ ] 2.1 Wheel sparks, braced pose; HUD gauges (cockpit-hud)
- [x] 3.1 Survey C15 (owner, 2026-10-26: "every mech has dash/skate, just diferent stats for them"): every roster
  mech's file sets its own `jet:` and `skate:` from `gen_mech_data.py`'s `MOBILITY` table (design, "Per mech");
  tests `every_roster_mech_has_rockets_and_skates_of_its_own`, `each_mech_skates_and_flies_on_its_own_budgets`
- [ ] 3.2 Playtest on the owner's machine: skate and fly the Picket, the Mule and the Ironhorse, read the gauges
