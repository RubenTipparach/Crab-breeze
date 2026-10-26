# Tasks: playtest-pipeline

## 1. Game side
- [ ] 1.1 Input actions plugin, with the scripted pilot as a second writer of the same actions (test: an action written by the pilot moves the body as a key press does)
- [x] 1.2 `--playtest <name>` loads `data/playtests/<name>.ron`, validated on load (test: an unknown beat stops startup with the file and line)
- [ ] 1.3 Beats `WalkTo`, `LookAt`, `Wait`, with stuck detection (test: a blocked `WalkTo` ends `stuck` after 10 s game time)
  (2026-10-26: built for the career loop, `crates/cb-app/src/playtest.rs` on `cb_core::playtest`: `--playtest <name>
  [--run <i>]`, a script of runs each with its `--play` mode, beats Wait, LookAround, WalkToTerminal, UseTerminal, Tab,
  AcceptJob, CloseTerminal, Board, GoToJob, FightJob, EnterBay, Disembark, Grant, BuyMech, ExpectOffer, BuyPart, Fit,
  Quit, ExpectRestored; the pilot writes input actions, key presses and the mouse's pointer messages on menu buttons;
  stuck after `stuck_s` of game time with no progress, or 20 s for a press or a click. The stuck rule has no test of
  its own yet; no generic WalkTo/LookAt markers; no JSON report)
- [ ] 1.4 The output lines and the JSON report (test: lines appear in order for a two-beat script)
- [ ] 1.5 `Board`, `PowerUp`, `StrideTo`, `FireAt`, `Disembark` once the stand-in mech exists; `SkateTo` and `JetTo` once skating and flight exist

## 2. Tools
- [ ] 2.1 `tools/playtest.py` verdict run and `--record` through `obs_record.py`
- [ ] 2.2 Share copy and Drive copy, report page in `docs/playtests/`
- [ ] 2.3 `--stills` on lavapipe in a cloud session

## 3. Proof
- [ ] 3.1 The `boarding` script in the grey test yard, recorded on the owner's machine, on Drive, its report committed
