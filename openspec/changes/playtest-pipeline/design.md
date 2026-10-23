# Design: playtest-pipeline

## 1. The scripted pilot

One scripted pilot (after Star Crew's `sc-client --playtest`). It owns no rules: each fixed step
it writes the same action state the input plugin would write from a keyboard or pad (move, look,
use, throttle, torso twist, fire groups). The game cannot tell it from a player.

A script is a RON list of beats:

| Beat | Does |
| --- | --- |
| `WalkTo(marker)` | walk on foot to a named marker by the navigation the pilot's body uses |
| `Board` | use the mech's ladder or hatch and sit (the camera goes to `EYE_pilot` in one motion) |
| `PowerUp` | press the cockpit's start sequence controls in order |
| `StrideTo(marker)` | drive the mech to a marker with throttle and steering |
| `LookAt(marker, s)` | turn the head toward a marker and hold for `s` seconds |
| `SkateTo(marker)` | switch to skating and drive to a marker on the ground |
| `JetTo(marker, altitude_m)` | lift off on the jets, fly to a marker at an altitude, and land |
| `FireAt(marker, group, s)` | aim and fire a weapon group for `s` seconds |
| `Disembark` | stand and climb out |
| `Wait(s)` | do nothing for `s` seconds |

A beat with no progress (distance to goal not shrinking by 0.1 m) for 10 s of game time is
`stuck`. Every beat has a time cap in its data.

## 2. Output lines and the report

Printed on stdout, flushed per line, exactly as the `playtest` skill lists them. The report is
`output/playtests/<name>-<seed>.json`: each beat's start and end time (game seconds), outcome,
the pilot's position each second, and frame time percentiles (only meaningful on real hardware).

## 3. Recording and Drive

`tools/playtest.py <name> [--record] [--no-upload]` runs the release build once to a verdict, then,
with `--record`, again under `obs_record.py` (`--capture game --start-on ^PLAYTEST_START --stop-on
^PLAYTEST_END --contact-sheet`), re-encodes a share copy (H.264, CRF 26, preset slow, no audio
unless the script asks), copies it to `G:/My Drive/dungeon-crawler-2026/crab-breeze-<date>-<name>.mp4`
and writes the report page. The Drive connector check and link are done by the session.

## 4. Cloud stills

`--stills` steps the scripted pilot at the fixed rate with no window and writes a PNG at the start
of each beat, through wgpu on lavapipe (`VK_ICD_FILENAMES`), as Mining the Sky's `shot` mode does.
They show what a frame looks like, never how fast it runs.

## 5. Cost

The pilot is one system in `FixedUpdate` reading a few markers: well under 0.1 ms a step. Nothing
draws when it is not active.

## Open questions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| P1 | Drive folder for videos | `G:/My Drive/dungeon-crawler-2026/` with a `crab-breeze-` prefix, as the owner's other games do | provisional (survey P1) |
| P2 | Audio in the share copy | keep the game's audio once it has any | provisional (survey P2) |
