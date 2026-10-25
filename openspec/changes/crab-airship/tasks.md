# Tasks: crab-airship

- [x] 0.1 Mockup: `docs/mockups/crab_fortress.html` (`tools/mockups/gen_crab_fortress.py`): the block-out at true scale with a 24 m Foreman and the crab mechs; fighters launching off the catapults on patrol and cannon runs; turrets tracking and firing at your mech (flak, beams, deck guns, missiles); the deck guard fighting and replaced up the lifts; crabs dropped on chutes; your mech's cannon, autocannon and missiles against each part's armour, weak points and hit points until it falls and crashes, also from your cockpit; shots in `docs/screenshots/fortress/`
- [ ] 1.1 `data/world/fortress.ron` and its validation
- [ ] 1.2 `cb-core::fortress`: the lift rule, pose, list and fall (tests: `a_side_with_two_thrusters_holds_its_half`, `three_kills_on_one_side_bring_it_down`, `two_kills_a_side_hold_at_full_throttle`, `a_falling_fortress_lists_toward_its_weak_side`, `the_sink_speed_never_passes_its_terminal_speed`)
- [ ] 1.3 Launch and drop timers (tests: `a_lost_catapult_halves_the_launches`, `no_drops_without_a_bay`, `nothing_launches_while_falling`)
- [ ] 1.4 Moving supports (test: `a_mech_standing_on_the_drifting_deck_stays_where_it_stood`, `a_mech_slides_off_past_its_grip`)
- [ ] 1.5 Downwash (test: `a_mech_under_a_live_thruster_is_pushed_down_and_burned`)
- [ ] 2.1 `cb-app::fortress`: the block-out in the game, parts as entities with hit shapes and targeting boxes (test: `the_autocannon_cannot_hurt_a_cowl_but_doubles_on_a_grille`)
- [ ] 2.2 The cockpit lift readout from the rule (test: `the_lift_readout_is_the_rule`)
- [ ] 2.3 Turrets, beams, deck guns and missile batteries on the existing weapons; the deck guard on the crab brain
- [ ] 2.4 The fall: sliding off, breaking up, the crash and the wreck
- [ ] 2.5 Wave 3 of the defence brings the fortress (G16)
- [ ] 3.1 The Blender block-out, then the detailed model with LODs (after the owner picks from the mockup)
- [ ] 4.1 Playtest beats: climb to the deck, land, fight the guard, walk a pylon, kill three thrusters on one side, jump clear, watch it fall; measure with `--bench defence`
