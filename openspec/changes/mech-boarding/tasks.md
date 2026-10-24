# Tasks: mech-boarding

## 1. Art and mockup (cloud session)

- [ ] 1.1 Pod: close the cockpit over with the all-view screen shell and its seam ribs
- [ ] 1.2 Hatch nodes on the pod: `LATCH_*`, `HATCH_upper`, `FLAP_L/R`, `HATCH_lower`, `SHUTTER_1..4`, `STEP_1..3`, `CONSOLE_front`, `VENT_*`
- [ ] 1.3 Breeze: the chest takes the pod in place of the bubble canopy
- [ ] 1.4 Mockup: the sit pose, the hatch timeline, steam and mist, the all-view screen, the pilot's path in first person

## 2. Engine (owner's machine)

- [x] 2.1 `data/mechs/breeze/boarding.ron`: the timeline from the mockup, validated on load (test: `the_shipped_timelines_load`)
- [x] 2.2 `cb-core::boarding`: the stage machine (closed, sitting, opening, open, boarding, closing, sealed, and back), pure and tested: one timeline clock run toward a target (tests: `opening_sits_then_opens_by_ready_time`, `closing_is_the_opening_in_reverse`, `standing_up_takes_the_sit_in_reverse`; sit pose: `sitting_lowers_the_pelvis_lays_the_hock_back_and_keeps_the_feet`)
- [ ] 2.3 `cb-app`: animate the nodes from the timeline (done: nodes matched by name or `PREFIX_*`, turned about their origin and slid in their own frame; waits on the pod glb), steam and mist particles, the all-view cube render and screen material
- [x] 2.4 Replace `pilot-boarding`'s lift path with the stair path; one Use asks the mech to sit and open and boards when the stair is down (tests: `use_at_a_closed_mech_asks_it_to_open_and_boards_once_it_has`, `walking_away_stops_waiting_for_the_hatch`)
- [ ] 2.5 Measure the all-view screen's cost in the cockpit (perf-measure)
- [ ] 2.6 Playtest: walk up, board, seal, power up, disembark, recorded and on Drive
