# Tasks: mech-boarding

## 1. Art and mockup (cloud session)

- [ ] 1.1 Pod: close the cockpit over with the all-view screen shell and its seam ribs
- [ ] 1.2 Hatch nodes on the pod: `LATCH_*`, `HATCH_upper`, `FLAP_L/R`, `HATCH_lower`, `SHUTTER_1..4`, `STEP_1..3`, `CONSOLE_front`, `VENT_*`
- [ ] 1.3 Breeze: the chest takes the pod in place of the bubble canopy
- [ ] 1.4 Mockup: the sit pose, the hatch timeline, steam and mist, the all-view screen, the pilot's path in first person

## 2. Engine (owner's machine)

- [ ] 2.1 `data/mechs/breeze/boarding.ron`: the timeline from the mockup, validated on load
- [ ] 2.2 `cb-core::boarding`: the stage machine (closed, sitting, opening, open, boarding, closing, sealed, and back), pure and tested
- [ ] 2.3 `cb-app`: animate the nodes from the timeline, steam and mist particles, the all-view cube render and screen material
- [ ] 2.4 Replace `pilot-boarding`'s lift path with the stair path
- [ ] 2.5 Measure the all-view screen's cost in the cockpit (perf-measure)
- [ ] 2.6 Playtest: walk up, board, seal, power up, disembark, recorded and on Drive
