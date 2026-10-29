# Tasks: close-combat

- [x] 1.1 The mode and its toggle, pure and tested (on in range, held off, by hand)
- [x] 1.2 Strikes: phases, the sweep, hits through the damage rules, tested
- [x] 2.1 Tab binding; the triggers strike in close combat; the MFD shows it
- [x] 2.2 The arm pose over the gait during a strike
- [x] 2.3a Stance designed in the Mech Arms Lab (`tools/mockups/mech_stance.js`): three blows break it, heavy blows count 1.5, it mends after 3 s
- [x] 2.3 Stance and knockdown (`cb_core::mech::knockdown`, `melee.stance_*`/`knockdown_*` in data, classes from `tools/mechs/gen_mech_data.py`; the blow in `cb-app` combat.rs, held and slid in `step_mech`, drawn in the mech draw; not yet seen running): each blow adds its move's stance; at `break_at` the struck mech falls, lies down and gets up (3.9 s), slid back along the blow, held still meanwhile; no stun lock
- [x] 2.4a The moves designed in the mech artifact's Melee lab: jab, hook, uppercut, overhead smash, shoulder charge, stomp kick, gun recoil (`tools/mockups/mech_melee.js`)
- [x] 2.4 The game plays the lab's moves on `shoulder_*` and `forearm_*` (not `upper_arm_*`, which carries no mesh), read from data written from the lab: `data/combat/melee_moves.ron` by `tools/mockups/export_melee_moves.mjs`, played by `cb_core::mech::melee_moves`; Z and V throw the jab (the other moves are in the data, not yet on keys)
- [x] 2.5 Each gun shot plays the recoil move on its arm (`recoil_L`/`recoil_R`, over the aimed arm; a back unit's or pod's own barrel does not kick)
- [ ] 3.1 Playtest: a close fight seen from the cockpit
- [x] 2.6 No close-combat mode (owner, 2026-10-29: "remove melee toggle, always allo z/v to punch"): Z and V punch at any time from the seat, the guns firing on; the Tab binding, `CloseCombat` and the MELEE MODE sign are gone
