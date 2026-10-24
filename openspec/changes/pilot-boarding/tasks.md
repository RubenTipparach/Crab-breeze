# Tasks: pilot-boarding

## 1. Core (`cb-core::pilot`)
- [x] 1.1 `PilotConfig` in `data/pilot/pilot.ron`, bindings in `data/input/bindings.ron` (test: `the_shipped_pilot_data_loads`)
- [x] 1.2 On foot (tests: `walking_forward_moves_along_the_view`, `the_pilot_cannot_walk_through_a_building`)
- [x] 1.3 Boarding (tests: `use_far_from_the_ladder_does_nothing`, `boarding_ends_seated_at_the_eye_point`, `the_boarding_path_is_continuous`, `disembarking_leaves_the_pilot_at_the_ladder`)

## 2. App
- [x] 2.1 Input actions plugin from bindings data
- [x] 2.2 One camera through every state
- [x] 2.3 `--stills <dir>` capture instrument with frame time

## 3. Proof
- [x] 3.1 `docs/screenshots/` street and cockpit stills
- [ ] 3.2 A playtest take: walk to the mech, board, stride (owner playing, or `playtest-pipeline`)
