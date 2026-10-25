# Tasks: mech-boarding

## 1. Art and mockup (cloud session)

- [x] 1.1 Three chassis with a chest bay and opening armour (Foreman, Stevedore, Ironhorse: `build_mechs.py`, `mech_boarding.py`; original designs after the owner's "New mechs are just stolen")
- [x] 1.2 Chest nodes: `CHEST_*`, `RAM_*`, `LADDER`, `LADDER_*`, `VENT_*`, the `boarding` root extras
- [x] 1.3 Three cockpit interiors in the bays (`build_cockpit.py --variant`), with their inner tracks
- [x] 1.4 Mockup: the kneel, the chest opening with rams, steam and mist, the ladder, the pilot's path in first person (`docs/mockups/mech_designs.html`)

## 2. Engine (owner's machine)

- [ ] 2.0 Move the timeline to the chosen chassis (survey M6, C8): its chest tracks and ladder replace the Breeze pod's
- [x] 2.1 `data/mechs/breeze/boarding.ron`: the timeline from the mockup, validated on load (test: `the_shipped_timelines_load`)
- [x] 2.2 `cb-core::boarding`: the stage machine (closed, sitting, opening, open, boarding, closing, sealed, and back), pure and tested: one timeline clock run toward a target (tests: `opening_sits_then_opens_by_ready_time`, `closing_is_the_opening_in_reverse`, `standing_up_takes_the_sit_in_reverse`; sit pose: `sitting_lowers_the_pelvis_lays_the_hock_back_and_keeps_the_feet`)
- [ ] 2.3 `cb-app`: animate the nodes from the timeline (done: nodes matched by name or `PREFIX_*`, turned about their origin and slid in their own frame; waits on the pod glb), steam and mist particles, the screens drawn from the sensor head (the cube map is for the cabin's reflections only), the interior parts carried on the chest joints named in `cockpit.rides`
- [x] 2.4 Replace `pilot-boarding`'s lift path with the stair path; one Use asks the mech to sit and open and boards when the stair is down (tests: `use_at_a_closed_mech_asks_it_to_open_and_boards_once_it_has`, `walking_away_stops_waiting_for_the_hatch`)
- [x] 2.7 Kneel as far as the legs can (the mockup's pose, from the generator) and climb a rope to the hatch (test: `boarding_climbs_straight_up_the_rope`)
- [ ] 2.5 Measure the all-view screen's cost in the cockpit (perf-measure)
- [ ] 2.6 Playtest: walk up, board, seal, power up, disembark, recorded and on Drive
