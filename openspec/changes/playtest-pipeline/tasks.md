# Tasks: playtest-pipeline

## 1. Game side
- [ ] 1.1 Input actions plugin, with the scripted pilot as a second writer of the same actions (test: an action written by the pilot moves the body as a key press does)
- [ ] 1.2 `--playtest <name>` loads `data/playtests/<name>.ron`, validated on load (test: an unknown beat stops startup with the file and line)
- [ ] 1.3 Beats `WalkTo`, `LookAt`, `Wait`, with stuck detection (test: a blocked `WalkTo` ends `stuck` after 10 s game time)
- [ ] 1.4 The output lines and the JSON report (test: lines appear in order for a two-beat script)
- [ ] 1.5 `Board`, `PowerUp`, `StrideTo`, `FireAt`, `Disembark` once the stand-in mech exists; `SkateTo` and `JetTo` once skating and flight exist

## 2. Tools
- [ ] 2.1 `tools/playtest.py` verdict run and `--record` through `obs_record.py`
- [ ] 2.2 Share copy and Drive copy, report page in `docs/playtests/`
- [ ] 2.3 `--stills` on lavapipe in a cloud session

## 3. Proof
- [ ] 3.1 The `boarding` script in the grey test yard, recorded on the owner's machine, on Drive, its report committed
